//! Pure-Rust ISO Base Media File Format (ISO-BMFF / MP4) Container Demuxer.
//!
//! Provides zero-dependency, safe, pure-Rust parsing of MP4 video files,
//! extracting video tracks, frame sample offsets, timestamps, and keyframe tables
//! without external C/FFmpeg dependencies.

use crate::error::DitherError;

/// Codec FourCC identifier for video tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    /// AVC / H.264 (`avc1`)
    Avc1,
    /// HEVC / H.265 (`hvc1` or `hev1`)
    Hevc,
    /// Motion JPEG (`mjpeg` or `jpeg`)
    MotionJpeg,
    /// MPEG-4 Visual (`mp4v`)
    Mp4v,
    /// Unknown fourcc code
    Other([u8; 4]),
}

impl Default for VideoCodec {
    fn default() -> Self {
        Self::Other([0; 4])
    }
}

impl VideoCodec {
    pub const fn from_fourcc(fourcc: [u8; 4]) -> Self {
        match &fourcc {
            b"avc1" => Self::Avc1,
            b"hvc1" | b"hev1" => Self::Hevc,
            b"mjpg" | b"jpeg" => Self::MotionJpeg,
            b"mp4v" => Self::Mp4v,
            _ => Self::Other(fourcc),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Avc1 => "H.264 (avc1)",
            Self::Hevc => "HEVC (hvc1)",
            Self::MotionJpeg => "Motion JPEG",
            Self::Mp4v => "MPEG-4 Visual",
            Self::Other(_) => "Unknown Codec",
        }
    }
}

/// Information and sample table for a parsed video track.
#[derive(Debug, Clone)]
pub struct Mp4Track {
    pub track_id: u32,
    pub width: u32,
    pub height: u32,
    pub timescale: u32,
    pub duration: u64,
    pub codec: VideoCodec,
    pub samples: Vec<Mp4Sample>,
}

impl Mp4Track {
    /// Nominal frame rate (FPS) calculated from duration and sample count.
    pub fn nominal_fps(&self) -> f32 {
        if self.duration == 0 || self.timescale == 0 || self.samples.is_empty() {
            return 30.0;
        }
        let duration_secs = self.duration as f64 / self.timescale as f64;
        if duration_secs > 0.0 { (self.samples.len() as f64 / duration_secs) as f32 } else { 30.0 }
    }
}

/// A single video sample (frame) descriptor inside the MP4 container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp4Sample {
    /// Byte offset in the file where this sample begins.
    pub offset: u64,
    /// Byte size of this sample.
    pub size: u32,
    /// Timestamp presentation time in track timescale units.
    pub timestamp: u64,
    /// Duration of this sample in timescale units.
    pub duration: u32,
    /// Whether this sample is a sync sample / keyframe (I-frame).
    pub is_keyframe: bool,
}

/// Parsed ISO-BMFF / MP4 container structure.
#[derive(Debug, Clone, Default)]
pub struct Mp4Demuxer {
    pub major_brand: [u8; 4],
    pub minor_version: u32,
    pub timescale: u32,
    pub duration: u64,
    pub tracks: Vec<Mp4Track>,
}

impl Mp4Demuxer {
    /// Parse an MP4 file from a byte slice.
    pub fn parse(bytes: &[u8]) -> Result<Self, DitherError> {
        if bytes.len() < 8 {
            return Err(DitherError::InvalidParameter("File too small to be an MP4 container".into()));
        }

        let mut demuxer = Self::default();
        let mut offset = 0;
        let mut found_moov = false;

        while offset + 8 <= bytes.len() {
            let (box_type, box_size, header_len) = parse_box_header(&bytes[offset..])?;
            if box_size < header_len as u64 {
                return Err(DitherError::InvalidParameter("Malformed box size".into()));
            }

            let box_end = offset.saturating_add(box_size as usize).min(bytes.len());
            let payload = &bytes[offset + header_len..box_end];

            match &box_type {
                b"ftyp" => {
                    if payload.len() >= 8 {
                        demuxer.major_brand.copy_from_slice(&payload[0..4]);
                        demuxer.minor_version = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                    }
                }
                b"moov" => {
                    found_moov = true;
                    demuxer.parse_moov(payload)?;
                }
                _ => {}
            }

            offset = box_end;
            if box_size == 0 {
                break;
            }
        }

        if !found_moov {
            return Err(DitherError::InvalidParameter("No 'moov' metadata atom found in MP4".into()));
        }

        Ok(demuxer)
    }

    /// Retrieve the primary video track, if present.
    pub fn primary_video_track(&self) -> Option<&Mp4Track> {
        self.tracks.first()
    }

    /// Extract raw sample bytes from the full container buffer.
    pub fn get_sample_bytes<'a>(&self, sample: &Mp4Sample, container_bytes: &'a [u8]) -> Option<&'a [u8]> {
        let start = sample.offset as usize;
        let end = start.checked_add(sample.size as usize)?;
        if end <= container_bytes.len() { Some(&container_bytes[start..end]) } else { None }
    }

    fn parse_moov(&mut self, payload: &[u8]) -> Result<(), DitherError> {
        let mut offset = 0;
        while offset + 8 <= payload.len() {
            let (box_type, box_size, header_len) = parse_box_header(&payload[offset..])?;
            let box_end = offset.saturating_add(box_size as usize).min(payload.len());
            let box_payload = &payload[offset + header_len..box_end];

            match &box_type {
                b"mvhd" => {
                    if box_payload.len() >= 20 {
                        let version = box_payload[0];
                        if version == 0 && box_payload.len() >= 20 {
                            self.timescale = u32::from_be_bytes([box_payload[12], box_payload[13], box_payload[14], box_payload[15]]);
                            self.duration = u32::from_be_bytes([box_payload[16], box_payload[17], box_payload[18], box_payload[19]]) as u64;
                        } else if version == 1 && box_payload.len() >= 32 {
                            self.timescale = u32::from_be_bytes([box_payload[20], box_payload[21], box_payload[22], box_payload[23]]);
                            self.duration = u64::from_be_bytes([
                                box_payload[24],
                                box_payload[25],
                                box_payload[26],
                                box_payload[27],
                                box_payload[28],
                                box_payload[29],
                                box_payload[30],
                                box_payload[31],
                            ]);
                        }
                    }
                }
                b"trak" => {
                    if let Some(track) = parse_trak(box_payload)? {
                        self.tracks.push(track);
                    }
                }
                _ => {}
            }

            offset = box_end;
            if box_size == 0 {
                break;
            }
        }
        Ok(())
    }
}

