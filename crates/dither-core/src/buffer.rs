use crate::color::RgbColor;
use crate::error::DitherError;

/// Continuous 24-bit sRGB raw pixel buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageBuffer {
    /// Create a new blank (black) image buffer.
    pub fn new(width: u32, height: u32) -> Result<Self, DitherError> {
        if width == 0 || height == 0 {
            return Err(DitherError::ZeroDimension);
        }
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or_else(|| DitherError::InvalidParameter("buffer dimensions exceed memory capacity".into()))?;

        Ok(Self { width, height, data: vec![0; len] })
    }

    /// Construct an image buffer from existing RGB24 byte slice.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Result<Self, DitherError> {
        if width == 0 || height == 0 {
            return Err(DitherError::ZeroDimension);
        }
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|p| p.checked_mul(3))
            .ok_or_else(|| DitherError::InvalidParameter("buffer dimensions exceed capacity".into()))?;

        if data.len() != expected_len {
            return Err(DitherError::InvalidDimensions { width, height, expected_len, actual_len: data.len() });
        }

        Ok(Self { width, height, data })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Get pixel color with bounds checking.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<RgbColor> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y as usize * self.width as usize) + x as usize) * 3;
        let r = *self.data.get(idx)?;
        let g = *self.data.get(idx + 1)?;
        let b = *self.data.get(idx + 2)?;
        Some(RgbColor::new(r, g, b))
    }

    /// Clamped pixel fetch for convolution/diffusion passes.
    #[inline]
    pub fn get_pixel_clamped(&self, x: i32, y: i32) -> RgbColor {
        let cx = x.clamp(0, (self.width as i32) - 1) as usize;
        let cy = y.clamp(0, (self.height as i32) - 1) as usize;
        let idx = (cy * self.width as usize + cx) * 3;
        let r = self.data.get(idx).copied().unwrap_or(0);
        let g = self.data.get(idx + 1).copied().unwrap_or(0);
        let b = self.data.get(idx + 2).copied().unwrap_or(0);
        RgbColor::new(r, g, b)
    }

    /// Set pixel color at coordinate.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: RgbColor) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = ((y as usize * self.width as usize) + x as usize) * 3;
        if idx + 2 < self.data.len() {
            self.data[idx] = color.r;
            self.data[idx + 1] = color.g;
            self.data[idx + 2] = color.b;
            true
        } else {
            false
        }
    }

    /// Convert entire buffer to grayscale via ITU-R BT.601 luminance.
    pub fn to_grayscale(&self) -> Result<Self, DitherError> {
        let mut out = Self::new(self.width, self.height)?;
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(px) = self.get_pixel(x, y) {
                    let y_lum = px.luminance();
                    out.set_pixel(x, y, RgbColor::new(y_lum, y_lum, y_lum));
                }
            }
        }
        Ok(out)
    }

    /// Downsample the image to fit within `max_w` and `max_h` while preserving aspect ratio.
    ///
    /// Uses box-area averaging for downsampling, eliminating aliasing artifacts.
    /// If the image is already within bounds, returns a clone.
    pub fn downsample(&self, max_w: u32, max_h: u32) -> Result<Self, DitherError> {
        if max_w == 0 || max_h == 0 {
            return Err(DitherError::ZeroDimension);
        }
        if self.width <= max_w && self.height <= max_h {
            return Ok(self.clone());
        }

        let scale_x = max_w as f32 / self.width as f32;
        let scale_y = max_h as f32 / self.height as f32;
        let scale = scale_x.min(scale_y);

        let target_w = ((self.width as f32 * scale).round() as u32).max(1);
        let target_h = ((self.height as f32 * scale).round() as u32).max(1);

        let mut out = Self::new(target_w, target_h)?;

        let x_ratio = self.width as f64 / target_w as f64;
        let y_ratio = self.height as f64 / target_h as f64;

        for ty in 0..target_h {
            let sy_start = (ty as f64 * y_ratio) as usize;
            let sy_end = (((ty + 1) as f64 * y_ratio).ceil() as usize).min(self.height as usize).max(sy_start + 1);

            for tx in 0..target_w {
                let sx_start = (tx as f64 * x_ratio) as usize;
                let sx_end = (((tx + 1) as f64 * x_ratio).ceil() as usize).min(self.width as usize).max(sx_start + 1);

                let mut r_sum = 0u64;
                let mut g_sum = 0u64;
                let mut b_sum = 0u64;
                let mut count = 0u64;

                for sy in sy_start..sy_end {
                    let row_idx = sy * (self.width as usize) * 3;
                    for sx in sx_start..sx_end {
                        let idx = row_idx + sx * 3;
                        if idx + 2 < self.data.len() {
                            r_sum += self.data[idx] as u64;
                            g_sum += self.data[idx + 1] as u64;
                            b_sum += self.data[idx + 2] as u64;
                            count += 1;
                        }
                    }
                }

                if let Some(r_avg) = r_sum.checked_div(count)
                    && let Some(g_avg) = g_sum.checked_div(count)
                    && let Some(b_avg) = b_sum.checked_div(count)
                {
                    out.set_pixel(tx, ty, RgbColor::new(r_avg.min(255) as u8, g_avg.min(255) as u8, b_avg.min(255) as u8));
                }
            }
        }

        Ok(out)
    }
}
