use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Gaussian Blur: Separable 1D Gaussian convolution across horizontal and vertical axes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianBlur {
    pub sigma: f32, // standard deviation (e.g. 1.0 - 5.0)
}

impl Default for GaussianBlur {
    fn default() -> Self {
        Self { sigma: 1.5 }
    }
}

impl GaussianBlur {
    pub fn new(sigma: f32) -> Self {
        Self { sigma: sigma.max(0.1) }
    }

    /// Precomputes normalized 1D Gaussian kernel weights.
    fn compute_kernel(&self) -> (Vec<f32>, i32) {
        let radius = (self.sigma * 3.0).ceil().max(1.0) as i32;
        let mut weights = Vec::with_capacity((radius * 2 + 1) as usize);
        let two_sigma_sq = 2.0 * self.sigma * self.sigma;
        let mut sum = 0.0f32;

        for x in -radius..=radius {
            let w = (-((x * x) as f32) / two_sigma_sq).exp();
            weights.push(w);
            sum += w;
        }

        // Normalize
        for w in &mut weights {
            *w /= sum;
        }

        (weights, radius)
    }

    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let width = input.width as usize;
        let height = input.height as usize;
        let (kernel, radius) = self.compute_kernel();

        // Intermediate buffer for horizontal pass: stored as f32 for precision
        let mut temp = vec![0.0f32; width * height * 3];

        // 1. Horizontal Pass (Rayon parallel over rows)
        temp.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            for x in 0..width {
                let mut acc_r = 0.0f32;
                let mut acc_g = 0.0f32;
                let mut acc_b = 0.0f32;

                for (ki, dx) in (-radius..=radius).enumerate() {
                    let k_weight = kernel[ki];
                    let sample_x = (x as i32 + dx).clamp(0, (width as i32) - 1) as usize;
                    let src_idx = (y * width + sample_x) * 3;
                    acc_r += input.data[src_idx] as f32 * k_weight;
                    acc_g += input.data[src_idx + 1] as f32 * k_weight;
                    acc_b += input.data[src_idx + 2] as f32 * k_weight;
                }

                let out_idx = x * 3;
                row_out[out_idx] = acc_r;
                row_out[out_idx + 1] = acc_g;
                row_out[out_idx + 2] = acc_b;
            }
        });

        // 2. Vertical Pass (Rayon parallel over output rows)
        let mut out = ImageBuffer::new(input.width, input.height)?;
        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            for x in 0..width {
                let mut acc_r = 0.0f32;
                let mut acc_g = 0.0f32;
                let mut acc_b = 0.0f32;

                for (ki, dy) in (-radius..=radius).enumerate() {
                    let k_weight = kernel[ki];
                    let sample_y = (y as i32 + dy).clamp(0, (height as i32) - 1) as usize;
                    let src_idx = (sample_y * width + x) * 3;
                    acc_r += temp[src_idx] * k_weight;
                    acc_g += temp[src_idx + 1] * k_weight;
                    acc_b += temp[src_idx + 2] * k_weight;
                }

                let out_idx = x * 3;
                row_out[out_idx] = acc_r.clamp(0.0, 255.0) as u8;
                row_out[out_idx + 1] = acc_g.clamp(0.0, 255.0) as u8;
                row_out[out_idx + 2] = acc_b.clamp(0.0, 255.0) as u8;
            }
        });

        Ok(out)
    }
}

/// Sharpen: High-pass unsharp mask filter emphasizing edge gradients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sharpen {
    pub amount: f32, // Sharpening strength (0.0 to 3.0, default 0.8)
    pub sigma: f32,  // Unsharp mask blur scale (default 1.0)
}

impl Default for Sharpen {
    fn default() -> Self {
        Self { amount: 0.8, sigma: 1.0 }
    }
}

impl Sharpen {
    pub fn new(amount: f32, sigma: f32) -> Self {
        Self { amount: amount.max(0.0), sigma: sigma.max(0.1) }
    }

    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let blurred = GaussianBlur::new(self.sigma).apply(input)?;
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let amount = self.amount;

        out.data.par_chunks_exact_mut(3).enumerate().for_each(|(i, out_pixel)| {
            let orig_idx = i * 3;
            let orig_r = input.data[orig_idx] as f32;
            let orig_g = input.data[orig_idx + 1] as f32;
            let orig_b = input.data[orig_idx + 2] as f32;

            let blur_r = blurred.data[orig_idx] as f32;
            let blur_g = blurred.data[orig_idx + 1] as f32;
            let blur_b = blurred.data[orig_idx + 2] as f32;

            let sharp_r = orig_r + amount * (orig_r - blur_r);
            let sharp_g = orig_g + amount * (orig_g - blur_g);
            let sharp_b = orig_b + amount * (orig_b - blur_b);

            out_pixel[0] = sharp_r.clamp(0.0, 255.0) as u8;
            out_pixel[1] = sharp_g.clamp(0.0, 255.0) as u8;
            out_pixel[2] = sharp_b.clamp(0.0, 255.0) as u8;
        });

        Ok(out)
    }
}
