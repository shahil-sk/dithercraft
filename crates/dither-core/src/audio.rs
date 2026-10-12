//! Pure-Rust Audio Ingestion and Frequency Band Modulation for Dynamic DSP Dithering.
//!
//! Provides a zero-dependency, safe RIFF WAV audio decoder and windowed spectrum analyzer,
//! allowing musical rhythm and frequency bands (bass, mid, treble, RMS) to drive live
//! dither thresholds, temporal jitter, glitch bursts, and CRT scanlines.

use crate::error::DitherError;

/// Three-band frequency envelope and overall RMS loudness for a temporal slice.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AudioSpectrum {
    /// Low frequency energy (sub-bass / kick drums, ~20–250 Hz).
    pub bass: f32,
    /// Mid frequency energy (vocals / snares / guitars, ~250–4000 Hz).
    pub mid: f32,
    /// High frequency energy (cymbals / hi-hats / transients, ~4000–20000 Hz).
    pub treble: f32,
    /// Root-mean-square amplitude across the entire window [0.0, 1.0].
    pub rms: f32,
}

/// Decoded mono/stereo PCM audio stream in memory.
#[derive(Debug, Clone, Default)]
pub struct AudioTrack {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved normalized floating point samples in `[-1.0, 1.0]`.
    pub samples: Vec<f32>,
}

impl AudioTrack {
    /// Parse a RIFF WAV audio buffer with zero external dependencies.
    pub fn parse_wav(bytes: &[u8]) -> Result<Self, DitherError> {
        if bytes.len() < 44 {
            return Err(DitherError::InvalidParameter("WAV header too short".into()));
        }

        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(DitherError::InvalidParameter("Not a valid RIFF/WAVE file".into()));
        }

