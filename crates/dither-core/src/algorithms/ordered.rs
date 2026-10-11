use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

// Bayer 2x2: normalized / 4
pub static BAYER_2X2: [u8; 4] = [0, 2, 3, 1];

// Bayer 4x4: normalized / 16
pub static BAYER_4X4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

// Bayer 8x8: normalized / 64
pub static BAYER_8X8: [u8; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33,
    9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
];

// Clustered Dot 4x4 (halftone cluster)
pub static CLUSTERED_DOT_4X4: [u8; 16] = [12, 5, 6, 13, 4, 0, 1, 7, 11, 3, 2, 8, 15, 10, 9, 14];

// Clustered Dot 8x8
pub static CLUSTERED_DOT_8X8: [u8; 64] = [
    24, 10, 12, 26, 35, 47, 49, 37, 8, 0, 2, 14, 45, 59, 61, 51, 22, 6, 4, 16, 43, 57, 63, 53, 30, 20, 18, 28, 33, 41, 55, 39, 34, 46, 48, 36, 25,
    11, 13, 27, 44, 58, 60, 50, 9, 1, 3, 15, 42, 56, 62, 52, 23, 7, 5, 17, 32, 40, 54, 38, 31, 21, 19, 29,
];

// Horizontal lines 4x4
pub static HORIZONTAL_LINES_4X4: [u8; 16] = [0, 0, 0, 0, 8, 8, 8, 8, 4, 4, 4, 4, 12, 12, 12, 12];

// Vertical lines 4x4
pub static VERTICAL_LINES_4X4: [u8; 16] = [0, 8, 4, 12, 0, 8, 4, 12, 0, 8, 4, 12, 0, 8, 4, 12];

// Diagonal lines 4x4
pub static DIAGONAL_LINES_4X4: [u8; 16] = [0, 4, 8, 12, 4, 8, 12, 0, 8, 12, 0, 4, 12, 0, 4, 8];

// Checkerboard 4x4
pub static CHECKERBOARD_4X4: [u8; 16] = [0, 8, 0, 8, 8, 0, 8, 0, 0, 8, 0, 8, 8, 0, 8, 0];

/// Helper to execute ordered dithering over rows in parallel using Rayon.
pub fn apply_matrix(input: &ImageBuffer, palette: &Palette, matrix: &[u8], dim: usize, divisor: i32) -> Result<ImageBuffer, DitherError> {
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }
    let mut out = ImageBuffer::new(input.width, input.height)?;
    let width = input.width as usize;
    let half_div = divisor / 2;

    out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
        let row_in_start = y * width * 3;
        let mat_y = (y % dim) * dim;

        for x in 0..width {
            let in_idx = row_in_start + x * 3;
            let r = input.data[in_idx] as i32;
            let g = input.data[in_idx + 1] as i32;
            let b = input.data[in_idx + 2] as i32;

            let mat_val = matrix[mat_y + (x % dim)] as i32;
            // Shift range around zero: [-half_div, half_div] scaled to [-128, 128]
            let bias = ((mat_val - half_div) * 255) / divisor;

            let adjusted = RgbColor::new((r + bias).clamp(0, 255) as u8, (g + bias).clamp(0, 255) as u8, (b + bias).clamp(0, 255) as u8);

            let quantized = palette.find_nearest(&adjusted);
            let out_idx = x * 3;
            row_out[out_idx] = quantized.r;
            row_out[out_idx + 1] = quantized.g;
            row_out[out_idx + 2] = quantized.b;
        }
    });

    Ok(out)
}

/// Dynamic Bayer 16x16 generation from recursive bit interleaving (academic formula).
pub fn bayer_16x16_val(x: usize, y: usize) -> u8 {
    let mut val = 0u16;
    for bit in 0..4 {
        let x_bit = ((x >> bit) & 1) as u16;
        let y_bit = ((y >> bit) & 1) as u16;
        val |= (x_bit ^ y_bit) << (2 * (3 - bit) + 1);
        val |= y_bit << (2 * (3 - bit));
    }
    (val & 0xFF) as u8
}

// ---------------- Struct Implementations ----------------

