use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;

/// 16x16 Canonical Void-and-Cluster Blue Noise Dither Array (Robert Ulichney, SPIE 1993)
pub static VOID_AND_CLUSTER_16X16: [u8; 256] = [
    34, 245, 81, 192, 23, 235, 71, 182, 30, 241, 77, 188, 19, 231, 67, 178, 137, 11, 163, 114, 147, 15, 173, 104, 133, 7, 159, 110, 143, 1, 169, 100,
    218, 57, 203, 43, 228, 53, 199, 39, 214, 59, 207, 47, 224, 49, 195, 35, 96, 177, 126, 3, 86, 167, 116, 253, 92, 175, 122, 255, 82, 163, 120, 249,
    27, 239, 75, 186, 32, 243, 79, 190, 21, 233, 69, 180, 36, 247, 73, 184, 141, 5, 167, 108, 135, 9, 161, 112, 145, 13, 171, 102, 139, 3, 165, 106,
    222, 51, 197, 37, 216, 61, 205, 45, 226, 55, 201, 41, 220, 63, 209, 33, 90, 171, 118, 251, 94, 179, 124, 5, 84, 165, 114, 253, 88, 173, 128, 7,
    29, 237, 65, 176, 25, 247, 83, 194, 31, 239, 75, 186, 17, 229, 63, 174, 131, 17, 155, 102, 149, 7, 157, 116, 137, 11, 163, 108, 141, 19, 153, 98,
    212, 65, 211, 49, 224, 61, 201, 41, 218, 57, 203, 45, 228, 67, 215, 51, 88, 169, 128, 249, 98, 181, 122, 1, 90, 171, 126, 247, 94, 185, 118, 245,
    35, 243, 79, 190, 19, 231, 73, 184, 27, 235, 81, 192, 23, 241, 77, 188, 145, 13, 171, 106, 139, 15, 169, 100, 143, 9, 159, 112, 135, 5, 167, 104,
    226, 55, 205, 43, 216, 53, 197, 37, 222, 59, 207, 47, 214, 65, 209, 39, 84, 161, 114, 255, 92, 177, 120, 251, 86, 163, 116, 253, 82, 179, 124,
    251,
];

/// Void-and-Cluster Blue Noise Masking (Robert Ulichney 1993)
#[derive(Default)]
pub struct VoidAndCluster;

impl DitherAlgorithm for VoidAndCluster {
    fn name(&self) -> &'static str {
        "Void-and-Cluster"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let mat_y = (y % 16) * 16;

            for x in 0..width {
                let mask_val = VOID_AND_CLUSTER_16X16[mat_y + (x % 16)] as i32;
                let bias = ((mask_val - 128) * 255) / 256;

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
}

/// Frequency Modulation (Blue Noise FM / Martin Roberts R2 Low-Discrepancy 2014)
#[derive(Default)]
pub struct BlueNoiseFM;

impl DitherAlgorithm for BlueNoiseFM {
    fn name(&self) -> &'static str {
        "Frequency Modulation (Blue Noise)"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        // Plastic constant quasi-random 2D generators
        const A1: f64 = 0.7548776662466927;
        const A2: f64 = 0.5698402909980532;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let y_f = y as f64;

            for x in 0..width {
                let x_f = x as f64;
                let val = (0.5 + A1 * x_f + A2 * y_f).fract();
                let bias = ((val - 0.5) * 64.0) as i32;

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
}

/// Amplitude Modulation (AM / Clustered Screen Arrays)
#[derive(Default)]
pub struct AmplitudeModulation;

impl DitherAlgorithm for AmplitudeModulation {
    fn name(&self) -> &'static str {
        "Amplitude Modulation"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let cy = (y % 8) as f32 - 3.5;

            for x in 0..width {
                let cx = (x % 8) as f32 - 3.5;
                let dist = (cx * cx + cy * cy).sqrt() / 5.0; // [0.0, 1.0]
                let bias = ((0.5 - dist) * 72.0) as i32;

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
}

/// Stochastic Screening (Poisson-disk randomized screening)
#[derive(Default)]
pub struct StochasticScreening;

impl DitherAlgorithm for StochasticScreening {
    fn name(&self) -> &'static str {
        "Stochastic Screening"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;

