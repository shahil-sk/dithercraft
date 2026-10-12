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

    /// Adjust tone levels and midtone gamma in-place using a 256-element lookup table.
    ///
    /// - `black_point`: Input values below this become 0.
    /// - `white_point`: Input values above this become 255.
    /// - `gamma`: Non-linear power curve exponent (0.1 to 5.0, where 1.0 is linear).
    pub fn adjust_levels(&mut self, black_point: u8, white_point: u8, gamma: f32) {
        let bp = black_point as f32;
        let wp = (white_point as f32).max(bp + 1.0);
        let range = wp - bp;
        let inv_gamma = 1.0 / gamma.clamp(0.1, 5.0);

        let mut lut = [0u8; 256];
        for (i, entry) in lut.iter_mut().enumerate() {
            let norm = ((i as f32 - bp) / range).clamp(0.0, 1.0);
            let adjusted = norm.powf(inv_gamma) * 255.0;
            *entry = adjusted.clamp(0.0, 255.0).round() as u8;
        }

        for b in &mut self.data {
            *b = lut[*b as usize];
        }
    }

    /// Apply Color Vision Deficiency simulation in-place across the image buffer.
    pub fn apply_cvd_simulation(&mut self, mode: crate::color::CvdMode) {
        if mode == crate::color::CvdMode::Normal {
            return;
        }
        for chunk in self.data.chunks_exact_mut(3) {
            let px = RgbColor::new(chunk[0], chunk[1], chunk[2]);
            let sim = px.simulate_cvd(mode);
            chunk[0] = sim.r;
            chunk[1] = sim.g;
            chunk[2] = sim.b;
        }
    }
}

use std::sync::Mutex;

/// Bounded memory arena pool for zero-allocation recycling of ImageBuffers.
#[derive(Debug, Default)]
pub struct ImageBufferPool {
    pool: Mutex<Vec<ImageBuffer>>,
}

impl ImageBufferPool {
    /// Create a new empty pool.
    pub fn new() -> Self {
        Self { pool: Mutex::new(Vec::new()) }
    }

    /// Acquire a buffer with width and height from the pool, or allocate a new one.
    pub fn acquire(&self, width: u32, height: u32) -> Result<ImageBuffer, DitherError> {
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|p| p.checked_mul(3))
            .ok_or_else(|| DitherError::InvalidParameter("capacity overflow".into()))?;

        if let Ok(mut lock) = self.pool.lock()
            && let Some(pos) = lock.iter().position(|b| b.width == width && b.height == height && b.data.len() == expected_len)
        {
            let mut buf = lock.swap_remove(pos);
            buf.data.fill(0);
            return Ok(buf);
        }
        ImageBuffer::new(width, height)
    }

    /// Release a buffer back into the pool.
    pub fn release(&self, buffer: ImageBuffer) {
        if let Ok(mut lock) = self.pool.lock()
            && lock.len() < 16
        {
            lock.push(buffer);
        }
    }
}

/// Bidirectional 2D Hilbert fractal space-filling curve generator.
#[derive(Debug, Clone, Copy, Default)]
pub struct HilbertCurve;

impl HilbertCurve {
    /// Convert 1D distance step `d` to 2D coordinates `(x, y)` on an `n x n` grid (where `n` is a power of 2).
    pub fn d2xy(n: usize, mut d: usize) -> (usize, usize) {
        let mut x = 0;
        let mut y = 0;
        let mut s = 1;
        while s < n {
            let rx = 1 & (d / 2);
            let ry = 1 & (d ^ rx);
            if ry == 0 {
                if rx == 1 {
                    x = s - 1 - x;
                    y = s - 1 - y;
                }
                std::mem::swap(&mut x, &mut y);
            }
            x += s * rx;
            y += s * ry;
            d /= 4;
            s *= 2;
        }
        (x, y)
    }

    /// Inverse mapping: convert 2D coordinates `(x, y)` to 1D distance step `d` on an `n x n` grid.
    pub fn xy2d(n: usize, mut x: usize, mut y: usize) -> usize {
        let mut d = 0;
        let mut s = n / 2;
        while s > 0 {
            let rx = usize::from((x & s) > 0);
            let ry = usize::from((y & s) > 0);
            d += s * s * ((3 * rx) ^ ry);
            if ry == 0 {
                if rx == 1 {
                    x = n - 1 - x;
                    y = n - 1 - y;
                }
                std::mem::swap(&mut x, &mut y);
            }
            s /= 2;
        }
        d
    }

    /// Generate an ordered sequence of 2D coordinates visiting an image of size `(width, height)`.
    pub fn generate_order(width: usize, height: usize) -> Vec<(usize, usize)> {
        let max_dim = width.max(height);
        let mut n = 1;
        while n < max_dim {
            n *= 2;
        }
        let total_steps = n * n;
        let mut path = Vec::with_capacity(width * height);
        for d in 0..total_steps {
            let (x, y) = Self::d2xy(n, d);
            if x < width && y < height {
                path.push((x, y));
            }
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_levels() {
        let mut buf = ImageBuffer::new(2, 1).expect("buffer");
        buf.set_pixel(0, 0, RgbColor::new(50, 100, 150));
        buf.set_pixel(1, 0, RgbColor::new(200, 250, 255));

        buf.adjust_levels(50, 200, 1.0);
        let px0 = buf.get_pixel(0, 0).expect("px0");
        assert_eq!(px0.r, 0); // 50 mapped to black point 0
        let px1 = buf.get_pixel(1, 0).expect("px1");
        assert_eq!(px1.r, 255); // 200 mapped to white point 255
    }

    #[test]
    fn test_hilbert_curve_bijection() {
        let n = 16;
        for d in 0..(n * n) {
            let (x, y) = HilbertCurve::d2xy(n, d);
            assert!(x < n && y < n);
            let d_back = HilbertCurve::xy2d(n, x, y);
            assert_eq!(d, d_back);
        }

        let order = HilbertCurve::generate_order(10, 8);
        assert_eq!(order.len(), 80);
    }

    #[test]
    fn test_image_buffer_pool_recycling() {
        let pool = ImageBufferPool::new();
        let buf1 = pool.acquire(64, 64).expect("buf1");
        assert_eq!(buf1.width, 64);
        assert_eq!(buf1.height, 64);

        pool.release(buf1);
        let buf2 = pool.acquire(64, 64).expect("buf2 recycled");
        assert_eq!(buf2.width, 64);
        assert_eq!(buf2.height, 64);
    }
}