#[derive(Default)]
pub struct Bayer2x2;
impl DitherAlgorithm for Bayer2x2 {
    fn name(&self) -> &'static str {
        "Bayer 2x2"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &BAYER_2X2, 2, 4)
    }
}

#[derive(Default)]
pub struct Bayer4x4;
impl DitherAlgorithm for Bayer4x4 {
    fn name(&self) -> &'static str {
        "Bayer 4x4"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &BAYER_4X4, 4, 16)
    }
}

#[derive(Default)]
pub struct Bayer8x8;
impl DitherAlgorithm for Bayer8x8 {
    fn name(&self) -> &'static str {
        "Bayer 8x8"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &BAYER_8X8, 8, 64)
    }
}

#[derive(Default)]
pub struct Bayer16x16;
impl DitherAlgorithm for Bayer16x16 {
    fn name(&self) -> &'static str {
        "Bayer 16x16"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let r = input.data[in_idx] as i32;
                let g = input.data[in_idx + 1] as i32;
                let b = input.data[in_idx + 2] as i32;

                let mat_val = bayer_16x16_val(x % 16, y % 16) as i32;
                let bias = ((mat_val - 128) * 255) / 256;

                let adjusted = RgbColor::new((r + bias).clamp(0, 255) as u8, (g + bias).clamp(0, 255) as u8, (b + bias).clamp(0, 255) as u8);
                let q = palette.find_nearest(&adjusted);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });
        Ok(out)
    }
}

#[derive(Default)]
pub struct ClusteredDot4x4;
impl DitherAlgorithm for ClusteredDot4x4 {
    fn name(&self) -> &'static str {
        "Clustered Dot 4x4"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &CLUSTERED_DOT_4X4, 4, 16)
    }
}

#[derive(Default)]
pub struct ClusteredDot8x8;
impl DitherAlgorithm for ClusteredDot8x8 {
    fn name(&self) -> &'static str {
        "Clustered Dot 8x8"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &CLUSTERED_DOT_8X8, 8, 64)
    }
}

#[derive(Default)]
pub struct HorizontalLines;
impl DitherAlgorithm for HorizontalLines {
    fn name(&self) -> &'static str {
        "Horizontal Lines"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &HORIZONTAL_LINES_4X4, 4, 16)
    }
}

#[derive(Default)]
pub struct VerticalLines;
impl DitherAlgorithm for VerticalLines {
    fn name(&self) -> &'static str {
        "Vertical Lines"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &VERTICAL_LINES_4X4, 4, 16)
    }
}

#[derive(Default)]
pub struct DiagonalLines;
impl DitherAlgorithm for DiagonalLines {
    fn name(&self) -> &'static str {
        "Diagonal Lines"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &DIAGONAL_LINES_4X4, 4, 16)
    }
}

#[derive(Default)]
pub struct Checkerboard45;
impl DitherAlgorithm for Checkerboard45 {
    fn name(&self) -> &'static str {
        "Checkerboard 45°"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        apply_matrix(input, palette, &CHECKERBOARD_4X4, 4, 16)
    }
}

/// Interleaved Gradient Noise (Jorge Jimenez, SIGGRAPH 2014)
#[derive(Default)]
pub struct InterleavedGradientNoise;
impl DitherAlgorithm for InterleavedGradientNoise {
    fn name(&self) -> &'static str {
        "Interleaved Gradient Noise"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in_start = y * width * 3;
            for x in 0..width {
                let in_idx = row_in_start + x * 3;
                let r = input.data[in_idx] as f32;
                let g = input.data[in_idx + 1] as f32;
                let b = input.data[in_idx + 2] as f32;

                // Jimenez IGN equation
                let x_f = x as f32;
                let y_f = y as f32;
                let inner = 0.06711056 * x_f + 0.00583715 * y_f;
                let fract1 = inner - inner.floor();
                let mult = 52.982_918 * fract1;
                let ign = mult - mult.floor(); // [0.0, 1.0)

                let bias = (ign - 0.5) * 64.0;
                let adjusted =
                    RgbColor::new((r + bias).clamp(0.0, 255.0) as u8, (g + bias).clamp(0.0, 255.0) as u8, (b + bias).clamp(0.0, 255.0) as u8);
                let q = palette.find_nearest(&adjusted);
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });
        Ok(out)
    }
}
