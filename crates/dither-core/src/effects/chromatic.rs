use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use rayon::prelude::*;

/// Chromatic Aberration: Radially and directionally shifts RGB color channels.
#[derive(Debug, Clone)]
pub struct ChromaticAberration {
    pub shift_x: i32,
    pub shift_y: i32,
    pub radial: bool,
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self { shift_x: 3, shift_y: 0, radial: false }
    }
}

impl ChromaticAberration {
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let height = input.height as usize;
        let center_x = (width as f32) / 2.0;
        let center_y = (height as f32) / 2.0;
        let max_radius = center_x.hypot(center_y).max(1.0);

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let y_i = y as i32;

            for x in 0..width {
                let x_i = x as i32;

                let (rx, ry, bx, by) = if self.radial {
                    let dx = (x as f32) - center_x;
                    let dy = (y as f32) - center_y;
                    let norm = (dx.hypot(dy) / max_radius).min(1.0);
                    let factor = norm * (self.shift_x.abs() as f32);
                    let off_x = (dx / (dx.hypot(dy).max(0.001)) * factor) as i32;
                    let off_y = (dy / (dx.hypot(dy).max(0.001)) * factor) as i32;
                    (x_i + off_x, y_i + off_y, x_i - off_x, y_i - off_y)
                } else {
                    (x_i + self.shift_x, y_i + self.shift_y, x_i - self.shift_x, y_i - self.shift_y)
                };

                let r_px = input.get_pixel_clamped(rx, ry);
                let g_px = input.get_pixel_clamped(x_i, y_i);
                let b_px = input.get_pixel_clamped(bx, by);

                let out_idx = x * 3;
                row_out[out_idx] = r_px.r;
                row_out[out_idx + 1] = g_px.g;
                row_out[out_idx + 2] = b_px.b;
            }
        });

        Ok(out)
    }
}
