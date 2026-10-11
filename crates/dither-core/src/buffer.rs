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
}