fn parse_box_header(bytes: &[u8]) -> Result<([u8; 4], u64, usize), DitherError> {
    if bytes.len() < 8 {
        return Err(DitherError::InvalidParameter("Incomplete box header".into()));
    }
    let size32 = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64;
    let mut box_type = [0u8; 4];
    box_type.copy_from_slice(&bytes[4..8]);

    if size32 == 1 {
        if bytes.len() < 16 {
            return Err(DitherError::InvalidParameter("Incomplete 64-bit box header".into()));
        }
        let size64 = u64::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]);
        Ok((box_type, size64, 16))
    } else {
        Ok((box_type, size32, 8))
    }
}

fn parse_trak(payload: &[u8]) -> Result<Option<Mp4Track>, DitherError> {
    let mut offset = 0;
    let mut is_video = false;
    let mut track_id = 0u32;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut timescale = 1000u32;
    let mut duration = 0u64;
    let mut codec = VideoCodec::Other([0; 4]);
    let mut samples = Vec::new();

    while offset + 8 <= payload.len() {
        let (box_type, box_size, header_len) = parse_box_header(&payload[offset..])?;
        let box_end = offset.saturating_add(box_size as usize).min(payload.len());
        let box_payload = &payload[offset + header_len..box_end];

        match &box_type {
            b"tkhd" => {
                if box_payload.len() >= 84 {
                    let version = box_payload[0];
                    if version == 0 {
                        track_id = u32::from_be_bytes([box_payload[12], box_payload[13], box_payload[14], box_payload[15]]);
                    } else if version == 1 && box_payload.len() >= 96 {
                        track_id = u32::from_be_bytes([box_payload[20], box_payload[21], box_payload[22], box_payload[23]]);
                    }
                    // Width and height are 16.16 fixed point numbers at end of tkhd
                    let w_offset = box_payload.len().saturating_sub(8);
                    let h_offset = box_payload.len().saturating_sub(4);
                    let w_raw =
                        u32::from_be_bytes([box_payload[w_offset], box_payload[w_offset + 1], box_payload[w_offset + 2], box_payload[w_offset + 3]]);
                    let h_raw =
                        u32::from_be_bytes([box_payload[h_offset], box_payload[h_offset + 1], box_payload[h_offset + 2], box_payload[h_offset + 3]]);
                    width = w_raw >> 16;
                    height = h_raw >> 16;
                }
            }
            b"mdia" => {
                let mdia_info = parse_mdia(box_payload)?;
                if mdia_info.is_video {
                    is_video = true;
                    timescale = mdia_info.timescale;
                    duration = mdia_info.duration;
                    codec = mdia_info.codec;
                    samples = mdia_info.samples;
                    if mdia_info.width > 0 {
                        width = mdia_info.width;
                    }
                    if mdia_info.height > 0 {
                        height = mdia_info.height;
                    }
                }
            }
            _ => {}
        }

        offset = box_end;
        if box_size == 0 {
            break;
        }
    }

    if is_video { Ok(Some(Mp4Track { track_id, width, height, timescale, duration, codec, samples })) } else { Ok(None) }
}

#[derive(Default)]
struct MdiaInfo {
    is_video: bool,
    timescale: u32,
    duration: u64,
    width: u32,
    height: u32,
    codec: VideoCodec,
    samples: Vec<Mp4Sample>,
}

fn parse_mdia(payload: &[u8]) -> Result<MdiaInfo, DitherError> {
    let mut info = MdiaInfo { codec: VideoCodec::Other([0; 4]), timescale: 1000, ..Default::default() };
    let mut offset = 0;

    while offset + 8 <= payload.len() {
        let (box_type, box_size, header_len) = parse_box_header(&payload[offset..])?;
        let box_end = offset.saturating_add(box_size as usize).min(payload.len());
        let box_payload = &payload[offset + header_len..box_end];

        match &box_type {
            b"mdhd" => {
                if box_payload.len() >= 20 {
                    let version = box_payload[0];
                    if version == 0 {
                        info.timescale = u32::from_be_bytes([box_payload[12], box_payload[13], box_payload[14], box_payload[15]]);
                        info.duration = u32::from_be_bytes([box_payload[16], box_payload[17], box_payload[18], box_payload[19]]) as u64;
                    } else if version == 1 && box_payload.len() >= 32 {
                        info.timescale = u32::from_be_bytes([box_payload[20], box_payload[21], box_payload[22], box_payload[23]]);
                        info.duration = u64::from_be_bytes([
                            box_payload[24],
                            box_payload[25],
                            box_payload[26],
                            box_payload[27],
                            box_payload[28],
                            box_payload[29],
                            box_payload[30],
                            box_payload[31],
                        ]);
                    }
                }
            }
            b"hdlr" => {
                if box_payload.len() >= 12 {
                    let handler_type = &box_payload[8..12];
                    if handler_type == b"vide" {
                        info.is_video = true;
                    }
                }
            }
            b"minf" => {
                let minf_info = parse_minf(box_payload)?;
                info.codec = minf_info.codec;
                info.samples = minf_info.samples;
                if minf_info.width > 0 {
                    info.width = minf_info.width;
                }
                if minf_info.height > 0 {
                    info.height = minf_info.height;
                }
            }
            _ => {}
        }

        offset = box_end;
        if box_size == 0 {
            break;
        }
    }

    Ok(info)
}

#[derive(Default)]
struct MinfInfo {
    width: u32,
    height: u32,
    codec: VideoCodec,
    samples: Vec<Mp4Sample>,
}

fn parse_minf(payload: &[u8]) -> Result<MinfInfo, DitherError> {
    let mut info = MinfInfo { codec: VideoCodec::Other([0; 4]), ..Default::default() };
    let mut offset = 0;

    while offset + 8 <= payload.len() {
        let (box_type, box_size, header_len) = parse_box_header(&payload[offset..])?;
        let box_end = offset.saturating_add(box_size as usize).min(payload.len());
        let box_payload = &payload[offset + header_len..box_end];

        if &box_type == b"stbl" {
            let stbl_info = parse_stbl(box_payload)?;
            info.codec = stbl_info.codec;
            info.width = stbl_info.width;
            info.height = stbl_info.height;
            info.samples = stbl_info.samples;
        }

        offset = box_end;
        if box_size == 0 {
            break;
        }
    }

    Ok(info)
}

