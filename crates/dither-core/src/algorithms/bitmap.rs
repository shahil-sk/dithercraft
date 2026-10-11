use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// Fixed Threshold at luminance 128 (or mid-point).
#[derive(Default)]
pub struct FixedThreshold {
    pub threshold: u8,
}
impl FixedThreshold {
    pub fn new(threshold: u8) -> Self {
        Self { threshold }
    }
}
impl DitherAlgorithm for FixedThreshold {
    fn name(&self) -> &'static str {
        "Fixed Threshold"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let thresh = if self.threshold == 0 { 128 } else { self.threshold };

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let px = RgbColor::new(input.data[in_idx], input.data[in_idx + 1], input.data[in_idx + 2]);
                let lum = px.luminance();
                let binarized = if lum >= thresh { RgbColor::WHITE } else { RgbColor::BLACK };
                let q = palette.find_nearest(&binarized);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });
        Ok(out)
    }
}

/// Otsu's Global Adaptive Thresholding (Nobuyuki Otsu 1979)
/// Computes optimal histogram threshold minimizing intra-class variance.
#[derive(Default)]
pub struct OtsuThreshold;
impl OtsuThreshold {
    pub fn calculate_threshold(input: &ImageBuffer) -> u8 {
        let mut hist = [0u32; 256];
        let total = input.pixel_count() as f64;
        if total == 0.0 {
            return 128;
        }

        for chunk in input.data.chunks_exact(3) {
            let lum = RgbColor::new(chunk[0], chunk[1], chunk[2]).luminance() as usize;
            hist[lum] += 1;
        }

        let mut sum_all = 0.0;
        for (i, &count) in hist.iter().enumerate() {
            sum_all += (i as f64) * (count as f64);
        }

        let mut sum_b = 0.0;
        let mut w_b = 0.0;
        let mut max_variance = 0.0;
        let mut best_thresh = 128u8;

        for (t, &count) in hist.iter().enumerate() {
            w_b += count as f64;
            if w_b == 0.0 {
                continue;
            }
            let w_f = total - w_b;
            if w_f == 0.0 {
                break;
            }

            sum_b += (t as f64) * (count as f64);
            let m_b = sum_b / w_b;
            let m_f = (sum_all - sum_b) / w_f;

            let between_variance = w_b * w_f * (m_b - m_f) * (m_b - m_f);
            if between_variance > max_variance {
                max_variance = between_variance;
                best_thresh = t as u8;
            }
        }
        best_thresh
    }
}
impl DitherAlgorithm for OtsuThreshold {
    fn name(&self) -> &'static str {
        "Adaptive Otsu Threshold"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let thresh = Self::calculate_threshold(input);
        FixedThreshold::new(thresh).apply(input, palette)
    }
}

/// Mean Global Thresholding
#[derive(Default)]
pub struct MeanThreshold;
impl DitherAlgorithm for MeanThreshold {
    fn name(&self) -> &'static str {
        "Mean Threshold"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let total = input.pixel_count();
        if total == 0 {
            return Err(DitherError::ZeroDimension);
        }

        let sum: u64 = input.data.chunks_exact(3).map(|chunk| RgbColor::new(chunk[0], chunk[1], chunk[2]).luminance() as u64).sum();
        let mean = (sum / total as u64) as u8;
        FixedThreshold::new(mean).apply(input, palette)
    }
}

/// Multi-Level Step Quantization
#[derive(Default)]
pub struct MultiLevelStep {
    pub levels: u8,
}
impl MultiLevelStep {
    pub fn new(levels: u8) -> Self {
        Self { levels: levels.max(2) }
    }
}
impl DitherAlgorithm for MultiLevelStep {
    fn name(&self) -> &'static str {
        "Multi-Level Step"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let levels = self.levels.max(2) as u32;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let r = input.data[in_idx] as u32;
                let g = input.data[in_idx + 1] as u32;
                let b = input.data[in_idx + 2] as u32;

                // Quantize to steps
                let step_r = ((r * levels / 256) * 255 / (levels - 1)) as u8;
                let step_g = ((g * levels / 256) * 255 / (levels - 1)) as u8;
                let step_b = ((b * levels / 256) * 255 / (levels - 1)) as u8;

                let q = palette.find_nearest(&RgbColor::new(step_r, step_g, step_b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });
        Ok(out)
    }
}