            for x in 0..width {
                // Golden ratio hash with spatial offset
                let h1 = ((x as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (y as u64).wrapping_mul(0xBF58476D1CE4E5B9)) >> 32;
                let norm = (h1 & 0xFF) as i32;
                let bias = ((norm - 128) * 48) / 128;

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
}

/// Uniform Random Noise Dithering
#[derive(Default)]
pub struct UniformRandomNoise;

impl DitherAlgorithm for UniformRandomNoise {
    fn name(&self) -> &'static str {
        "Uniform Random Noise"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let mut state = ((y as u64) << 32) | (0x85EBCA6B);

            for x in 0..width {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(x as u64 + 1);
                let noise = ((state >> 48) as i32 % 65) - 32; // in [-32, 32]

                let in_idx = row_in + x * 3;
                let r = (input.data[in_idx] as i32 + noise).clamp(0, 255) as u8;
                let g = (input.data[in_idx + 1] as i32 + noise).clamp(0, 255) as u8;
                let b = (input.data[in_idx + 2] as i32 + noise).clamp(0, 255) as u8;

                let q = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Triangular Probability Density Function (HP-TPDF Dithering / Vanderkooy & Lipshitz 1989)
#[derive(Default)]
pub struct TriangularNoise;

impl DitherAlgorithm for TriangularNoise {
    fn name(&self) -> &'static str {
        "Triangular Noise (TPDF)"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let mut s1 = ((y as u64) << 32) | 0xA0761D6478BD642F;
            let mut s2 = ((y as u64) << 16) | 0xE7037ED1A0B428DB;

            for x in 0..width {
                s1 = s1.wrapping_mul(6364136223846793005).wrapping_add(x as u64 + 1);
                s2 = s2.wrapping_mul(1442695040888963407).wrapping_add(x as u64 + 2);

                let u1 = ((s1 >> 48) as i32 % 33) - 16; // [-16, 16]
                let u2 = ((s2 >> 48) as i32 % 33) - 16; // [-16, 16]
                let tpdf = u1 + u2; // Triangular distribution in [-32, 32]

                let in_idx = row_in + x * 3;
                let r = (input.data[in_idx] as i32 + tpdf).clamp(0, 255) as u8;
                let g = (input.data[in_idx + 1] as i32 + tpdf).clamp(0, 255) as u8;
                let b = (input.data[in_idx + 2] as i32 + tpdf).clamp(0, 255) as u8;

                let q = palette.find_nearest(&RgbColor::new(r, g, b));
                let out_idx = x * 3;
                row_out[out_idx] = q.r;
                row_out[out_idx + 1] = q.g;
                row_out[out_idx + 2] = q.b;
            }
        });

        Ok(out)
    }
}

/// Procedural Blue Noise Generator using Robert Ulichney's Void and Cluster method (1993).
/// Synthesizes an isotropic high-frequency threshold matrix of size `N x N` on a toroidal grid.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProceduralBlueNoise;

impl ProceduralBlueNoise {
    /// Generate an `N x N` blue noise threshold matrix scaled to 0..=255.
    /// `size` will be clamped between 4 and 64 for rapid interactive generation.
    pub fn generate_matrix(size: usize) -> Vec<u8> {
        let n = size.clamp(4, 64);
        let total = n * n;
        let mut matrix = vec![0u8; total];

        let sigma = 1.5_f32;
        let two_sigma_sq = 2.0 * sigma * sigma;

        let mut binary_pattern = vec![false; total];
        let mut ranks = vec![0usize; total];

        let toroidal_dist_sq = |x1: usize, y1: usize, x2: usize, y2: usize, n: usize| -> f32 {
            let dx = (x1 as isize - x2 as isize).abs();
            let dy = (y1 as isize - y2 as isize).abs();
            let dx_toroidal = dx.min(n as isize - dx) as f32;
            let dy_toroidal = dy.min(n as isize - dy) as f32;
            dx_toroidal * dx_toroidal + dy_toroidal * dy_toroidal
        };

        binary_pattern[0] = true;
        ranks[0] = 0;

        let mut energy = vec![0.0_f32; total];
        for (idx, e) in energy.iter_mut().enumerate() {
            let x = idx % n;
            let y = idx / n;
            let d2 = toroidal_dist_sq(x, y, 0, 0, n);
            *e = (-d2 / two_sigma_sq).exp();
        }

        for rank in 1..total {
            let mut min_energy = f32::MAX;
            let mut best_idx = 0;
            for (idx, &is_set) in binary_pattern.iter().enumerate() {
                if !is_set && energy[idx] < min_energy {
                    min_energy = energy[idx];
                    best_idx = idx;
                }
            }

            binary_pattern[best_idx] = true;
            ranks[best_idx] = rank;

            let bx = best_idx % n;
            let by = best_idx / n;
            for (idx, e) in energy.iter_mut().enumerate() {
                let x = idx % n;
                let y = idx / n;
                let d2 = toroidal_dist_sq(x, y, bx, by, n);
                *e += (-d2 / two_sigma_sq).exp();
            }
        }

        for (i, &r) in ranks.iter().enumerate() {
            matrix[i] = ((r * 255) / (total - 1).max(1)) as u8;
        }

        matrix
    }
}

impl DitherAlgorithm for ProceduralBlueNoise {
    fn name(&self) -> &'static str {
        "Procedural Blue Noise"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let matrix = Self::generate_matrix(16);
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let mat_y = (y % 16) * 16;

            for x in 0..width {
                let mask_val = matrix[mat_y + (x % 16)] as i32;
                let bias = ((mask_val - 128) * 255) / 256;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_blue_noise_matrix_generation() {
        let mat = ProceduralBlueNoise::generate_matrix(8);
        assert_eq!(mat.len(), 64);
        assert_eq!(mat.iter().min(), Some(&0));
        assert_eq!(mat.iter().max(), Some(&255));
    }
}
