use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use serde::{Deserialize, Serialize};

/// Epsilon Glow: Diffusion bloom filter extracting highlights and convolving luminous halos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpsilonGlow {
    pub threshold: u8,  // Highlight threshold (e.g. 180)
    pub radius: usize,  // Blur radius in pixels (e.g. 4)
    pub intensity: f32, // Blend intensity (e.g. 0.8)
}

impl Default for EpsilonGlow {
    fn default() -> Self {
        Self { threshold: 180, radius: 4, intensity: 0.75 }
    }
}

impl EpsilonGlow {
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let width = input.width as usize;
        let height = input.height as usize;
        let r = self.radius.max(1);

        // 1. Extract highlights
        let mut bloom = vec![0u8; input.data.len()];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 3;
                let px = RgbColor::new(input.data[idx], input.data[idx + 1], input.data[idx + 2]);
                if px.luminance() >= self.threshold {
                    bloom[idx] = px.r;
                    bloom[idx + 1] = px.g;
                    bloom[idx + 2] = px.b;
                }
            }
        }

        // 2. Horizontal separable box blur
        let mut temp = vec![0u8; bloom.len()];
        let window_size = (r * 2 + 1) as i32;

        for y in 0..height {
            let row_offset = y * width * 3;
            for x in 0..width {
                let mut acc_r = 0i32;
                let mut acc_g = 0i32;
                let mut acc_b = 0i32;
                for dx in -(r as i32)..=(r as i32) {
                    let cx = (x as i32 + dx).clamp(0, (width as i32) - 1) as usize;
                    let c_idx = row_offset + cx * 3;
                    acc_r += bloom[c_idx] as i32;
                    acc_g += bloom[c_idx + 1] as i32;
                    acc_b += bloom[c_idx + 2] as i32;
                }
                let out_idx = row_offset + x * 3;
                temp[out_idx] = (acc_r / window_size) as u8;
                temp[out_idx + 1] = (acc_g / window_size) as u8;
                temp[out_idx + 2] = (acc_b / window_size) as u8;
            }
        }

        // 3. Vertical pass and additive blend with original
        let mut out = ImageBuffer::new(input.width, input.height)?;
        for y in 0..height {
            for x in 0..width {
                let mut acc_r = 0i32;
                let mut acc_g = 0i32;
                let mut acc_b = 0i32;
                for dy in -(r as i32)..=(r as i32) {
                    let cy = (y as i32 + dy).clamp(0, (height as i32) - 1) as usize;
                    let c_idx = (cy * width + x) * 3;
                    acc_r += temp[c_idx] as i32;
                    acc_g += temp[c_idx + 1] as i32;
                    acc_b += temp[c_idx + 2] as i32;
                }
                let glow_r = (acc_r / window_size) as f32 * self.intensity;
                let glow_g = (acc_g / window_size) as f32 * self.intensity;
                let glow_b = (acc_b / window_size) as f32 * self.intensity;

                let orig_idx = (y * width + x) * 3;
                let r_final = (input.data[orig_idx] as f32 + glow_r).min(255.0) as u8;
                let g_final = (input.data[orig_idx + 1] as f32 + glow_g).min(255.0) as u8;
                let b_final = (input.data[orig_idx + 2] as f32 + glow_b).min(255.0) as u8;

                out.data[orig_idx] = r_final;
                out.data[orig_idx + 1] = g_final;
                out.data[orig_idx + 2] = b_final;
            }
        }

        Ok(out)
    }
}