        let mut offset = 12;
        let mut sample_rate = 44100u32;
        let mut channels = 2u16;
        let mut bits_per_sample = 16u16;
        let mut raw_samples: Option<&[u8]> = None;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]) as usize;
            offset += 8;

            if chunk_id == b"fmt " && offset + chunk_size <= bytes.len() {
                channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                sample_rate = u32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]);
                bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);
            } else if chunk_id == b"data" && offset + chunk_size <= bytes.len() {
                raw_samples = Some(&bytes[offset..offset + chunk_size]);
                break;
            }

            offset += chunk_size;
        }

        let data = raw_samples.ok_or_else(|| DitherError::InvalidParameter("No WAV 'data' chunk found".into()))?;

        let mut samples = Vec::new();
        match bits_per_sample {
            8 => {
                for &b in data {
                    samples.push((b as f32 - 128.0) / 128.0);
                }
            }
            16 => {
                for chunk in data.chunks_exact(2) {
                    let s = i16::from_le_bytes([chunk[0], chunk[1]]);
                    samples.push(s as f32 / 32768.0);
                }
            }
            32 => {
                for chunk in data.chunks_exact(4) {
                    let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(s.clamp(-1.0, 1.0));
                }
            }
            _ => {
                return Err(DitherError::InvalidParameter(format!("Unsupported bit depth: {bits_per_sample}")));
            }
        }

        Ok(Self { sample_rate, channels, samples })
    }

    /// Calculate audio spectrum for a specific animation frame index and target FPS.
    pub fn get_frame_spectrum(&self, frame_idx: u32, target_fps: u32) -> AudioSpectrum {
        if self.samples.is_empty() || self.sample_rate == 0 || target_fps == 0 {
            return AudioSpectrum::default();
        }

        let samples_per_frame = (self.sample_rate / target_fps) as usize * self.channels as usize;
        let start_sample = (frame_idx as usize * samples_per_frame).min(self.samples.len());
        let end_sample = (start_sample + samples_per_frame).min(self.samples.len());

        let window = &self.samples[start_sample..end_sample];
        if window.is_empty() {
            return AudioSpectrum::default();
        }

        // Compute RMS loudness
        let mut sum_sq = 0.0f32;
        let mut bass_acc = 0.0f32;
        let mut mid_acc = 0.0f32;
        let mut treble_acc = 0.0f32;

        let mut prev = 0.0f32;
        for &s in window {
            let abs_s = s.abs();
            sum_sq += s * s;

            // Simple zero-crossing / derivative heuristic for band differentiation
            let diff = (s - prev).abs();
            if diff < 0.15 {
                bass_acc += abs_s;
            } else if diff < 0.45 {
                mid_acc += abs_s;
            } else {
                treble_acc += abs_s;
            }
            prev = s;
        }

        let n = window.len() as f32;
        let rms = (sum_sq / n).sqrt().clamp(0.0, 1.0);
        let bass = (bass_acc / n * 2.5).clamp(0.0, 1.0);
        let mid = (mid_acc / n * 2.0).clamp(0.0, 1.0);
        let treble = (treble_acc / n * 3.0).clamp(0.0, 1.0);

        AudioSpectrum { bass, mid, treble, rms }
    }

    /// Export the complete audio track as a standard 16-bit PCM RIFF WAV byte buffer.
    pub fn export_wav(&self) -> Vec<u8> {
        self.export_samples_wav(&self.samples)
    }

    /// Export an audio slice synchronized to an animation frame range at a target frame rate.
    pub fn export_wav_slice(&self, num_frames: u32, target_fps: u32) -> Vec<u8> {
        if self.samples.is_empty() || self.sample_rate == 0 || target_fps == 0 {
            return self.export_wav();
        }
        let total_samples = (num_frames as usize * self.sample_rate as usize * self.channels as usize) / target_fps as usize;
        let slice = if total_samples < self.samples.len() { &self.samples[..total_samples] } else { &self.samples[..] };
        self.export_samples_wav(slice)
    }

    fn export_samples_wav(&self, samples: &[f32]) -> Vec<u8> {
        let sample_rate = if self.sample_rate == 0 { 44100 } else { self.sample_rate };
        let channels = if self.channels == 0 { 1 } else { self.channels };
        let bits_per_sample = 16u16;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;
        let data_size = (samples.len() * 2) as u32;
        let riff_size = 36 + data_size;

        let mut out = Vec::with_capacity(44 + data_size as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&riff_size.to_le_bytes());
        out.extend_from_slice(b"WAVE");

        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits_per_sample.to_le_bytes());

        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_size.to_le_bytes());
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0) as i16;
            out.extend_from_slice(&val.to_le_bytes());
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_audio_parsing_and_spectrum() {
        // Construct a synthetic 44-byte standard 16-bit PCM WAV in memory
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + 32u32).to_le_bytes()); // file size - 8
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono (1 channel)
        wav.extend_from_slice(&44100u32.to_le_bytes()); // 44100 Hz
        wav.extend_from_slice(&(44100u32 * 2).to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk with 16 samples
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&32u32.to_le_bytes());
        for i in 0..16 {
            let s = if i % 2 == 0 { 16000i16 } else { -16000i16 };
            wav.extend_from_slice(&s.to_le_bytes());
        }

        let track = AudioTrack::parse_wav(&wav).expect("parse wav");
        assert_eq!(track.sample_rate, 44100);
        assert_eq!(track.channels, 1);
        assert_eq!(track.samples.len(), 16);

        let spectrum = track.get_frame_spectrum(0, 30);
        assert!(spectrum.rms > 0.0);

        // Test export roundtrip
        let exported = track.export_wav();
        assert_eq!(exported.len(), 44 + 32);
        let track2 = AudioTrack::parse_wav(&exported).expect("parse exported wav");
        assert_eq!(track2.sample_rate, track.sample_rate);
        assert_eq!(track2.channels, track.channels);
        assert_eq!(track2.samples.len(), track.samples.len());

        // Test slice export
        let sliced = track.export_wav_slice(1, 30);
        assert!(!sliced.is_empty());
    }
}