#[derive(Default)]
struct StblInfo {
    width: u32,
    height: u32,
    codec: VideoCodec,
    samples: Vec<Mp4Sample>,
}

fn parse_stbl(payload: &[u8]) -> Result<StblInfo, DitherError> {
    let mut info = StblInfo { codec: VideoCodec::Other([0; 4]), ..Default::default() };
    let mut offset = 0;

    let mut stts_entries: Vec<(u32, u32)> = Vec::new();
    let mut stsc_entries: Vec<(u32, u32, u32)> = Vec::new();
    let mut stsz_entries: Vec<u32> = Vec::new();
    let mut stco_offsets: Vec<u64> = Vec::new();
    let mut sync_samples: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();

    while offset + 8 <= payload.len() {
        let (box_type, box_size, header_len) = parse_box_header(&payload[offset..])?;
        let box_end = offset.saturating_add(box_size as usize).min(payload.len());
        let box_payload = &payload[offset + header_len..box_end];

        match &box_type {
            b"stsd" => {
                if box_payload.len() >= 8 {
                    let entry_count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]);
                    if entry_count > 0 && box_payload.len() >= 16 {
                        let mut fourcc = [0u8; 4];
                        fourcc.copy_from_slice(&box_payload[12..16]);
                        info.codec = VideoCodec::from_fourcc(fourcc);

                        // VisualSampleEntry width and height
                        if box_payload.len() >= 40 {
                            info.width = u16::from_be_bytes([box_payload[32], box_payload[33]]) as u32;
                            info.height = u16::from_be_bytes([box_payload[34], box_payload[35]]) as u32;
                        }
                    }
                }
            }
            b"stts" => {
                if box_payload.len() >= 8 {
                    let count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]) as usize;
                    let mut entry_off = 8;
                    for _ in 0..count {
                        if entry_off + 8 <= box_payload.len() {
                            let sample_count = u32::from_be_bytes([
                                box_payload[entry_off],
                                box_payload[entry_off + 1],
                                box_payload[entry_off + 2],
                                box_payload[entry_off + 3],
                            ]);
                            let sample_delta = u32::from_be_bytes([
                                box_payload[entry_off + 4],
                                box_payload[entry_off + 5],
                                box_payload[entry_off + 6],
                                box_payload[entry_off + 7],
                            ]);
                            stts_entries.push((sample_count, sample_delta));
                            entry_off += 8;
                        }
                    }
                }
            }
            b"stsz" => {
                if box_payload.len() >= 12 {
                    let sample_size = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]);
                    let count = u32::from_be_bytes([box_payload[8], box_payload[9], box_payload[10], box_payload[11]]) as usize;

                    if sample_size > 0 {
                        stsz_entries = vec![sample_size; count];
                    } else {
                        let mut entry_off = 12;
                        for _ in 0..count {
                            if entry_off + 4 <= box_payload.len() {
                                let sz = u32::from_be_bytes([
                                    box_payload[entry_off],
                                    box_payload[entry_off + 1],
                                    box_payload[entry_off + 2],
                                    box_payload[entry_off + 3],
                                ]);
                                stsz_entries.push(sz);
                                entry_off += 4;
                            }
                        }
                    }
                }
            }
            b"stsc" => {
                if box_payload.len() >= 8 {
                    let count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]) as usize;
                    let mut entry_off = 8;
                    for _ in 0..count {
                        if entry_off + 12 <= box_payload.len() {
                            let first_chunk = u32::from_be_bytes([
                                box_payload[entry_off],
                                box_payload[entry_off + 1],
                                box_payload[entry_off + 2],
                                box_payload[entry_off + 3],
                            ]);
                            let samples_per_chunk = u32::from_be_bytes([
                                box_payload[entry_off + 4],
                                box_payload[entry_off + 5],
                                box_payload[entry_off + 6],
                                box_payload[entry_off + 7],
                            ]);
                            let desc_idx = u32::from_be_bytes([
                                box_payload[entry_off + 8],
                                box_payload[entry_off + 9],
                                box_payload[entry_off + 10],
                                box_payload[entry_off + 11],
                            ]);
                            stsc_entries.push((first_chunk, samples_per_chunk, desc_idx));
                            entry_off += 12;
                        }
                    }
                }
            }
            b"stco" => {
                if box_payload.len() >= 8 {
                    let count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]) as usize;
                    let mut entry_off = 8;
                    for _ in 0..count {
                        if entry_off + 4 <= box_payload.len() {
                            let off = u32::from_be_bytes([
                                box_payload[entry_off],
                                box_payload[entry_off + 1],
                                box_payload[entry_off + 2],
                                box_payload[entry_off + 3],
                            ]) as u64;
                            stco_offsets.push(off);
                            entry_off += 4;
                        }
                    }
                }
            }
            b"co64" => {
                if box_payload.len() >= 8 {
                    let count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]) as usize;
                    let mut entry_off = 8;
                    for _ in 0..count {
                        if entry_off + 8 <= box_payload.len() {
                            let off = u64::from_be_bytes([
                                box_payload[entry_off],
                                box_payload[entry_off + 1],
                                box_payload[entry_off + 2],
                                box_payload[entry_off + 3],
                                box_payload[entry_off + 4],
                                box_payload[entry_off + 5],
                                box_payload[entry_off + 6],
                                box_payload[entry_off + 7],
                            ]);
                            stco_offsets.push(off);
                            entry_off += 8;
                        }
                    }
                }
            }
            b"stss" if box_payload.len() >= 8 => {
                let count = u32::from_be_bytes([box_payload[4], box_payload[5], box_payload[6], box_payload[7]]) as usize;
                let mut entry_off = 8;
                for _ in 0..count {
                    if entry_off + 4 <= box_payload.len() {
                        let sample_nr = u32::from_be_bytes([
                            box_payload[entry_off],
                            box_payload[entry_off + 1],
                            box_payload[entry_off + 2],
                            box_payload[entry_off + 3],
                        ]);
                        sync_samples.insert(sample_nr);
                        entry_off += 4;
                    }
                }
            }
            _ => {}
        }

        offset = box_end;
        if box_size == 0 {
            break;
        }
    }

    // Assemble samples using stsc, stco, stsz, and stts
    if !stsz_entries.is_empty() && !stco_offsets.is_empty() {
        let mut sample_durations = Vec::with_capacity(stsz_entries.len());
        for (count, delta) in stts_entries {
            for _ in 0..count {
                sample_durations.push(delta);
            }
        }

        let mut sample_idx = 0;
        let mut current_time = 0u64;

        for (chunk_idx, &chunk_offset) in stco_offsets.iter().enumerate() {
            let chunk_num = (chunk_idx as u32) + 1; // 1-indexed in stsc
            // Find samples per chunk from stsc
            let mut samples_in_chunk = 1u32;
            for entry in stsc_entries.iter().rev() {
                if chunk_num >= entry.0 {
                    samples_in_chunk = entry.1;
                    break;
                }
            }

            let mut byte_in_chunk = 0u64;
            for _ in 0..samples_in_chunk {
                if sample_idx >= stsz_entries.len() {
                    break;
                }

                let size = stsz_entries[sample_idx];
                let offset = chunk_offset + byte_in_chunk;
                let duration = sample_durations.get(sample_idx).copied().unwrap_or(1000);
                let is_keyframe = sync_samples.is_empty() || sync_samples.contains(&(sample_idx as u32 + 1));

                info.samples.push(Mp4Sample { offset, size, timestamp: current_time, duration, is_keyframe });

                current_time += duration as u64;
                byte_in_chunk += size as u64;
                sample_idx += 1;
            }
        }
    }

    Ok(info)
}

