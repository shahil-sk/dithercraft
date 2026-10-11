use crate::algorithms::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// CMYK Halftone Four-Color Process Separations with standard angled screens.
#[derive(Debug, Clone)]
pub struct CmykHalftone {
    pub frequency: f32,
    pub cyan_angle_deg: f32,
    pub magenta_angle_deg: f32,
    pub yellow_angle_deg: f32,
    pub black_angle_deg: f32,
}

impl Default for CmykHalftone {
    fn default() -> Self {
        Self { frequency: 0.15, cyan_angle_deg: 15.0, magenta_angle_deg: 75.0, yellow_angle_deg: 0.0, black_angle_deg: 45.0 }
    }
}

impl CmykHalftone {
    #[inline]
    fn screen_val(x: f32, y: f32, angle_rad: f32, freq: f32) -> f32 {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let u = x * cos_a - y * sin_a;
        let v = x * sin_a + y * cos_a;

        // 2D sinusoid halftone threshold in [0.0, 1.0]
        ((u * freq * std::f32::consts::TAU).sin() * (v * freq * std::f32::consts::TAU).sin()) * 0.5 + 0.5
    }
}

impl DitherAlgorithm for CmykHalftone {
    fn name(&self) -> &'static str {
        "CMYK Halftone"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        let c_rad = self.cyan_angle_deg.to_radians();
        let m_rad = self.magenta_angle_deg.to_radians();
        let y_rad = self.yellow_angle_deg.to_radians();
        let k_rad = self.black_angle_deg.to_radians();
        let freq = self.frequency;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(py, row_out)| {
            let row_in_start = py * width * 3;
            let y_f = py as f32;

            for px in 0..width {
                let x_f = px as f32;
                let in_idx = row_in_start + px * 3;

                let r_norm = input.data[in_idx] as f32 / 255.0;
                let g_norm = input.data[in_idx + 1] as f32 / 255.0;
                let b_norm = input.data[in_idx + 2] as f32 / 255.0;

                // RGB to CMYK
                let max_rgb = r_norm.max(g_norm).max(b_norm);
                let k = 1.0 - max_rgb;
                let (c, m, y) = if k < 1.0 {
                    ((1.0 - r_norm - k) / (1.0 - k), (1.0 - g_norm - k) / (1.0 - k), (1.0 - b_norm - k) / (1.0 - k))
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Compare each separation against its rotated screen
                let c_dot = if c > Self::screen_val(x_f, y_f, c_rad, freq) { 1.0 } else { 0.0 };
                let m_dot = if m > Self::screen_val(x_f, y_f, m_rad, freq) { 1.0 } else { 0.0 };
                let y_dot = if y > Self::screen_val(x_f, y_f, y_rad, freq) { 1.0 } else { 0.0 };
                let k_dot = if k > Self::screen_val(x_f, y_f, k_rad, freq) { 1.0 } else { 0.0 };

                // Recomposite CMYK to RGB
                let r_res = ((1.0 - c_dot) * (1.0 - k_dot) * 255.0) as u8;
                let g_res = ((1.0 - m_dot) * (1.0 - k_dot) * 255.0) as u8;
                let b_res = ((1.0 - y_dot) * (1.0 - k_dot) * 255.0) as u8;

                let quantized = palette.find_nearest(&RgbColor::new(r_res, g_res, b_res));
                let out_idx = px * 3;
                row_out[out_idx] = quantized.r;
                row_out[out_idx + 1] = quantized.g;
                row_out[out_idx + 2] = quantized.b;
            }
        });

        Ok(out)
    }
}
