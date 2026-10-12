use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Hermite smoothstep interpolation between edge0 and edge1.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Selective spatial dithering dividing the tonal range into Shadows, Midtones,
/// and Highlights with independent algorithms and smooth feathered transitions.
pub struct TonalBandDither {
    pub shadow_cutoff: u8,
    pub highlight_cutoff: u8,
    pub feather: u8,
    pub shadow_algo: Box<dyn DitherAlgorithm>,
    pub midtone_algo: Box<dyn DitherAlgorithm>,
    pub highlight_algo: Box<dyn DitherAlgorithm>,
}

impl TonalBandDither {
    /// Construct a new tonal band multi-algorithm ditherer.
    pub fn new(
        shadow_cutoff: u8,
        highlight_cutoff: u8,
        feather: u8,
        shadow_algo: Box<dyn DitherAlgorithm>,
        midtone_algo: Box<dyn DitherAlgorithm>,
        highlight_algo: Box<dyn DitherAlgorithm>,
    ) -> Self {
        Self { shadow_cutoff, highlight_cutoff, feather, shadow_algo, midtone_algo, highlight_algo }
    }

    /// Compute weights `(w_shadow, w_mid, w_hi)` for a given source pixel luminance in [0, 255].
    #[inline]
    pub fn compute_weights(&self, lum: u8) -> (f32, f32, f32) {
        let l = lum as f32;
        let s_cut = self.shadow_cutoff as f32;
        let h_cut = self.highlight_cutoff as f32;
        let f = self.feather.max(1) as f32;

        let s_w = 1.0 - smoothstep(s_cut - f, s_cut + f, l);
        let h_w = smoothstep(h_cut - f, h_cut + f, l);
        let m_w = (1.0 - s_w - h_w).max(0.0);

        let total = (s_w + m_w + h_w).max(0.001);
        (s_w / total, m_w / total, h_w / total)
    }
}

impl DitherAlgorithm for TonalBandDither {
    fn name(&self) -> &'static str {
        "Tonal Band Multi-Dither"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        // Render each band's algorithm pass
        let shadow_pass = self.shadow_algo.apply(input, palette)?;
        let midtone_pass = self.midtone_algo.apply(input, palette)?;
        let highlight_pass = self.highlight_algo.apply(input, palette)?;

        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            for x in 0..width {
                let px_x = x as u32;
                let px_y = y as u32;
                let src_px = input.get_pixel(px_x, px_y).unwrap_or(RgbColor::WHITE);
                let lum = src_px.luminance();
                let (w_s, w_m, w_h) = self.compute_weights(lum);

                let s_px = shadow_pass.get_pixel(px_x, px_y).unwrap_or(RgbColor::BLACK);
                let m_px = midtone_pass.get_pixel(px_x, px_y).unwrap_or(RgbColor::new(128, 128, 128));
                let h_px = highlight_pass.get_pixel(px_x, px_y).unwrap_or(RgbColor::WHITE);

                let blended_r = (s_px.r as f32 * w_s + m_px.r as f32 * w_m + h_px.r as f32 * w_h).clamp(0.0, 255.0) as u8;
                let blended_g = (s_px.g as f32 * w_s + m_px.g as f32 * w_m + h_px.g as f32 * w_h).clamp(0.0, 255.0) as u8;
                let blended_b = (s_px.b as f32 * w_s + m_px.b as f32 * w_m + h_px.b as f32 * w_h).clamp(0.0, 255.0) as u8;

                let nearest = palette.find_nearest(&RgbColor::new(blended_r, blended_g, blended_b));
                let out_idx = x * 3;
                row_out[out_idx] = nearest.r;
                row_out[out_idx + 1] = nearest.g;
                row_out[out_idx + 2] = nearest.b;
            }
        });

        Ok(out)
    }
}

/// Alpha Mask Dither: Combines two dithering algorithms using an arbitrary grayscale mask buffer.
/// 255 in the mask renders 100% Algorithm A, 0 renders 100% Algorithm B.
pub struct AlphaMaskDither {
    pub algo_a: Box<dyn DitherAlgorithm>,
    pub algo_b: Box<dyn DitherAlgorithm>,
    pub mask: ImageBuffer,
}

impl AlphaMaskDither {
    pub fn new(algo_a: Box<dyn DitherAlgorithm>, algo_b: Box<dyn DitherAlgorithm>, mask: ImageBuffer) -> Self {
        Self { algo_a, algo_b, mask }
    }
}

impl DitherAlgorithm for AlphaMaskDither {
    fn name(&self) -> &'static str {
        "Alpha Mask Dither"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        let pass_a = self.algo_a.apply(input, palette)?;
        let pass_b = self.algo_b.apply(input, palette)?;

        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            for x in 0..width {
                let px_x = x as u32;
                let px_y = y as u32;
                let mask_px = self.mask.get_pixel(px_x, px_y).unwrap_or(RgbColor::WHITE);
                let alpha = mask_px.luminance() as f32 / 255.0;

                let px_a = pass_a.get_pixel(px_x, px_y).unwrap_or(RgbColor::WHITE);
                let px_b = pass_b.get_pixel(px_x, px_y).unwrap_or(RgbColor::BLACK);

                let blended_r = (px_a.r as f32 * alpha + px_b.r as f32 * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
                let blended_g = (px_a.g as f32 * alpha + px_b.g as f32 * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
                let blended_b = (px_a.b as f32 * alpha + px_b.b as f32 * (1.0 - alpha)).clamp(0.0, 255.0) as u8;

                let nearest = palette.find_nearest(&RgbColor::new(blended_r, blended_g, blended_b));
                let out_idx = x * 3;
                row_out[out_idx] = nearest.r;
                row_out[out_idx + 1] = nearest.g;
                row_out[out_idx + 2] = nearest.b;
            }
        });

        Ok(out)
    }
}
