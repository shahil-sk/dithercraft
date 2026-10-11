use crate::algorithms::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Mismatch Mode: Intentionally desynchronizes color depth quantization grids from luminosity scales.
#[derive(Debug, Clone)]
pub struct MismatchMode {
    pub bit_depth: u8,   // e.g. 2, 3, 4, 5 bits per channel
    pub grid_scale: u32, // spatial mismatch grid stride (e.g. 2, 4, 8)
    pub channel_phase: i32,
}

impl Default for MismatchMode {
    fn default() -> Self {
        Self { bit_depth: 3, grid_scale: 4, channel_phase: 24 }
    }
}

impl DitherAlgorithm for MismatchMode {
    fn name(&self) -> &'static str {
        "Color Depth Mismatch"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let bits = self.bit_depth.clamp(1, 7);
        let shift = 8 - bits;
        let mask = (0xFF << shift) as u8;
        let grid = self.grid_scale.max(1) as usize;
        let phase = self.channel_phase;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            let grid_y_offset = ((y / grid) % 2) as i32 * phase;

            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let grid_x_offset = ((x / grid) % 2) as i32 * phase;
                let offset = grid_x_offset ^ grid_y_offset;

                let r = ((input.data[in_idx] as i32 + offset).clamp(0, 255) as u8) & mask;
                let g = ((input.data[in_idx + 1] as i32 - offset).clamp(0, 255) as u8) & mask;
                let b = ((input.data[in_idx + 2] as i32 + (offset / 2)).clamp(0, 255) as u8) & mask;

                let quantized = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = quantized.r;
                row_out[out_idx + 1] = quantized.g;
                row_out[out_idx + 2] = quantized.b;
            }
        });

        Ok(out)
    }
}
