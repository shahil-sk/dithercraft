use crate::algorithms::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use serde::{Deserialize, Serialize};

/// Custom N x M threshold matrix dithering algorithm.
///
/// Enables artists and fabrication workflows to design, load, and test custom
/// spatial ordered matrices (e.g., custom crosshatchings, typographic stamps,
/// micro-textures, and Bayer variants up to 16x16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomMatrixDither {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub matrix: Vec<u8>,
}

impl Default for CustomMatrixDither {
    fn default() -> Self {
        // Default to a 4x4 balanced spiral cluster matrix
        Self {
            name: "Custom Spiral 4x4".into(),
            width: 4,
            height: 4,
            matrix: vec![16, 32, 48, 64, 128, 144, 160, 80, 112, 240, 176, 96, 224, 208, 192, 128],
        }
    }
}

impl CustomMatrixDither {
    /// Create a new custom threshold matrix with explicit dimensions and thresholds in 0..=255.
    pub fn new(name: impl Into<String>, width: usize, height: usize, matrix: Vec<u8>) -> Result<Self, DitherError> {
        if !(2..=16).contains(&width) || !(2..=16).contains(&height) {
            return Err(DitherError::InvalidParameter(format!("Matrix dimensions {width}x{height} out of bounds (allowed: 2..=16)")));
        }
        if matrix.len() != width * height {
            return Err(DitherError::InvalidParameter(format!(
                "Matrix length {} does not match dimensions {width}x{height} (expected {})",
                matrix.len(),
                width * height
            )));
        }
        Ok(Self { name: name.into(), width, height, matrix })
    }

    /// Construct a matrix from floating-point values in [0.0, 1.0].
    pub fn from_normalized(name: impl Into<String>, width: usize, height: usize, floats: &[f32]) -> Result<Self, DitherError> {
        let u8s: Vec<u8> = floats.iter().map(|&f| (f.clamp(0.0, 1.0) * 255.0).round() as u8).collect();
        Self::new(name, width, height, u8s)
    }

    /// Serialize the custom matrix configuration to standard JSON string.
    pub fn to_json(&self) -> Result<String, DitherError> {
        serde_json::to_string_pretty(self).map_err(|e| DitherError::PresetLoadFailed(format!("Failed to serialize matrix: {e}")))
    }

    /// Parse and instantiate a custom matrix from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, DitherError> {
        let matrix: Self = serde_json::from_str(json).map_err(|e| DitherError::PresetLoadFailed(format!("Failed to parse matrix JSON: {e}")))?;
        Self::new(matrix.name, matrix.width, matrix.height, matrix.matrix)
    }

    /// Get threshold value at coordinate (x, y) wrapped by matrix dimensions.
    #[inline(always)]
    pub fn get_threshold(&self, x: u32, y: u32) -> u8 {
        let col = (x as usize) % self.width;
        let row = (y as usize) % self.height;
        self.matrix[row * self.width + col]
    }
}

impl DitherAlgorithm for CustomMatrixDither {
    fn name(&self) -> &'static str {
        "Custom Threshold Matrix"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        let mut output = ImageBuffer::new(input.width, input.height)?;
        let w = self.width;
        let h = self.height;

        for y in 0..input.height {
            let row_offset = ((y as usize) % h) * w;
            for x in 0..input.width {
                let col = (x as usize) % w;
                let thresh = self.matrix[row_offset + col];

                let px = input.get_pixel(x, y).unwrap_or(RgbColor::BLACK);

                // Offset pixel channels by threshold offset (-128..+127 scaled)
                let delta = (thresh as i16) - 128;
                let r = (px.r as i16 + delta / 2).clamp(0, 255) as u8;
                let g = (px.g as i16 + delta / 2).clamp(0, 255) as u8;
                let b = (px.b as i16 + delta / 2).clamp(0, 255) as u8;

                let adjusted = RgbColor::new(r, g, b);
                let matched = palette.find_nearest(&adjusted);
                output.set_pixel(x, y, matched);
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_matrix_creation_and_execution() {
        let matrix = CustomMatrixDither::default();
        assert_eq!(matrix.width, 4);
        assert_eq!(matrix.height, 4);
        assert_eq!(matrix.matrix.len(), 16);

        let mut buf = ImageBuffer::new(8, 8).expect("buf");
        for y in 0..8 {
            for x in 0..8 {
                buf.set_pixel(x, y, RgbColor::new((x * 30) as u8, (y * 30) as u8, 128));
            }
        }
        let pal = Palette::monochrome();
        let out = matrix.apply(&buf, &pal).expect("dither");
        assert_eq!(out.width, 8);
        assert_eq!(out.height, 8);

        // JSON roundtrip
        let json = matrix.to_json().expect("to json");
        let parsed = CustomMatrixDither::from_json(&json).expect("from json");
        assert_eq!(parsed.name, matrix.name);
        assert_eq!(parsed.matrix, matrix.matrix);
    }

    #[test]
    fn test_custom_matrix_dimension_validation() {
        assert!(CustomMatrixDither::new("invalid", 1, 4, vec![0; 4]).is_err());
        assert!(CustomMatrixDither::new("invalid", 20, 20, vec![0; 400]).is_err());
        assert!(CustomMatrixDither::new("mismatch", 2, 2, vec![0; 3]).is_err());
    }
}
