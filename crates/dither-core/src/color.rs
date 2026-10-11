use serde::{Deserialize, Serialize};

/// 24-bit sRGB color with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Calculate perceived photometric luminance according to ITU-R BT.601:
    /// Y = 0.299 * R + 0.587 * G + 0.114 * B
    #[inline]
    pub fn luminance(&self) -> u8 {
        let y = (77 * self.r as u32 + 150 * self.g as u32 + 29 * self.b as u32) >> 8;
        y.min(255) as u8
    }

    /// Squared Euclidean distance in RGB color space.
    #[inline]
    pub fn distance_sq(&self, other: &Self) -> u32 {
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;
        (dr * dr + dg * dg + db * db) as u32
    }

    /// Perceptual color distance squared using the Redmean approximation.
    /// Accurately models human trichromatic vision with zero float allocations.
    #[inline]
    pub fn distance_redmean(&self, other: &Self) -> u32 {
        let r_bar = (self.r as i32 + other.r as i32) / 2;
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;

        let weight_r = 2 + (r_bar >> 8);
        let weight_g = 4;
        let weight_b = 2 + ((255 - r_bar) >> 8);

        (weight_r * dr * dr + weight_g * dg * dg + weight_b * db * db) as u32
    }

    /// Parse a 6-character hex string (e.g. "#FF00FF" or "FF00FF").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let s = hex.strip_prefix('#').unwrap_or(hex);
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Self::new(r, g, b))
    }

    /// Format as uppercase 6-character hex string (e.g. "#FF00FF").
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}
