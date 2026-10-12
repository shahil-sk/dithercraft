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
}
