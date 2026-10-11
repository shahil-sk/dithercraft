use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use rayon::prelude::*;

/// Vintage CRT Simulation with scanlines, phosphor glow, barrel curvature, and vignette.
#[derive(Debug, Clone)]
pub struct VintageCrtSimulation {
    pub scanline_intensity: f32, // 0.0 to 1.0 (default 0.25)
    pub barrel_curvature: f32,   // 0.0 to 0.5 (default 0.08)
    pub vignette_intensity: f32, // 0.0 to 1.0 (default 0.3)
}

impl Default for VintageCrtSimulation {
    fn default() -> Self {
        Self { scanline_intensity: 0.25, barrel_curvature: 0.08, vignette_intensity: 0.3 }
    }
}

impl VintageCrtSimulation {
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        let w_f = width as f32;
        let h_f = height as f32;
        let k = self.barrel_curvature;
        let scan_int = self.scanline_intensity;
        let vig_int = self.vignette_intensity;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let ny = (y as f32 / h_f) * 2.0 - 1.0; // [-1.0, 1.0]
            let is_scanline = (y % 2) == 1;
            let scan_factor = if is_scanline { 1.0 - scan_int * 0.5 } else { 1.0 };

            for x in 0..width {
                let nx = (x as f32 / w_f) * 2.0 - 1.0; // [-1.0, 1.0]

                // Barrel curvature distortion
                let r2 = nx * nx + ny * ny;
                let (src_x, src_y) = if k > 0.0 {
                    let factor = 1.0 + k * r2;
                    let distorted_nx = nx * factor;
                    let distorted_ny = ny * factor;
                    let sx = ((distorted_nx + 1.0) * 0.5 * w_f) as i32;
                    let sy = ((distorted_ny + 1.0) * 0.5 * h_f) as i32;
                    (sx, sy)
                } else {
                    (x as i32, y as i32)
                };

                let px = if src_x >= 0 && src_x < width as i32 && src_y >= 0 && src_y < height as i32 {
                    input.get_pixel(src_x as u32, src_y as u32).unwrap_or(RgbColor::BLACK)
                } else {
                    RgbColor::BLACK // Outside barrel curvature boundary is black bezel
                };

                // Vignette calculation
                let vig = 1.0 - (r2 * 0.5 * vig_int).clamp(0.0, 1.0);
                let factor = scan_factor * vig;

                let out_idx = x * 3;
                row_out[out_idx] = ((px.r as f32 * factor).clamp(0.0, 255.0)) as u8;
                row_out[out_idx + 1] = ((px.g as f32 * factor).clamp(0.0, 255.0)) as u8;
                row_out[out_idx + 2] = ((px.b as f32 * factor).clamp(0.0, 255.0)) as u8;
            }
        });

        Ok(out)
    }
}
