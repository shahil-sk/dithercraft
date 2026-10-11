use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;
use std::f32::consts::TAU;

/// Helper to apply a screen threshold function across image rows in parallel.
fn apply_screen_fn<F>(input: &ImageBuffer, palette: &Palette, screen_fn: F) -> Result<ImageBuffer, DitherError>
where
    F: Fn(f32, f32) -> f32 + Sync + Send,
{
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }
    let mut out = ImageBuffer::new(input.width, input.height)?;
    let width = input.width as usize;

    out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
        let y_f = y as f32;
        let row_in = y * width * 3;

        for x in 0..width {
            let x_f = x as f32;
            let thresh = screen_fn(x_f, y_f); // in [0.0, 1.0]
            let bias = ((thresh - 0.5) * 255.0) as i32;

            let in_idx = row_in + x * 3;
            let r = (input.data[in_idx] as i32 + bias).clamp(0, 255) as u8;
            let g = (input.data[in_idx + 1] as i32 + bias).clamp(0, 255) as u8;
            let b = (input.data[in_idx + 2] as i32 + bias).clamp(0, 255) as u8;

            let q = palette.find_nearest(&RgbColor::new(r, g, b));
            let out_idx = x * 3;
            row_out[out_idx] = q.r;
            row_out[out_idx + 1] = q.g;
            row_out[out_idx + 2] = q.b;
        }
    });

    Ok(out)
}

/// Rotated Screen Halftone (configurable angle and frequency)
#[derive(Debug, Clone)]
pub struct RotatedScreen {
    pub name_str: &'static str,
    pub angle_deg: f32,
    pub frequency: f32,
}

impl RotatedScreen {
    pub const fn screen_0() -> Self {
        Self { name_str: "Rotated Screen 0°", angle_deg: 0.0, frequency: 0.125 }
    }
    pub const fn screen_15() -> Self {
        Self { name_str: "Rotated Screen 15°", angle_deg: 15.0, frequency: 0.125 }
    }
    pub const fn screen_45() -> Self {
        Self { name_str: "Rotated Screen 45°", angle_deg: 45.0, frequency: 0.125 }
    }
    pub const fn screen_75() -> Self {
        Self { name_str: "Rotated Screen 75°", angle_deg: 75.0, frequency: 0.125 }
    }
}

impl DitherAlgorithm for RotatedScreen {
    fn name(&self) -> &'static str {
        self.name_str
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let rad = self.angle_deg.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        let freq = self.frequency;

        apply_screen_fn(input, palette, move |x, y| {
            let u = x * cos_a - y * sin_a;
            let v = x * sin_a + y * cos_a;
            let s_u = (u * freq * TAU).cos();
            let s_v = (v * freq * TAU).cos();
            ((s_u + s_v) * 0.25 + 0.5).clamp(0.0, 1.0)
        })
    }
}

/// Circular Dot Halftone Screen
#[derive(Debug, Clone)]
pub struct CircularScreen {
    pub cell_size: f32,
}

impl Default for CircularScreen {
    fn default() -> Self {
        Self { cell_size: 8.0 }
    }
}

impl DitherAlgorithm for CircularScreen {
    fn name(&self) -> &'static str {
        "Circular Screen"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let s = self.cell_size.max(2.0);
        let half = s * 0.5;
        let max_r = half * std::f32::consts::SQRT_2;

        apply_screen_fn(input, palette, move |x, y| {
            let dx = (x % s) - half;
            let dy = (y % s) - half;
            let dist = (dx * dx + dy * dy).sqrt();
            (1.0 - (dist / max_r)).clamp(0.0, 1.0)
        })
    }
}

/// Diamond Dot Halftone Screen (Manhattan distance metric)
#[derive(Debug, Clone)]
pub struct DiamondDot {
    pub cell_size: f32,
}

impl Default for DiamondDot {
    fn default() -> Self {
        Self { cell_size: 8.0 }
    }
}

impl DitherAlgorithm for DiamondDot {
    fn name(&self) -> &'static str {
        "Diamond Dot"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let s = self.cell_size.max(2.0);
        let half = s * 0.5;

        apply_screen_fn(input, palette, move |x, y| {
            let dx = ((x % s) - half).abs();
            let dy = ((y % s) - half).abs();
            (1.0 - (dx + dy) / s).clamp(0.0, 1.0)
        })
    }
}

/// Elliptical Dot Halftone Screen (Anisotropic aspect ratio dot growth)
#[derive(Debug, Clone)]
pub struct EllipticalDot {
    pub cell_size: f32,
    pub aspect_ratio: f32,
    pub angle_deg: f32,
}

impl Default for EllipticalDot {
    fn default() -> Self {
        Self { cell_size: 8.0, aspect_ratio: 1.8, angle_deg: 45.0 }
    }
}

impl DitherAlgorithm for EllipticalDot {
    fn name(&self) -> &'static str {
        "Elliptical Dot"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let s = self.cell_size.max(2.0);
        let half = s * 0.5;
        let aspect = self.aspect_ratio.max(0.1);
        let rad = self.angle_deg.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        apply_screen_fn(input, palette, move |x, y| {
            let dx = (x % s) - half;
            let dy = (y % s) - half;
            let u = dx * cos_a - dy * sin_a;
            let v = dx * sin_a + dy * cos_a;
            let dist = ((u * u) + (v * v * aspect * aspect)).sqrt();
            (1.0 - (dist / s)).clamp(0.0, 1.0)
        })
    }
}

/// 1D Linear Raster Engraving Screen
#[derive(Debug, Clone)]
pub struct LineScreen {
    pub frequency: f32,
    pub angle_deg: f32,
}

impl Default for LineScreen {
    fn default() -> Self {
        Self { frequency: 0.125, angle_deg: 45.0 }
    }
}

impl DitherAlgorithm for LineScreen {
    fn name(&self) -> &'static str {
        "Line Screen"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let rad = self.angle_deg.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        let freq = self.frequency;

        apply_screen_fn(input, palette, move |x, y| {
            let u = x * cos_a - y * sin_a;
            let line = (u * freq * TAU).cos() * 0.5 + 0.5;
            line.clamp(0.0, 1.0)
        })
    }
}

/// Crosshatch Halftone Screen (Dual-axis perpendicular lines)
#[derive(Debug, Clone)]
pub struct CrosshatchHalftone {
    pub frequency: f32,
}

impl Default for CrosshatchHalftone {
    fn default() -> Self {
        Self { frequency: 0.125 }
    }
}

impl DitherAlgorithm for CrosshatchHalftone {
    fn name(&self) -> &'static str {
        "Crosshatch Halftone"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        let freq = self.frequency;
        let rad1 = 45.0f32.to_radians();
        let rad2 = 135.0f32.to_radians();
        let (c1, s1) = (rad1.cos(), rad1.sin());
        let (c2, s2) = (rad2.cos(), rad2.sin());

        apply_screen_fn(input, palette, move |x, y| {
            let u1 = x * c1 - y * s1;
            let u2 = x * c2 - y * s2;
            let l1 = (u1 * freq * TAU).cos() * 0.5 + 0.5;
            let l2 = (u2 * freq * TAU).cos() * 0.5 + 0.5;
            l1.max(l2).clamp(0.0, 1.0)
        })
    }
}
