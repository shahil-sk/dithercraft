use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Mezzotint copperplate engraving emulation
#[derive(Debug, Clone)]
pub struct Mezzotint {
    pub grain_size: u32,
    pub contrast: f32,
}

impl Default for Mezzotint {
    fn default() -> Self {
        Self { grain_size: 2, contrast: 1.6 }
    }
}

impl DitherAlgorithm for Mezzotint {
    fn name(&self) -> &'static str {
        "Mezzotint"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let scale = self.grain_size.max(1) as u64;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let sy = (y as u64) / scale;

            for x in 0..width {
                let sx = (x as u64) / scale;
                // High-frequency pseudo-random grain hash
                let h = (sx.wrapping_mul(0x9E3779B97F4A7C15) ^ sy.wrapping_mul(0xBF58476D1CE4E5B9)) >> 32;
                let noise = ((h & 0xFF) as f32 / 255.0 - 0.5) * 128.0 * self.contrast;
                let bias = noise as i32;

                let in_idx = row_in + x * 3;
                let r = (input.data[in_idx] as i32 + bias).clamp(0, 255) as u8;
                let g = (input.data[in_idx + 1] as i32 + bias).clamp(0, 255) as u8;
                let b = (input.data[in_idx + 2] as i32 + bias).clamp(0, 255) as u8;

                let q = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Variable-density pen-and-ink Stippling
#[derive(Debug, Clone)]
pub struct Stippling {
    pub dot_density: f32,
}

impl Default for Stippling {
    fn default() -> Self {
        Self { dot_density: 1.2 }
    }
}

impl DitherAlgorithm for Stippling {
    fn name(&self) -> &'static str {
        "Stippling"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let density = self.dot_density;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;

            for x in 0..width {
                let in_idx = row_in + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum_norm = px.luminance() as f32 / 255.0; // 0.0 (black) to 1.0 (white)

                // Quasi-random threshold
                let h = ((x as u64).wrapping_mul(374761393) ^ (y as u64).wrapping_mul(668265263)) >> 32;
                let rand_val = (h & 0xFFFF) as f32 / 65535.0;

                // Probability of ink dot proportional to darkness (1 - lum)
                let prob = (1.0 - lum_norm).powf(density);
                let stipple_tone = if rand_val < prob { 0u8 } else { 255u8 };

                let target = RgbColor::new(
                    ((px.r as f32 * 0.2 + stipple_tone as f32 * 0.8).clamp(0.0, 255.0)) as u8,
                    ((px.g as f32 * 0.2 + stipple_tone as f32 * 0.8).clamp(0.0, 255.0)) as u8,
                    ((px.b as f32 * 0.2 + stipple_tone as f32 * 0.8).clamp(0.0, 255.0)) as u8,
                );

                let q = palette.find_nearest(&target);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Woodcut Simulation (Directional relief grain carving)
#[derive(Debug, Clone)]
pub struct WoodcutSimulation {
    pub line_frequency: f32,
    pub grain_angle_deg: f32,
}

impl Default for WoodcutSimulation {
    fn default() -> Self {
        Self { line_frequency: 0.1, grain_angle_deg: 20.0 }
    }
}

impl DitherAlgorithm for WoodcutSimulation {
    fn name(&self) -> &'static str {
        "Woodcut Simulation"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let rad = self.grain_angle_deg.to_radians();
        let (cos_a, sin_a) = (rad.cos(), rad.sin());
        let freq = self.line_frequency;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let y_f = y as f32;

            for x in 0..width {
                let x_f = x as f32;
                let in_idx = row_in + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum_norm = px.luminance() as f32 / 255.0;

                let u = x_f * cos_a - y_f * sin_a;
                // Carve line modulated by local tone
                let line_phase = (u * freq * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                let is_carved = line_phase > lum_norm;

                let tone = if is_carved { 0u8 } else { 255u8 };
                let target = RgbColor::new(
                    ((px.r as f32 * 0.25 + tone as f32 * 0.75).clamp(0.0, 255.0)) as u8,
                    ((px.g as f32 * 0.25 + tone as f32 * 0.75).clamp(0.0, 255.0)) as u8,
                    ((px.b as f32 * 0.25 + tone as f32 * 0.75).clamp(0.0, 255.0)) as u8,
                );

                let q = palette.find_nearest(&target);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Newsprint Grain (Rotary press newsprint ink absorbency & fiber noise)
#[derive(Debug, Clone)]
pub struct NewsprintGrain {
    pub dot_gain: f32,
    pub fiber_roughness: f32,
}

impl Default for NewsprintGrain {
    fn default() -> Self {
        Self { dot_gain: 0.2, fiber_roughness: 0.15 }
    }
}

impl DitherAlgorithm for NewsprintGrain {
    fn name(&self) -> &'static str {
        "Newsprint Grain"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;

            for x in 0..width {
                let in_idx = row_in + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);

                // Coarse screen mesh (45 deg)
                let u = (x as f32 + y as f32) * 0.2;
                let v = (x as f32 - y as f32) * 0.2;
                let screen = (u.sin() * v.sin()) * 0.5 + 0.5;

                // Paper fiber noise
                let h = ((x as u64).wrapping_mul(0x45D9F3B) ^ (y as u64).wrapping_mul(0x119DE1F3)) >> 32;
                let fiber = ((h & 0xFF) as f32 / 255.0 - 0.5) * self.fiber_roughness;

                // Mechanical dot gain in midtones
                let lum_norm = px.luminance() as f32 / 255.0;
                let gain = (lum_norm * (1.0 - lum_norm) * 4.0) * self.dot_gain;
                let total_bias = ((screen + fiber - 0.5 - gain) * 96.0) as i32;

                let r = (px.r as i32 + total_bias).clamp(0, 255) as u8;
                let g = (px.g as i32 + total_bias).clamp(0, 255) as u8;
                let b = (px.b as i32 + total_bias).clamp(0, 255) as u8;

                let q = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Etching Crosshatch (Classical fine-art multi-layer engraving crosshatching)
#[derive(Debug, Clone)]
pub struct EtchingCrosshatch {
    pub line_spacing: usize,
}

impl Default for EtchingCrosshatch {
    fn default() -> Self {
        Self { line_spacing: 6 }
    }
}

impl DitherAlgorithm for EtchingCrosshatch {
    fn name(&self) -> &'static str {
        "Etching Crosshatch"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let s = self.line_spacing.max(2);

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;

            for x in 0..width {
                let in_idx = row_in + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum = px.luminance();

                // Layer 1 (45 deg hatching): active when lum < 200
                let l1 = (x + y) % s == 0;
                // Layer 2 (135 deg hatching): active when lum < 140
                let l2 = ((x as i32 - y as i32).rem_euclid(s as i32)) == 0;
                // Layer 3 (horizontal hatching): active when lum < 80
                let l3 = y % s == 0;
                // Layer 4 (vertical hatching): active when lum < 40
                let l4 = x % s == 0;

                let has_stroke = if lum > 200 {
                    false
                } else if lum > 140 {
                    l1
                } else if lum > 80 {
                    l1 || l2
                } else if lum > 40 {
                    l1 || l2 || l3
                } else {
                    l1 || l2 || l3 || l4
                };

                let tone = if has_stroke { 0u8 } else { 255u8 };
                let target = RgbColor::new(
                    ((px.r as f32 * 0.3 + tone as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                    ((px.g as f32 * 0.3 + tone as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                    ((px.b as f32 * 0.3 + tone as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                );

                let q = palette.find_nearest(&target);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Digital retro-tech Matrix Glitch Dispersion
#[derive(Debug, Clone)]
pub struct MatrixGlitchDispersion {
    pub jitter_scale: i32,
    pub bit_stride: usize,
}

impl Default for MatrixGlitchDispersion {
    fn default() -> Self {
        Self { jitter_scale: 12, bit_stride: 4 }
    }
}

impl DitherAlgorithm for MatrixGlitchDispersion {
    fn name(&self) -> &'static str {
        "Matrix Glitch Dispersion"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let stride = self.bit_stride.max(1);
        let jitter = self.jitter_scale;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let row_hash = ((y as u64).wrapping_mul(0x9E3779B9) >> 32) as i32;
            let row_shift = (row_hash % (jitter * 2 + 1)) - jitter;

            for x in 0..width {
                let cell_x = (x / stride) as i32;
                let cell_hash = (cell_x.wrapping_mul(0x5BD1E995) ^ row_hash) & 0x1F;
                let bit_disp = (cell_hash - 16) * 2;

                let src_x = (x as i32 + row_shift).clamp(0, (width as i32) - 1) as usize;
                let in_idx = row_in + src_x * 3;

                let r = (input.data[in_idx] as i32 + bit_disp).clamp(0, 255) as u8;
                let g = (input.data[in_idx + 1] as i32 - bit_disp).clamp(0, 255) as u8;
                let b = (input.data[in_idx + 2] as i32 + (bit_disp / 2)).clamp(0, 255) as u8;

                let q = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}
