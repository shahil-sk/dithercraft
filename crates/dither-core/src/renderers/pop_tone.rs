use crate::algorithms::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Pop Tone high-contrast graphic midtone remapping.
#[derive(Debug, Clone)]
pub struct PopTone {
    pub contrast_boost: f32,
    pub midtone_bias: f32,
}

impl Default for PopTone {
    fn default() -> Self {
        Self { contrast_boost: 2.5, midtone_bias: 0.5 }
    }
}

impl DitherAlgorithm for PopTone {
    fn name(&self) -> &'static str {
        "Pop Tone"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let k = self.contrast_boost;
        let bias = self.midtone_bias;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum_norm = px.luminance() as f32 / 255.0;

                // High-gain Sigmoidal remap
                let shifted = lum_norm - bias;
                let s_curve = 1.0 / (1.0 + (-k * shifted * 10.0).exp());
                let remapped_val = (s_curve * 255.0).clamp(0.0, 255.0) as u8;

                // Modulate RGB channels with the remapped tone
                let boost_px = RgbColor::new(
                    ((px.r as f32 * 0.3 + remapped_val as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                    ((px.g as f32 * 0.3 + remapped_val as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                    ((px.b as f32 * 0.3 + remapped_val as f32 * 0.7).clamp(0.0, 255.0)) as u8,
                );

                let quantized = palette.find_nearest(&boost_px);
                let out_idx = x * 3;
                row_out[out_idx] = quantized.r;
                row_out[out_idx + 1] = quantized.g;
                row_out[out_idx + 2] = quantized.b;
            }
        });

        Ok(out)
    }
}