// ============================================================================
// WebM / Matroska EBML Container Demuxer
// ============================================================================

/// Information and sample table for a parsed WebM / Matroska track.
#[derive(Debug, Clone)]
pub struct WebmTrack {
    pub track_number: u64,
    pub track_type: u64, // 1 = Video, 2 = Audio
    pub codec_id: String,
    pub width: u32,
    pub height: u32,
    pub samples: Vec<WebmSample>,
}

impl WebmTrack {
    pub fn nominal_fps(&self) -> f32 {
        30.0
    }
}

/// A single video or audio sample (block) inside a WebM Cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebmSample {
    pub track_number: u64,
    pub timecode: i16,
    pub is_keyframe: bool,
    pub offset: u64,
    pub size: u32,
}

/// Parsed WebM container metadata and sample directory.
#[derive(Debug, Clone)]
pub struct WebmDemuxer {
    pub tracks: Vec<WebmTrack>,
    pub timecode_scale: u64,
}

impl WebmDemuxer {
    /// Parse a WebM or Matroska container byte buffer with zero external dependencies.
    pub fn parse(bytes: &[u8]) -> Result<Self, DitherError> {
        if bytes.len() < 12 {
            return Err(DitherError::InvalidParameter("WebM buffer too short".into()));
        }

        let mut offset = 0;
        let ebml_id = read_ebml_id(bytes, &mut offset).ok_or_else(|| DitherError::InvalidParameter("Failed to read EBML ID".into()))?;

        if ebml_id != 0x1A45DFA3 {
            return Err(DitherError::InvalidParameter("Not a valid EBML/WebM container".into()));
        }

        let ebml_size = read_ebml_size(bytes, &mut offset).ok_or_else(|| DitherError::InvalidParameter("Failed to read EBML header size".into()))?;
        offset = (offset + ebml_size as usize).min(bytes.len());

        let mut tracks = Vec::new();
        let mut timecode_scale = 1_000_000u64; // default 1ms

        while offset + 4 < bytes.len() {
            let elem_id = match read_ebml_id(bytes, &mut offset) {
                Some(id) => id,
                None => break,
            };
            let elem_size = match read_ebml_size(bytes, &mut offset) {
                Some(sz) => sz,
                None => break,
            };

            let elem_end =
                if elem_size == u64::MAX || (offset + elem_size as usize) > bytes.len() { bytes.len() } else { offset + elem_size as usize };

            match elem_id {
                0x18538067 => {
                    // Segment: recurse inside its child elements
                    let mut seg_offset = offset;
                    while seg_offset + 4 < elem_end {
                        let child_id = match read_ebml_id(bytes, &mut seg_offset) {
                            Some(id) => id,
                            None => break,
                        };
                        let child_size = match read_ebml_size(bytes, &mut seg_offset) {
                            Some(sz) => sz,
                            None => break,
                        };
                        let child_end = (seg_offset + child_size as usize).min(elem_end);

                        match child_id {
                            0x1549A966 => {
                                // Info: parse TimecodeScale
                                let mut info_offset = seg_offset;
                                while info_offset + 2 < child_end {
                                    if let Some(prop_id) = read_ebml_id(bytes, &mut info_offset)
                                        && let Some(prop_size) = read_ebml_size(bytes, &mut info_offset)
                                    {
                                        if prop_id == 0x2AD7B1 {
                                            timecode_scale = read_ebml_uint(bytes, info_offset, prop_size as usize);
                                        }
                                        info_offset = (info_offset + prop_size as usize).min(child_end);
                                    } else {
                                        break;
                                    }
                                }
                            }
                            0x1654AE6B => {
                                // Tracks: parse TrackEntry elements
                                let mut trk_offset = seg_offset;
                                while trk_offset + 2 < child_end {
                                    if let Some(entry_id) = read_ebml_id(bytes, &mut trk_offset)
                                        && let Some(entry_size) = read_ebml_size(bytes, &mut trk_offset)
                                    {
                                        let entry_end = (trk_offset + entry_size as usize).min(child_end);
                                        if entry_id == 0xAE
                                            && let Some(track) = parse_webm_track_entry(bytes, trk_offset, entry_end)
                                        {
                                            tracks.push(track);
                                        }
                                        trk_offset = entry_end;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            0x1F43B675 => {
                                // Cluster: parse SimpleBlock and Block elements
                                parse_webm_cluster(bytes, seg_offset, child_end, &mut tracks);
                            }
                            _ => {}
                        }

                        seg_offset = child_end;
                    }
                    offset = elem_end;
                }
                _ => {
                    offset = elem_end;
                }
            }
        }

        if tracks.is_empty() {
            return Err(DitherError::InvalidParameter("No tracks found in WebM container".into()));
        }

        Ok(Self { tracks, timecode_scale })
    }

    /// Retrieve the primary video track, if present.
    pub fn primary_video_track(&self) -> Option<&WebmTrack> {
        self.tracks.iter().find(|t| t.track_type == 1)
    }

    /// Read raw sample byte slice safely.
    pub fn get_sample_bytes<'a>(&self, sample: &WebmSample, file_bytes: &'a [u8]) -> Option<&'a [u8]> {
        let start = sample.offset as usize;
        let end = start.checked_add(sample.size as usize)?;
        if end <= file_bytes.len() { Some(&file_bytes[start..end]) } else { None }
    }
}

fn parse_webm_track_entry(bytes: &[u8], mut offset: usize, end: usize) -> Option<WebmTrack> {
    let mut track_number = 1u64;
    let mut track_type = 1u64;
    let mut codec_id = String::new();
    let mut width = 0u32;
    let mut height = 0u32;

    while offset + 2 < end {
        let prop_id = read_ebml_id(bytes, &mut offset)?;
        let prop_size = read_ebml_size(bytes, &mut offset)?;
        let prop_end = (offset + prop_size as usize).min(end);

        match prop_id {
            0xD7 => track_number = read_ebml_uint(bytes, offset, prop_size as usize),
            0x83 => track_type = read_ebml_uint(bytes, offset, prop_size as usize),
            0x86 => {
                if let Ok(s) = std::str::from_utf8(&bytes[offset..prop_end]) {
                    codec_id = s.trim_matches('\0').to_string();
                }
            }
            0xE0 => {
                // Video settings container
                let mut v_offset = offset;
                while v_offset + 2 < prop_end {
                    if let Some(vid_id) = read_ebml_id(bytes, &mut v_offset)
                        && let Some(vid_sz) = read_ebml_size(bytes, &mut v_offset)
                    {
                        if vid_id == 0xB0 {
                            width = read_ebml_uint(bytes, v_offset, vid_sz as usize) as u32;
                        } else if vid_id == 0xBA {
                            height = read_ebml_uint(bytes, v_offset, vid_sz as usize) as u32;
                        }
                        v_offset = (v_offset + vid_sz as usize).min(prop_end);
                    } else {
                        break;
                    }
                }
            }
            _ => {}
        }

        offset = prop_end;
    }

    Some(WebmTrack { track_number, track_type, codec_id, width, height, samples: Vec::new() })
}

fn parse_webm_cluster(bytes: &[u8], mut offset: usize, end: usize, tracks: &mut [WebmTrack]) {
    while offset + 2 < end {
        let elem_id = match read_ebml_id(bytes, &mut offset) {
            Some(id) => id,
            None => break,
        };
        let elem_size = match read_ebml_size(bytes, &mut offset) {
            Some(sz) => sz,
            None => break,
        };
        let elem_end = (offset + elem_size as usize).min(end);

        if elem_id == 0xA3 || elem_id == 0xA1 {
            // SimpleBlock (0xA3) or Block (0xA1 inside BlockGroup)
            let mut b_offset = offset;
            if let Some(trk_num) = read_vint(bytes, &mut b_offset)
                && b_offset + 3 <= elem_end
            {
                let timecode = i16::from_be_bytes([bytes[b_offset], bytes[b_offset + 1]]);
                let flags = bytes[b_offset + 2];
                let is_keyframe = (flags & 0x80) != 0;
                b_offset += 3;

                let payload_offset = b_offset as u64;
                let payload_size = (elem_end - b_offset) as u32;

                if let Some(track) = tracks.iter_mut().find(|t| t.track_number == trk_num) {
                    track.samples.push(WebmSample { track_number: trk_num, timecode, is_keyframe, offset: payload_offset, size: payload_size });
                }
            }
        }

        offset = elem_end;
    }
}

fn read_vint(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    if *offset >= bytes.len() {
        return None;
    }
    let first = bytes[*offset];
    let (len, mask) = if first & 0x80 != 0 {
        (1, 0x7F)
    } else if first & 0x40 != 0 {
        (2, 0x3F)
    } else if first & 0x20 != 0 {
        (3, 0x1F)
    } else if first & 0x10 != 0 {
        (4, 0x0F)
    } else if first & 0x08 != 0 {
        (5, 0x07)
    } else if first & 0x04 != 0 {
        (6, 0x03)
    } else if first & 0x02 != 0 {
        (7, 0x01)
    } else if first & 0x01 != 0 {
        (8, 0x00)
    } else {
        return None;
    };

    if *offset + len > bytes.len() {
        return None;
    }
    let mut val = (first & mask) as u64;
    for i in 1..len {
        val = (val << 8) | (bytes[*offset + i] as u64);
    }
    *offset += len;
    Some(val)
}

fn read_ebml_id(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    if *offset >= bytes.len() {
        return None;
    }
    let first = bytes[*offset];
    let len = if first & 0x80 != 0 {
        1
    } else if first & 0x40 != 0 {
        2
    } else if first & 0x20 != 0 {
        3
    } else if first & 0x10 != 0 {
        4
    } else {
        return None;
    };

    if *offset + len > bytes.len() {
        return None;
    }
    let mut id = 0u32;
    for i in 0..len {
        id = (id << 8) | (bytes[*offset + i] as u32);
    }
    *offset += len;
    Some(id)
}

fn read_ebml_size(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    read_vint(bytes, offset)
}

fn read_ebml_uint(bytes: &[u8], offset: usize, len: usize) -> u64 {
    let end = (offset + len).min(bytes.len());
    let mut val = 0u64;
    for &b in &bytes[offset..end] {
        val = (val << 8) | (b as u64);
    }
    val
}

/// A single animation frame to be packed into an Animated WebP container.
#[derive(Debug, Clone)]
pub struct WebpAnimFrame {
    /// Frame presentation duration in milliseconds.
    pub duration_ms: u32,
    /// Horizontal offset on canvas (must be even).
    pub x: u32,
    /// Vertical offset on canvas (must be even).
    pub y: u32,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// WebP image subchunk (VP8L or VP8 chunk including FourCC tag and length).
    pub payload: Vec<u8>,
}

/// Pure-Rust Animated WebP container muxer and builder.
///
/// Packages individual VP8/VP8L bitstream chunks into a valid RIFF container
/// with VP8X extended headers, ANIM control chunks, and ANMF frame subchunks.
#[derive(Debug, Clone, Default)]
pub struct WebpAnimationMuxer {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub loop_count: u16,
    pub background_color: [u8; 4],
    pub frames: Vec<WebpAnimFrame>,
}

impl WebpAnimationMuxer {
    /// Create a new animated WebP muxer for the specified canvas dimensions.
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            canvas_width,
            canvas_height,
            loop_count: 0, // 0 = infinite loop
            background_color: [0, 0, 0, 0],
            frames: Vec::new(),
        }
    }

    /// Set loop count (0 = infinite loop).
    pub fn with_loop_count(mut self, loop_count: u16) -> Self {
        self.loop_count = loop_count;
        self
    }

    /// Add a frame to the animation sequence.
    pub fn add_frame(&mut self, frame: WebpAnimFrame) {
        self.frames.push(frame);
    }

    /// Helper to extract the primary image subchunk (`VP8L` or `VP8 `) from a
    /// standalone single-frame WebP byte buffer.
    pub fn extract_image_chunk(single_frame_webp: &[u8]) -> Option<&[u8]> {
        if single_frame_webp.len() < 20 {
            return None;
        }
        if &single_frame_webp[0..4] != b"RIFF" || &single_frame_webp[8..12] != b"WEBP" {
            return None;
        }

        let mut offset = 12;
        while offset + 8 <= single_frame_webp.len() {
            let tag = &single_frame_webp[offset..offset + 4];
            let chunk_len = u32::from_le_bytes([
                single_frame_webp[offset + 4],
                single_frame_webp[offset + 5],
                single_frame_webp[offset + 6],
                single_frame_webp[offset + 7],
            ]) as usize;

            let total_chunk_len = 8 + chunk_len + (chunk_len % 2);
            if offset + total_chunk_len > single_frame_webp.len() {
                return None;
            }

            if tag == b"VP8L" || tag == b"VP8 " {
                return Some(&single_frame_webp[offset..offset + total_chunk_len]);
            }

            offset += total_chunk_len;
        }

        None
    }

    /// Assemble and build the final Animated WebP byte stream.
    pub fn build(&self) -> Result<Vec<u8>, DitherError> {
        if self.canvas_width == 0 || self.canvas_height == 0 {
            return Err(DitherError::ZeroDimension);
        }
        if self.frames.is_empty() {
            return Err(DitherError::InvalidParameter("Animation muxer requires at least one frame".into()));
        }

        let mut buf = Vec::new();

        // 1. RIFF Header placeholder (12 bytes: RIFF + 4-byte size + WEBP)
        buf.extend_from_slice(b"RIFF\0\0\0\0WEBP");

        // 2. VP8X Chunk (Extended WebP header)
        buf.extend_from_slice(b"VP8X");
        buf.extend_from_slice(&10u32.to_le_bytes()); // Chunk size: 10 bytes

        // Flags: Bit 1 is Animation (0x02)
        buf.push(0x02);
        // Reserved (3 bytes)
        buf.extend_from_slice(&[0, 0, 0]);
        // Canvas width minus 1 (3 bytes LE)
        let w_minus_1 = self.canvas_width.saturating_sub(1);
        buf.push(w_minus_1 as u8);
        buf.push((w_minus_1 >> 8) as u8);
        buf.push((w_minus_1 >> 16) as u8);
        // Canvas height minus 1 (3 bytes LE)
        let h_minus_1 = self.canvas_height.saturating_sub(1);
        buf.push(h_minus_1 as u8);
        buf.push((h_minus_1 >> 8) as u8);
        buf.push((h_minus_1 >> 16) as u8);

        // 3. ANIM Chunk (Animation control)
        buf.extend_from_slice(b"ANIM");
        buf.extend_from_slice(&6u32.to_le_bytes()); // Chunk size: 6 bytes
        buf.extend_from_slice(&self.background_color); // 4 bytes RGBA background
        buf.extend_from_slice(&self.loop_count.to_le_bytes()); // 2 bytes loop count

        // 4. ANMF Chunks (One per frame)
        for frame in &self.frames {
            let anmf_payload_size = 16 + frame.payload.len();
            buf.extend_from_slice(b"ANMF");
            buf.extend_from_slice(&(anmf_payload_size as u32).to_le_bytes());

            // 16-byte ANMF Header
            // Frame X / 2 (3 bytes LE)
            let fx = frame.x / 2;
            buf.push(fx as u8);
            buf.push((fx >> 8) as u8);
            buf.push((fx >> 16) as u8);

            // Frame Y / 2 (3 bytes LE)
            let fy = frame.y / 2;
            buf.push(fy as u8);
            buf.push((fy >> 8) as u8);
            buf.push((fy >> 16) as u8);

            // Frame Width minus 1 (3 bytes LE)
            let fw = frame.width.saturating_sub(1);
            buf.push(fw as u8);
            buf.push((fw >> 8) as u8);
            buf.push((fw >> 16) as u8);

            // Frame Height minus 1 (3 bytes LE)
            let fh = frame.height.saturating_sub(1);
            buf.push(fh as u8);
            buf.push((fh >> 8) as u8);
            buf.push((fh >> 16) as u8);

            // Duration in ms (3 bytes LE)
            let dur = frame.duration_ms;
            buf.push(dur as u8);
            buf.push((dur >> 8) as u8);
            buf.push((dur >> 16) as u8);

            // Flags: 0x02 = do not blend, overwrite canvas
            buf.push(0x02);

            // Frame subchunk payload (VP8L / VP8)
            buf.extend_from_slice(&frame.payload);

            // Padding if ANMF payload size is odd
            if anmf_payload_size % 2 == 1 {
                buf.push(0);
            }
        }

        // 5. Finalize RIFF file size (total length - 8 bytes)
        let riff_size = (buf.len().saturating_sub(8)) as u32;
        buf[4..8].copy_from_slice(&riff_size.to_le_bytes());

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mp4_box_parsing_and_demuxing() {
        // Build a minimal synthetic MP4 buffer with ftyp and moov
        let mut buf = Vec::new();

        // 1. ftyp box
        let ftyp_size = 24u32;
        buf.extend_from_slice(&ftyp_size.to_be_bytes());
        buf.extend_from_slice(b"ftyp");
        buf.extend_from_slice(b"isom"); // major brand
        buf.extend_from_slice(&512u32.to_be_bytes()); // minor version
        buf.extend_from_slice(b"isom");
        buf.extend_from_slice(b"mp42");

        // 2. moov box
        let mut moov_payload = Vec::new();

        // mvhd box (32 bytes header + data)
        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&[0, 0, 0, 0]); // version & flags
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // creation time
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // mod time
        mvhd.extend_from_slice(&1000u32.to_be_bytes()); // timescale
        mvhd.extend_from_slice(&5000u32.to_be_bytes()); // duration
        let mvhd_box_size = (mvhd.len() + 8) as u32;
        moov_payload.extend_from_slice(&mvhd_box_size.to_be_bytes());
        moov_payload.extend_from_slice(b"mvhd");
        moov_payload.extend_from_slice(&mvhd);

        // trak box
        let mut trak_payload = Vec::new();

        // tkhd (84 bytes version 0)
        let mut tkhd = vec![0u8; 84];
        tkhd[15] = 1; // track ID = 1
        // width = 1920 (1920 << 16)
        let w_bytes = (1920u32 << 16).to_be_bytes();
        tkhd[76..80].copy_from_slice(&w_bytes);
        // height = 1080 (1080 << 16)
        let h_bytes = (1080u32 << 16).to_be_bytes();
        tkhd[80..84].copy_from_slice(&h_bytes);
        let tkhd_box_size = (tkhd.len() + 8) as u32;
        trak_payload.extend_from_slice(&tkhd_box_size.to_be_bytes());
        trak_payload.extend_from_slice(b"tkhd");
        trak_payload.extend_from_slice(&tkhd);

        // mdia box
        let mut mdia_payload = Vec::new();
        // mdhd
        let mut mdhd = Vec::new();
        mdhd.extend_from_slice(&[0, 0, 0, 0]);
        mdhd.extend_from_slice(&0u32.to_be_bytes());
        mdhd.extend_from_slice(&0u32.to_be_bytes());
        mdhd.extend_from_slice(&1000u32.to_be_bytes()); // timescale 1000
        mdhd.extend_from_slice(&5000u32.to_be_bytes()); // duration 5000
        let mdhd_box_size = (mdhd.len() + 8) as u32;
        mdia_payload.extend_from_slice(&mdhd_box_size.to_be_bytes());
        mdia_payload.extend_from_slice(b"mdhd");
        mdia_payload.extend_from_slice(&mdhd);

        // hdlr (vide)
        let mut hdlr = Vec::new();
        hdlr.extend_from_slice(&[0, 0, 0, 0]);
        hdlr.extend_from_slice(&0u32.to_be_bytes());
        hdlr.extend_from_slice(b"vide"); // handler type
        let hdlr_box_size = (hdlr.len() + 8) as u32;
        mdia_payload.extend_from_slice(&hdlr_box_size.to_be_bytes());
        mdia_payload.extend_from_slice(b"hdlr");
        mdia_payload.extend_from_slice(&hdlr);

        // minf -> stbl
        let mut minf_payload = Vec::new();
        let mut stbl_payload = Vec::new();

        // stsd (avc1)
        let mut stsd = Vec::new();
        stsd.extend_from_slice(&[0, 0, 0, 0]);
        stsd.extend_from_slice(&1u32.to_be_bytes()); // 1 entry
        stsd.extend_from_slice(&0u32.to_be_bytes()); // reserved
        stsd.extend_from_slice(b"avc1");
        let stsd_box_size = (stsd.len() + 8) as u32;
        stbl_payload.extend_from_slice(&stsd_box_size.to_be_bytes());
        stbl_payload.extend_from_slice(b"stsd");
        stbl_payload.extend_from_slice(&stsd);

        // stsz (sample sizes: 2 samples of 4096 bytes)
        let mut stsz = Vec::new();
        stsz.extend_from_slice(&[0, 0, 0, 0]);
        stsz.extend_from_slice(&0u32.to_be_bytes()); // variable size
        stsz.extend_from_slice(&2u32.to_be_bytes()); // 2 samples
        stsz.extend_from_slice(&4096u32.to_be_bytes());
        stsz.extend_from_slice(&2048u32.to_be_bytes());
        let stsz_box_size = (stsz.len() + 8) as u32;
        stbl_payload.extend_from_slice(&stsz_box_size.to_be_bytes());
        stbl_payload.extend_from_slice(b"stsz");
        stbl_payload.extend_from_slice(&stsz);

        // stco (1 chunk at offset 1000)
        let mut stco = Vec::new();
        stco.extend_from_slice(&[0, 0, 0, 0]);
        stco.extend_from_slice(&1u32.to_be_bytes()); // 1 chunk
        stco.extend_from_slice(&1000u32.to_be_bytes());
        let stco_box_size = (stco.len() + 8) as u32;
        stbl_payload.extend_from_slice(&stco_box_size.to_be_bytes());
        stbl_payload.extend_from_slice(b"stco");
        stbl_payload.extend_from_slice(&stco);

        // stsc (chunk 1 has 2 samples)
        let mut stsc = Vec::new();
        stsc.extend_from_slice(&[0, 0, 0, 0]);
        stsc.extend_from_slice(&1u32.to_be_bytes()); // 1 entry
        stsc.extend_from_slice(&1u32.to_be_bytes()); // chunk 1
        stsc.extend_from_slice(&2u32.to_be_bytes()); // 2 samples per chunk
        stsc.extend_from_slice(&1u32.to_be_bytes()); // desc index 1
        let stsc_box_size = (stsc.len() + 8) as u32;
        stbl_payload.extend_from_slice(&stsc_box_size.to_be_bytes());
        stbl_payload.extend_from_slice(b"stsc");
        stbl_payload.extend_from_slice(&stsc);

        // stbl wrap
        let stbl_box_size = (stbl_payload.len() + 8) as u32;
        minf_payload.extend_from_slice(&stbl_box_size.to_be_bytes());
        minf_payload.extend_from_slice(b"stbl");
        minf_payload.extend_from_slice(&stbl_payload);

        // minf wrap
        let minf_box_size = (minf_payload.len() + 8) as u32;
        mdia_payload.extend_from_slice(&minf_box_size.to_be_bytes());
        mdia_payload.extend_from_slice(b"minf");
        mdia_payload.extend_from_slice(&minf_payload);

        // mdia wrap
        let mdia_box_size = (mdia_payload.len() + 8) as u32;
        trak_payload.extend_from_slice(&mdia_box_size.to_be_bytes());
        trak_payload.extend_from_slice(b"mdia");
        trak_payload.extend_from_slice(&mdia_payload);

        // trak wrap
        let trak_box_size = (trak_payload.len() + 8) as u32;
        moov_payload.extend_from_slice(&trak_box_size.to_be_bytes());
        moov_payload.extend_from_slice(b"trak");
        moov_payload.extend_from_slice(&trak_payload);

        // moov wrap
        let moov_box_size = (moov_payload.len() + 8) as u32;
        buf.extend_from_slice(&moov_box_size.to_be_bytes());
        buf.extend_from_slice(b"moov");
        buf.extend_from_slice(&moov_payload);

        // Now parse with Mp4Demuxer
        let demuxer = Mp4Demuxer::parse(&buf).expect("parse MP4");
        assert_eq!(&demuxer.major_brand, b"isom");
        assert_eq!(demuxer.timescale, 1000);
        assert_eq!(demuxer.duration, 5000);

        let video = demuxer.primary_video_track().expect("primary video track");
        assert_eq!(video.track_id, 1);
        assert_eq!(video.width, 1920);
        assert_eq!(video.height, 1080);
        assert_eq!(video.codec, VideoCodec::Avc1);
        assert_eq!(video.samples.len(), 2);
        assert_eq!(video.samples[0].offset, 1000);
        assert_eq!(video.samples[0].size, 4096);
        assert_eq!(video.samples[1].offset, 5096);
        assert_eq!(video.samples[1].size, 2048);
    }

    #[test]
    fn test_webm_container_parsing_and_demuxing() {
        let mut buf = Vec::new();

        // 1. EBML header: 0x1A45DFA3 + size (0x84 = 4 bytes) + 4 dummy bytes
        buf.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]);
        buf.extend_from_slice(&[0x84, 0x42, 0x86, 0x81, 0x01]);

        // 2. Segment container: 0x18538067
        let mut segment = Vec::new();

        // 2a. Info
        let mut info = Vec::new();
        info.extend_from_slice(&[0x2A, 0xD7, 0xB1, 0x84, 0x00, 0x0F, 0x42, 0x40]);
        segment.extend_from_slice(&[0x15, 0x49, 0xA9, 0x66]);
        segment.push(0x80 | (info.len() as u8));
        segment.extend_from_slice(&info);

        // 2b. Tracks
        let mut tracks = Vec::new();
        let mut entry = Vec::new();
        entry.extend_from_slice(&[0xD7, 0x81, 0x01]); // TrackNumber: 1
        entry.extend_from_slice(&[0x83, 0x81, 0x01]); // TrackType: 1
        entry.extend_from_slice(&[0x86, 0x85, b'V', b'_', b'V', b'P', b'8']); // CodecID: V_VP8

        let mut video = Vec::new();
        video.extend_from_slice(&[0xB0, 0x82, 0x02, 0x80]); // Width 640
        video.extend_from_slice(&[0xBA, 0x82, 0x01, 0xE0]); // Height 480
        entry.extend_from_slice(&[0xE0, 0x80 | (video.len() as u8)]);
        entry.extend_from_slice(&video);

        tracks.extend_from_slice(&[0xAE, 0x80 | (entry.len() as u8)]);
        tracks.extend_from_slice(&entry);

        segment.extend_from_slice(&[0x16, 0x54, 0xAE, 0x6B]);
        segment.push(0x80 | (tracks.len() as u8));
        segment.extend_from_slice(&tracks);

        // 2c. Cluster
        let mut cluster = Vec::new();
        cluster.extend_from_slice(&[0xE7, 0x81, 0x00]);
        let sblock = vec![0x81, 0x00, 0x00, 0x80, 1, 2, 3, 4];
        cluster.extend_from_slice(&[0xA3, 0x80 | (sblock.len() as u8)]);
        cluster.extend_from_slice(&sblock);

        segment.extend_from_slice(&[0x1F, 0x43, 0xB6, 0x75]);
        segment.push(0x80 | (cluster.len() as u8));
        segment.extend_from_slice(&cluster);

        // Segment header
        buf.extend_from_slice(&[0x18, 0x53, 0x80, 0x67]);
        buf.push(0x80 | (segment.len() as u8));
        buf.extend_from_slice(&segment);

        // Demux WebM
        let demuxer = WebmDemuxer::parse(&buf).expect("parse webm");
        assert_eq!(demuxer.timecode_scale, 1_000_000);
        let video = demuxer.primary_video_track().expect("video track");
        assert_eq!(video.track_number, 1);
        assert_eq!(video.track_type, 1);
        assert_eq!(video.codec_id, "V_VP8");
        assert_eq!(video.width, 640);
        assert_eq!(video.height, 480);
        assert_eq!(video.samples.len(), 1);
        assert!(video.samples[0].is_keyframe);
        assert_eq!(video.samples[0].size, 4);

        let sample_bytes = demuxer.get_sample_bytes(&video.samples[0], &buf).expect("sample bytes");
        assert_eq!(sample_bytes, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_animated_webp_muxing_and_chunk_structure() {
        // Create synthetic VP8L subchunks: FourCC + 4-byte len + payload
        let mut vp8l_chunk1 = Vec::new();
        vp8l_chunk1.extend_from_slice(b"VP8L");
        vp8l_chunk1.extend_from_slice(&4u32.to_le_bytes());
        vp8l_chunk1.extend_from_slice(&[10, 20, 30, 40]);

        let mut vp8l_chunk2 = Vec::new();
        vp8l_chunk2.extend_from_slice(b"VP8L");
        vp8l_chunk2.extend_from_slice(&4u32.to_le_bytes());
        vp8l_chunk2.extend_from_slice(&[50, 60, 70, 80]);

        let mut muxer = WebpAnimationMuxer::new(320, 240);
        muxer.add_frame(WebpAnimFrame { duration_ms: 120, x: 0, y: 0, width: 320, height: 240, payload: vp8l_chunk1.clone() });
        muxer.add_frame(WebpAnimFrame { duration_ms: 250, x: 0, y: 0, width: 320, height: 240, payload: vp8l_chunk2 });

        let webp_bytes = muxer.build().expect("build animated webp");
        assert!(webp_bytes.len() >= 32);
        assert_eq!(&webp_bytes[0..4], b"RIFF");
        assert_eq!(&webp_bytes[8..12], b"WEBP");
        assert_eq!(&webp_bytes[12..16], b"VP8X");
        assert_eq!(&webp_bytes[30..34], b"ANIM");

        // Verify synthetic standalone webp chunk extraction
        let mut standalone = Vec::new();
        standalone.extend_from_slice(b"RIFF");
        let stand_len = (4 + vp8l_chunk1.len()) as u32;
        standalone.extend_from_slice(&stand_len.to_le_bytes());
        standalone.extend_from_slice(b"WEBP");
        standalone.extend_from_slice(&vp8l_chunk1);

        let extracted = WebpAnimationMuxer::extract_image_chunk(&standalone).expect("extract chunk");
        assert_eq!(extracted, &vp8l_chunk1);
    }
}
