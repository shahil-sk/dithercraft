use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use rayon::prelude::*;

/// 9 CRT and LCD Temporal Noise Patterns for multi-frame video and animation simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalNoisePattern {
    CrtScanBeamJitter,
    PhosphorGrainHum,
    InterlacedFieldShimmer,
    LcdSubpixelGridDrift,
    VhsTapePhaseNoise,
    ThermalAnalogDrift,
    StaticHashWhiteNoise,
    BayerPhaseStepping,
    BlueNoiseTemporalJitter,
}

impl TemporalNoisePattern {
    pub const ALL: [Self; 9] = [
        Self::CrtScanBeamJitter,
        Self::PhosphorGrainHum,
        Self::InterlacedFieldShimmer,
        Self::LcdSubpixelGridDrift,
        Self::VhsTapePhaseNoise,
        Self::ThermalAnalogDrift,
        Self::StaticHashWhiteNoise,
        Self::BayerPhaseStepping,
        Self::BlueNoiseTemporalJitter,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::CrtScanBeamJitter => "CRT Scan Beam Jitter",
            Self::PhosphorGrainHum => "Phosphor Grain Hum",
            Self::InterlacedFieldShimmer => "Interlaced Field Shimmer",
            Self::LcdSubpixelGridDrift => "LCD Subpixel Grid Drift",
            Self::VhsTapePhaseNoise => "VHS Tape Phase Noise",
            Self::ThermalAnalogDrift => "Thermal Analog Drift",
            Self::StaticHashWhiteNoise => "Static Hash White Noise",
            Self::BayerPhaseStepping => "Bayer Phase Stepping",
            Self::BlueNoiseTemporalJitter => "Blue Noise Temporal Jitter",
        }
    }

    /// Apply temporal pattern at frame index `frame_idx` with given `intensity` (0.0 to 1.0).
    pub fn apply(&self, input: &ImageBuffer, frame_idx: u32, intensity: f32) -> Result<ImageBuffer, DitherError> {
        let mut out = input.clone();
        let width = input.width as usize;
        let height = input.height as usize;
        let t = frame_idx as f32;

        match self {
            Self::CrtScanBeamJitter => {
                // High-voltage deflection jitter: periodic horizontal shift coupled with frame
                let beam_phase = (t * 0.4).sin();
                let shift = (beam_phase * 4.0 * intensity) as i32;
                for y in 0..height {
                    let y_mod = ((y as f32 * 0.1 + t).sin() * 2.0 * intensity) as i32;
                    let total_shift = shift + y_mod;
                    for x in 0..width {
                        let src_x = (x as i32 + total_shift).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        let dst_idx = (y * width + x) * 3;
                        out.data[dst_idx] = input.data[src_idx];
                        out.data[dst_idx + 1] = input.data[src_idx + 1];
                        out.data[dst_idx + 2] = input.data[src_idx + 2];
                    }
                }
            }

            Self::PhosphorGrainHum => {
                // 50/60 Hz mains hum modulating phosphor brightness across vertical raster
                let hum = ((t * 0.5 + 0.1).sin() * 0.5 + 0.5) * intensity * 24.0;
                out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row)| {
                    let band = ((y as f32 * 0.05 + t * 0.2).sin() * hum) as i32;
                    for chunk in row.chunks_exact_mut(3) {
                        chunk[0] = (chunk[0] as i32 + band).clamp(0, 255) as u8;
                        chunk[1] = (chunk[1] as i32 + band).clamp(0, 255) as u8;
                        chunk[2] = (chunk[2] as i32 + band).clamp(0, 255) as u8;
                    }
                });
            }

            Self::InterlacedFieldShimmer => {
                // Alternate field brightness offset between even and odd frames
                let field_offset = if frame_idx.is_multiple_of(2) { 1 } else { 0 };
                let delta = (intensity * 30.0) as i32;
                out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row)| {
                    if (y % 2) == field_offset {
                        for chunk in row.chunks_exact_mut(3) {
                            chunk[0] = (chunk[0] as i32 + delta).clamp(0, 255) as u8;
                            chunk[1] = (chunk[1] as i32 + delta).clamp(0, 255) as u8;
                            chunk[2] = (chunk[2] as i32 + delta).clamp(0, 255) as u8;
                        }
                    }
                });
            }

            Self::LcdSubpixelGridDrift => {
                // Subpixel crystal response latency drift
                let drift_col = ((t * 0.1).cos() * 2.0 * intensity) as i32;
                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        let src_x = (x as i32 + drift_col).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        out.data[idx] = input.data[src_idx]; // R from shifted
                    }
                }
            }

            Self::VhsTapePhaseNoise => {
                // Lower tracking head noise band
                let head_band_y = ((t * 12.0) as usize) % height;
                let band_height = (12.0 * intensity) as usize;
                for y in head_band_y..(head_band_y + band_height).min(height) {
                    let tear = ((y as f32 * 0.8).sin() * 20.0 * intensity) as i32;
                    for x in 0..width {
                        let src_x = (x as i32 + tear).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        let dst_idx = (y * width + x) * 3;
                        out.data[dst_idx] = input.data[src_idx];
                        out.data[dst_idx + 1] = input.data[src_idx + 1];
                        out.data[dst_idx + 2] = input.data[src_idx + 2];
                    }
                }
            }

            Self::ThermalAnalogDrift => {
                // Low frequency thermal color temp drift
                let r_bias = ((t * 0.05).sin() * 15.0 * intensity) as i32;
                let b_bias = (-(t * 0.05).sin() * 15.0 * intensity) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + r_bias).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + b_bias).clamp(0, 255) as u8;
                }
            }

            Self::StaticHashWhiteNoise => {
                // Pure high-frequency pseudo-random static hash
                let mut seed = ((frame_idx as u64) << 32) ^ 0x9E3779B97F4A7C15;
                let range = (intensity * 40.0) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let noise = ((seed >> 48) as i32 % (range * 2 + 1)) - range;
                    chunk[0] = (chunk[0] as i32 + noise).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + noise).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + noise).clamp(0, 255) as u8;
                }
            }

            Self::BayerPhaseStepping => {
                // Temporal rotation of Bayer dither matrix phase
                let phase_shift = ((frame_idx % 4) * 2) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + phase_shift).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + phase_shift).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + phase_shift).clamp(0, 255) as u8;
                }
            }

            Self::BlueNoiseTemporalJitter => {
                // Blue noise golden-ratio temporal stepping (Roberts 2014)
                let phi_step = ((frame_idx as f32 * 0.618_034) % 1.0 - 0.5) * intensity * 30.0;
                let step_i = phi_step as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + step_i).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + step_i).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + step_i).clamp(0, 255) as u8;
                }
            }
        }

        Ok(out)
    }
}
