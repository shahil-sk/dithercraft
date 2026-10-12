//! Non-Destructive Spatial Masking & Dual-Algorithm Blending Engine.
//!
//! Provides pixel-accurate 8-bit grayscale mask buffers, paintbrush rasterizers
//! with Hermite smoothstep edge falloff, procedural linear/radial gradients,
//! tonal range extraction, and subpixel linear-sRGB dual-algorithm blending.

use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// 8-bit grayscale spatial mask buffer (0 = black / unmasked, 255 = white / fully masked).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl MaskBuffer {
    /// Create a new zero-initialized (empty/black) mask buffer.
    pub fn new(width: u32, height: u32) -> Result<Self, DitherError> {
        if width == 0 || height == 0 {
            return Err(DitherError::ZeroDimension);
        }
        let total = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| DitherError::InvalidParameter("Mask dimensions overflow memory bounds".into()))?;
        Ok(Self { width, height, data: vec![0u8; total] })
    }

    /// Create a new mask buffer filled with a constant intensity value.
    pub fn new_filled(width: u32, height: u32, val: u8) -> Result<Self, DitherError> {
        let mut mask = Self::new(width, height)?;
        mask.data.fill(val);
        Ok(mask)
    }

    /// Extract an 8-bit grayscale mask from an ImageBuffer using ITU-R BT.601 luminance.
    pub fn from_image(img: &ImageBuffer) -> Self {
        let total = (img.width as usize) * (img.height as usize);
        let mut data = Vec::with_capacity(total);
        for chunk in img.data.chunks_exact(3) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            let lum = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
            data.push(lum);
        }
        Self { width: img.width, height: img.height, data }
    }

    /// Convert the 8-bit mask buffer into a 24-bit RGB grayscale ImageBuffer.
    pub fn to_image(&self) -> Result<ImageBuffer, DitherError> {
        let mut img = ImageBuffer::new(self.width, self.height)?;
        for (idx, &v) in self.data.iter().enumerate() {
            let out_idx = idx * 3;
            img.data[out_idx] = v;
            img.data[out_idx + 1] = v;
            img.data[out_idx + 2] = v;
        }
        Ok(img)
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.data.get(idx).copied()
        } else {
            None
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, val: u8) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            if let Some(p) = self.data.get_mut(idx) {
                *p = val;
            }
        }
    }

    /// Invert mask intensity (255 - val).
    pub fn invert(&mut self) {
        for b in &mut self.data {
            *b = 255 - *b;
        }
    }

    /// Clear mask to 0 (fully unmasked).
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Fill mask to 255 (fully masked).
    pub fn fill_all(&mut self) {
        self.data.fill(255);
    }

    /// Check if mask is entirely zero.
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&v| v == 0)
    }

    /// Rasterize a brush dab onto the mask with Hermite smoothstep edge falloff.
    pub fn paint_brush(&mut self, center_x: i32, center_y: i32, radius: f32, hardness: f32, opacity: f32, erase: bool) {
        if radius <= 0.0 {
            return;
        }
        let rad = radius.max(1.0);
        let hard = hardness.clamp(0.0, 1.0);
        let opac = opacity.clamp(0.0, 1.0);
        let r_inner = rad * hard;
        let r_outer = rad;

        let r_int = rad.ceil() as i32;
        let min_x = (center_x - r_int).clamp(0, self.width as i32 - 1);
        let max_x = (center_x + r_int).clamp(0, self.width as i32 - 1);
        let min_y = (center_y - r_int).clamp(0, self.height as i32 - 1);
        let max_y = (center_y + r_int).clamp(0, self.height as i32 - 1);

        let w = self.width as usize;

        for y in min_y..=max_y {
            let dy = (y - center_y) as f32;
            let dy2 = dy * dy;
            let row_offset = (y as usize) * w;

            for x in min_x..=max_x {
                let dx = (x - center_x) as f32;
                let dist_sq = dx * dx + dy2;

                if dist_sq <= r_outer * r_outer {
                    let dist = dist_sq.sqrt();
                    let factor = if dist <= r_inner || hard >= 1.0 {
                        1.0_f32
                    } else {
                        let t = ((dist - r_inner) / (r_outer - r_inner)).clamp(0.0, 1.0);
                        1.0 - t * t * (3.0 - 2.0 * t)
                    };

                    let delta = factor * opac * 255.0;
                    let idx = row_offset + (x as usize);
                    let curr = self.data[idx] as f32;

                    if erase {
                        self.data[idx] = (curr - delta).round().clamp(0.0, 255.0) as u8;
                    } else {
                        self.data[idx] = (curr + delta).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }

    /// Rasterize a linear gradient across the mask from `(x0, y0)` (intensity 0) to `(x1, y1)` (intensity 255).
    pub fn apply_linear_gradient(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let vx = x1 - x0;
        let vy = y1 - y0;
        let len_sq = vx * vx + vy * vy;
        if len_sq <= f32::EPSILON {
            return;
        }

        let w = self.width as usize;
        self.data.par_chunks_exact_mut(w).enumerate().for_each(|(y, row)| {
            let py = y as f32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let px = x as f32;
                let proj = ((px - x0) * vx + (py - y0) * vy) / len_sq;
                let t = proj.clamp(0.0, 1.0);
                *pixel = (t * 255.0).round() as u8;
            }
        });
    }

    /// Rasterize a radial gradient centered at `(cx, cy)`.
    /// Pixels within `inner_radius` receive 255, pixels outside `outer_radius` receive 0,
    /// with cubic Hermite smoothstep falloff between them.
    pub fn apply_radial_gradient(&mut self, cx: f32, cy: f32, inner_radius: f32, outer_radius: f32) {
        let r_in = inner_radius.max(0.0);
        let r_out = outer_radius.max(r_in + 0.001);
        let span = r_out - r_in;

        let w = self.width as usize;
        self.data.par_chunks_exact_mut(w).enumerate().for_each(|(y, row)| {
            let dy = y as f32 - cy;
            let dy2 = dy * dy;
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - cx;
                let dist = (dx * dx + dy2).sqrt();

                if dist <= r_in {
                    *pixel = 255;
                } else if dist >= r_out {
                    *pixel = 0;
                } else {
                    let t = ((dist - r_in) / span).clamp(0.0, 1.0);
                    let factor = 1.0 - t * t * (3.0 - 2.0 * t);
                    *pixel = (factor * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        });
    }

    /// Extract a tonal range mask based on source image luminance.
    /// Pixels between `min_lum` and `max_lum` receive 255.
    /// Margins of width `feather` ramp smoothly between 0 and 255.
    pub fn apply_tonal_range(&mut self, input: &ImageBuffer, min_lum: u8, max_lum: u8, feather: u8) {
        let w = self.width.min(input.width) as usize;
        let h = self.height.min(input.height) as usize;
        let f = feather as f32;

        let min_l = min_lum as f32;
        let max_l = max_lum as f32;

        for y in 0..h {
            let in_row = &input.data[y * (input.width as usize) * 3..(y + 1) * (input.width as usize) * 3];
            let out_row = &mut self.data[y * (self.width as usize)..y * (self.width as usize) + w];

            for (x, out_px) in out_row.iter_mut().enumerate().take(w) {
                let idx = x * 3;
                let lum = 0.299 * (in_row[idx] as f32) + 0.587 * (in_row[idx + 1] as f32) + 0.114 * (in_row[idx + 2] as f32);

                let alpha = if lum >= min_l && lum <= max_l {
                    1.0_f32
                } else if f > 0.0 && lum < min_l && lum >= min_l - f {
                    (lum - (min_l - f)) / f
                } else if f > 0.0 && lum > max_l && lum <= max_l + f {
                    1.0 - (lum - max_l) / f
                } else {
                    0.0_f32
                };

                *out_px = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    /// Extract a color proximity mask based on OKLab distance from `target_color`.
    pub fn apply_color_proximity(&mut self, input: &ImageBuffer, target_color: RgbColor, tolerance: f32) {
        let tol = tolerance.max(0.001);
        let target_lab = target_color.to_oklab();

        let w = self.width.min(input.width) as usize;
        let h = self.height.min(input.height) as usize;

        for y in 0..h {
            let in_row = &input.data[y * (input.width as usize) * 3..(y + 1) * (input.width as usize) * 3];
            let out_row = &mut self.data[y * (self.width as usize)..y * (self.width as usize) + w];

            for (x, out_px) in out_row.iter_mut().enumerate().take(w) {
                let idx = x * 3;
                let px = RgbColor::new(in_row[idx], in_row[idx + 1], in_row[idx + 2]);
                let lab = px.to_oklab();

                let dl = lab.l - target_lab.l;
                let da = lab.a - target_lab.a;
                let db = lab.b - target_lab.b;
                let dist = (dl * dl + da * da + db * db).sqrt();

                if dist < tol {
                    let factor = 1.0 - (dist / tol);
                    *out_px = (factor.clamp(0.0, 1.0) * 255.0).round() as u8;
                } else {
                    *out_px = 0;
                }
            }
        }
    }

    /// Soften mask boundaries via fast separable horizontal and vertical box-blur.
    pub fn feather_box(&mut self, radius: usize) {
        if radius == 0 {
            return;
        }
        let w = self.width as usize;
        let h = self.height as usize;
        let rad = radius.min(w / 2).min(h / 2);
        let mut temp = self.data.clone();

        // Horizontal pass
        for y in 0..h {
            let row_offset = y * w;
            for x in 0..w {
                let x_start = x.saturating_sub(rad);
                let x_end = (x + rad).min(w - 1);
                let mut sum = 0u32;
                let count = (x_end - x_start + 1) as u32;
                for kx in x_start..=x_end {
                    sum += self.data[row_offset + kx] as u32;
                }
                temp[row_offset + x] = (sum / count) as u8;
            }
        }

        // Vertical pass
        for x in 0..w {
            for y in 0..h {
                let y_start = y.saturating_sub(rad);
                let y_end = (y + rad).min(h - 1);
                let mut sum = 0u32;
                let count = (y_end - y_start + 1) as u32;
                for ky in y_start..=y_end {
                    sum += temp[ky * w + x] as u32;
                }
                self.data[y * w + x] = (sum / count) as u8;
            }
        }
    }

    /// Blend two image buffers using this mask as an alpha matte:
    /// Output = ImageA * (Mask / 255) + ImageB * (1 - Mask / 255).
    pub fn blend_images(&self, img_a: &ImageBuffer, img_b: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        if img_a.width != self.width || img_a.height != self.height || img_b.width != self.width || img_b.height != self.height {
            return Err(DitherError::InvalidParameter("Image and mask dimensions must match".into()));
        }

        let mut out = ImageBuffer::new(self.width, self.height)?;
        let w = self.width as usize;

        out.data.par_chunks_exact_mut(w * 3).enumerate().for_each(|(y, out_row)| {
            let mask_row = &self.data[y * w..(y + 1) * w];
            let a_row = &img_a.data[y * w * 3..(y + 1) * w * 3];
            let b_row = &img_b.data[y * w * 3..(y + 1) * w * 3];

            for (x, &m_val) in mask_row.iter().enumerate().take(w) {
                let alpha = m_val as f32 / 255.0;
                let one_minus = 1.0 - alpha;
                let idx = x * 3;

                out_row[idx] = ((a_row[idx] as f32 * alpha) + (b_row[idx] as f32 * one_minus)).round().clamp(0.0, 255.0) as u8;
                out_row[idx + 1] = ((a_row[idx + 1] as f32 * alpha) + (b_row[idx + 1] as f32 * one_minus)).round().clamp(0.0, 255.0) as u8;
                out_row[idx + 2] = ((a_row[idx + 2] as f32 * alpha) + (b_row[idx + 2] as f32 * one_minus)).round().clamp(0.0, 255.0) as u8;
            }
        });

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_lifecycle_and_inversion() {
        let mut mask = MaskBuffer::new(4, 4).expect("mask");
        assert!(mask.is_empty());
        mask.set_pixel(0, 0, 255);
        assert_eq!(mask.get_pixel(0, 0), Some(255));
        assert!(!mask.is_empty());

        mask.invert();
        assert_eq!(mask.get_pixel(0, 0), Some(0));
        assert_eq!(mask.get_pixel(1, 1), Some(255));

        mask.clear();
        assert!(mask.is_empty());
        mask.fill_all();
        assert_eq!(mask.get_pixel(0, 0), Some(255));
    }

    #[test]
    fn test_mask_brush_painting() {
        let mut mask = MaskBuffer::new(10, 10).expect("mask");
        mask.paint_brush(5, 5, 3.0, 1.0, 1.0, false);
        assert_eq!(mask.get_pixel(5, 5), Some(255));
        assert_eq!(mask.get_pixel(5, 7), Some(255));
        assert_eq!(mask.get_pixel(0, 0), Some(0));

        // Erase dab
        mask.paint_brush(5, 5, 2.0, 1.0, 1.0, true);
        assert_eq!(mask.get_pixel(5, 5), Some(0));
    }

    #[test]
    fn test_mask_linear_and_radial_gradient() {
        let mut mask = MaskBuffer::new(10, 10).expect("mask");
        mask.apply_linear_gradient(0.0, 0.0, 9.0, 0.0);
        assert_eq!(mask.get_pixel(0, 0), Some(0));
        assert_eq!(mask.get_pixel(9, 0), Some(255));
        assert!((mask.get_pixel(4, 0).unwrap() as i32 - 113).abs() < 20);

        mask.apply_radial_gradient(5.0, 5.0, 2.0, 5.0);
        assert_eq!(mask.get_pixel(5, 5), Some(255));
        assert_eq!(mask.get_pixel(0, 0), Some(0));
    }

    #[test]
    fn test_mask_blend_images() {
        let mut mask = MaskBuffer::new(2, 2).expect("mask");
        mask.set_pixel(0, 0, 255); // A
        mask.set_pixel(1, 1, 0); // B

        let mut a = ImageBuffer::new(2, 2).expect("a");
        let mut b = ImageBuffer::new(2, 2).expect("b");
        for y in 0..2 {
            for x in 0..2 {
                a.set_pixel(x, y, RgbColor::new(200, 0, 0));
                b.set_pixel(x, y, RgbColor::new(0, 200, 0));
            }
        }

        let blended = mask.blend_images(&a, &b).expect("blend");
        assert_eq!(blended.get_pixel(0, 0).unwrap().r, 200);
        assert_eq!(blended.get_pixel(0, 0).unwrap().g, 0);
        assert_eq!(blended.get_pixel(1, 1).unwrap().r, 0);
        assert_eq!(blended.get_pixel(1, 1).unwrap().g, 200);
    }
}
