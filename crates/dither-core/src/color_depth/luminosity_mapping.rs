use crate::algorithms::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Luminosity Mapping: Directly projects pixel brightness onto ordered palette entries.
#[derive(Default, Debug, Clone)]
pub struct LuminosityMapping {
    pub invert: bool,
}

impl DitherAlgorithm for LuminosityMapping {
    fn name(&self) -> &'static str {
        "Luminosity Mapping"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut sorted_palette = palette.clone();
        sorted_palette.sort_by_luminance(true);
        let n = sorted_palette.len();

        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum = if self.invert { 255 - px.luminance() } else { px.luminance() };
                let slot = ((lum as usize * n) / 256).min(n - 1);
                let color = sorted_palette.swatches[slot].color;

                let out_idx = x * 3;
                row_out[out_idx] = color.r;
                row_out[out_idx + 1] = color.g;
                row_out[out_idx + 2] = color.b;
            }
        });

        Ok(out)
    }
}
