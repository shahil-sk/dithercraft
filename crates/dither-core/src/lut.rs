use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Industry-standard `.cube` 3D Color Lookup Table (LUT) engine.
/// Provides trilinear interpolation across an $N \times N \times N$ color lattice for cinematic
/// retro color grading, film stock emulation, and phosphor tinting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cube3dLut {
    pub title: String,
    pub size: usize,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub table: Vec<[f32; 3]>,
}

impl Cube3dLut {
    /// Construct a new 3D LUT with the specified dimension and table data.
    pub fn new(title: impl Into<String>, size: usize, table: Vec<[f32; 3]>) -> Result<Self, DitherError> {
        let expected = size * size * size;
        if !(2..=128).contains(&size) {
            return Err(DitherError::InvalidParameter(format!("LUT size {size} out of range [2, 128]")));
        }
        if table.len() != expected {
            return Err(DitherError::InvalidParameter(format!("LUT table length {} does not match expected {expected}", table.len())));
        }
        Ok(Self { title: title.into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table })
    }

    /// Parse an industry-standard `.cube` format string.
    pub fn from_cube_str(input: &str) -> Result<Self, DitherError> {
        let mut title = "Untitled LUT".to_string();
        let mut size = 0usize;
        let mut domain_min = [0.0f32, 0.0, 0.0];
        let mut domain_max = [1.0f32, 1.0, 1.0];
        let mut table = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("TITLE") {
                let trimmed = rest.trim().trim_matches('"');
                if !trimmed.is_empty() {
                    title = trimmed.to_string();
                }
            } else if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
                size = rest.trim().parse::<usize>().map_err(|e| DitherError::InvalidParameter(format!("invalid LUT_3D_SIZE: {e}")))?;
            } else if let Some(rest) = line.strip_prefix("DOMAIN_MIN") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() == 3
                    && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse(), parts[1].parse(), parts[2].parse())
                {
                    domain_min = [r, g, b];
                }
            } else if let Some(rest) = line.strip_prefix("DOMAIN_MAX") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() == 3
                    && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse(), parts[1].parse(), parts[2].parse())
                {
                    domain_max = [r, g, b];
                }
            } else {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 3
                    && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>(), parts[2].parse::<f32>())
                {
                    table.push([r, g, b]);
                }
            }
        }

        if !(2..=128).contains(&size) {
            return Err(DitherError::InvalidParameter(format!("LUT size {size} out of range [2, 128]")));
        }
        if table.len() != size * size * size {
            return Err(DitherError::InvalidParameter(format!("LUT entries {} does not match expected {}", table.len(), size * size * size)));
        }

        Ok(Self { title, size, domain_min, domain_max, table })
    }

    /// Apply the 3D LUT to a single 24-bit RGB pixel using zero-allocation trilinear interpolation.
    #[inline]
    pub fn apply_pixel(&self, r: u8, g: u8, b: u8) -> RgbColor {
        let n = self.size;
        let n_f = (n - 1) as f32;

        let u = (r as f32 / 255.0) * n_f;
        let v = (g as f32 / 255.0) * n_f;
        let w = (b as f32 / 255.0) * n_f;

        let x0 = (u.floor() as usize).min(n - 1);
        let x1 = (x0 + 1).min(n - 1);
        let fx = u - x0 as f32;

        let y0 = (v.floor() as usize).min(n - 1);
        let y1 = (y0 + 1).min(n - 1);
        let fy = v - y0 as f32;

        let z0 = (w.floor() as usize).min(n - 1);
        let z1 = (z0 + 1).min(n - 1);
        let fz = w - z0 as f32;

        let idx = |rx: usize, gx: usize, bx: usize| -> usize { rx + gx * n + bx * n * n };

        let c000 = self.table[idx(x0, y0, z0)];
        let c100 = self.table[idx(x1, y0, z0)];
        let c010 = self.table[idx(x0, y1, z0)];
        let c110 = self.table[idx(x1, y1, z0)];
        let c001 = self.table[idx(x0, y0, z1)];
        let c101 = self.table[idx(x1, y0, z1)];
        let c011 = self.table[idx(x0, y1, z1)];
        let c111 = self.table[idx(x1, y1, z1)];

        let c00_r = c000[0] * (1.0 - fx) + c100[0] * fx;
        let c00_g = c000[1] * (1.0 - fx) + c100[1] * fx;
        let c00_b = c000[2] * (1.0 - fx) + c100[2] * fx;

        let c10_r = c010[0] * (1.0 - fx) + c110[0] * fx;
        let c10_g = c010[1] * (1.0 - fx) + c110[1] * fx;
        let c10_b = c010[2] * (1.0 - fx) + c110[2] * fx;

        let c01_r = c001[0] * (1.0 - fx) + c101[0] * fx;
        let c01_g = c001[1] * (1.0 - fx) + c101[1] * fx;
        let c01_b = c001[2] * (1.0 - fx) + c101[2] * fx;

        let c11_r = c011[0] * (1.0 - fx) + c111[0] * fx;
        let c11_g = c011[1] * (1.0 - fx) + c111[1] * fx;
        let c11_b = c011[2] * (1.0 - fx) + c111[2] * fx;

        let c0_r = c00_r * (1.0 - fy) + c10_r * fy;
        let c0_g = c00_g * (1.0 - fy) + c10_g * fy;
        let c0_b = c00_b * (1.0 - fy) + c10_b * fy;

        let c1_r = c01_r * (1.0 - fy) + c11_r * fy;
        let c1_g = c01_g * (1.0 - fy) + c11_g * fy;
        let c1_b = c01_b * (1.0 - fy) + c11_b * fy;

        let out_r = (c0_r * (1.0 - fz) + c1_r * fz).clamp(0.0, 1.0);
        let out_g = (c0_g * (1.0 - fz) + c1_g * fz).clamp(0.0, 1.0);
        let out_b = (c0_b * (1.0 - fz) + c1_b * fz).clamp(0.0, 1.0);

        RgbColor::new(
            (out_r * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
            (out_g * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
            (out_b * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
        )
    }

    /// Apply the 3D LUT in-place across an entire image buffer in parallel with Rayon.
    pub fn apply_buffer(&self, buffer: &mut ImageBuffer) {
        buffer.data.par_chunks_exact_mut(3).for_each(|chunk| {
            let res = self.apply_pixel(chunk[0], chunk[1], chunk[2]);
            chunk[0] = res.r;
            chunk[1] = res.g;
            chunk[2] = res.b;
        });
    }

    /// Construct an identity 3D LUT (passes all color coordinates through unchanged).
    pub fn identity(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;
                    table.push([rf, gf, bf]);
                }
            }
        }

        Self { title: "Identity".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }

    /// Technicolor 2-Strip emulation (1920s Kinemacolor/Technicolor red-cyan color gamut).
    pub fn technicolor_2strip(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;

                    // Red record and Cyan record blending
                    let red_out = (rf * 1.15 + gf * 0.1 - bf * 0.1).clamp(0.0, 1.0);
                    let green_out = (gf * 0.85 + bf * 0.25).clamp(0.0, 1.0);
                    let blue_out = (gf * 0.25 + bf * 0.60).clamp(0.0, 1.0);

                    table.push([red_out, green_out, blue_out]);
                }
            }
        }

        Self { title: "Technicolor 2-Strip".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }

    /// Vintage Warm 1970s print emulation (warm sepia highlights, lifted shadows, retro saturation roll-off).
    pub fn vintage_warm_1970(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;

                    let red_out = (rf * 0.95 + 0.06).powf(0.92).clamp(0.0, 1.0);
                    let green_out = (gf * 0.92 + 0.04).powf(0.96).clamp(0.0, 1.0);
                    let blue_out = (bf * 0.80 + 0.08).powf(1.10).clamp(0.0, 1.0);

                    table.push([red_out, green_out, blue_out]);
                }
            }
        }

        Self { title: "Vintage Warm 1970".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }

    /// Amber Phosphor CRT grade (monochrome amber terminal glow with rich contrast).
    pub fn amber_phosphor(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;

                    // Luminance calculation
                    let y = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
                    let amber_r = (y * 1.05).clamp(0.0, 1.0);
                    let amber_g = (y * 0.69).clamp(0.0, 1.0);
                    let amber_b = (y * 0.05).clamp(0.0, 1.0);

                    table.push([amber_r, amber_g, amber_b]);
                }
            }
        }

        Self { title: "Amber Phosphor".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }

    /// Cyber Matrix grade (acid green glow with crushed blacks and neon green high-range).
    pub fn cyber_matrix(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;

                    let y = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
                    let r_out = (rf * 0.15 + y * 0.05).clamp(0.0, 1.0);
                    let g_out = (gf * 0.70 + y * 0.45).clamp(0.0, 1.0);
                    let b_out = (bf * 0.20 + y * 0.10).clamp(0.0, 1.0);

                    table.push([r_out, g_out, b_out]);
                }
            }
        }

        Self { title: "Cyber Matrix".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }

    /// Bleach Bypass (Silver Retention) photographic effect: harsh contrast, desaturated hues.
    pub fn bleach_bypass(size: usize) -> Self {
        let size = size.clamp(2, 64);
        let total = size * size * size;
        let mut table = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;

                    let y = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
                    // Overlay blend of luminance onto color
                    let overlay = |base: f32, top: f32| -> f32 { if base < 0.5 { 2.0 * base * top } else { 1.0 - 2.0 * (1.0 - base) * (1.0 - top) } };

                    let r_out = (overlay(rf, y) * 0.65 + rf * 0.35).clamp(0.0, 1.0);
                    let g_out = (overlay(gf, y) * 0.65 + gf * 0.35).clamp(0.0, 1.0);
                    let b_out = (overlay(bf, y) * 0.65 + bf * 0.35).clamp(0.0, 1.0);

                    table.push([r_out, g_out, b_out]);
                }
            }
        }

        Self { title: "Bleach Bypass".into(), size, domain_min: [0.0, 0.0, 0.0], domain_max: [1.0, 1.0, 1.0], table }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_3d_lut_identity() {
        let lut = Cube3dLut::identity(16);
        let original = RgbColor::new(128, 64, 200);
        let processed = lut.apply_pixel(original.r, original.g, original.b);
        assert!((original.r as i32 - processed.r as i32).abs() <= 1);
        assert!((original.g as i32 - processed.g as i32).abs() <= 1);
        assert!((original.b as i32 - processed.b as i32).abs() <= 1);
    }

    #[test]
    fn test_cube_3d_lut_parsing() {
        let cube_sample = r#"
# Sample 3D LUT
TITLE "Test 2x2x2"
LUT_3D_SIZE 2
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 1.0 1.0 1.0

0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;
        let lut = Cube3dLut::from_cube_str(cube_sample).expect("parse valid .cube");
        assert_eq!(lut.title, "Test 2x2x2");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.table.len(), 8);

        let mid = lut.apply_pixel(128, 128, 128);
        assert!((mid.r as i32 - 128).abs() <= 2);
    }

    #[test]
    fn test_cube_3d_lut_presets() {
        let p1 = Cube3dLut::technicolor_2strip(8);
        let p2 = Cube3dLut::vintage_warm_1970(8);
        let p3 = Cube3dLut::amber_phosphor(8);
        let p4 = Cube3dLut::cyber_matrix(8);
        let p5 = Cube3dLut::bleach_bypass(8);

        let mut buf = ImageBuffer::new(16, 16).unwrap();
        buf.set_pixel(0, 0, RgbColor::new(200, 100, 50));

        p1.apply_buffer(&mut buf);
        p2.apply_buffer(&mut buf);
        p3.apply_buffer(&mut buf);
        p4.apply_buffer(&mut buf);
        p5.apply_buffer(&mut buf);

        assert_eq!(buf.width, 16);
    }
}
