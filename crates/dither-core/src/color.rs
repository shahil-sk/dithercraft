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

    /// Convert sRGB to CIE L*a*b* under D65 standard illuminant.
    pub fn to_lab(&self) -> LabColor {
        // sRGB to linear RGB
        let to_linear = |c: u8| -> f32 {
            let v = c as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        let r_lin = to_linear(self.r);
        let g_lin = to_linear(self.g);
        let b_lin = to_linear(self.b);

        // Linear RGB to CIE XYZ (D65)
        let x = r_lin * 0.4124564 + g_lin * 0.3575761 + b_lin * 0.1804375;
        let y = r_lin * 0.2126729 + g_lin * 0.7151522 + b_lin * 0.0721750;
        let z = r_lin * 0.0193339 + g_lin * 0.119_192 + b_lin * 0.9503041;

        // Normalize by D65 reference white (Xn=0.95047, Yn=1.0, Zn=1.08883)
        let f = |t: f32| -> f32 {
            const DELTA: f32 = 6.0 / 29.0;
            const DELTA3: f32 = DELTA * DELTA * DELTA;
            if t > DELTA3 { t.cbrt() } else { t / (3.0 * DELTA * DELTA) + (4.0 / 29.0) }
        };

        let fx = f(x / 0.95047);
        let fy = f(y / 1.0);
        let fz = f(z / 1.08883);

        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);

        LabColor { l, a, b }
    }

    /// CIE 1976 ΔE*ab perceptual color difference.
    pub fn distance_cielab_de(&self, other: &Self) -> f32 {
        let lab1 = self.to_lab();
        let lab2 = other.to_lab();
        let dl = lab1.l - lab2.l;
        let da = lab1.a - lab2.a;
        let db = lab1.b - lab2.b;
        (dl * dl + da * da + db * db).sqrt()
    }

    /// Convert sRGB 8-bit channels to linear RGB [0.0, 1.0].
    #[inline]
    pub fn to_linear_f32(&self) -> (f32, f32, f32) {
        let to_lin = |c: u8| -> f32 {
            let v = c as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        (to_lin(self.r), to_lin(self.g), to_lin(self.b))
    }

    /// Convert linear RGB [0.0, 1.0] to gamma-compressed sRGB 8-bit.
    #[inline]
    pub fn from_linear_f32(r: f32, g: f32, b: f32) -> Self {
        let to_srgb = |v: f32| -> u8 {
            let clamped = v.clamp(0.0, 1.0);
            let s = if clamped <= 0.0031308 { clamped * 12.92 } else { 1.055 * clamped.powf(1.0 / 2.4) - 0.055 };
            (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
        };
        Self::new(to_srgb(r), to_srgb(g), to_srgb(b))
    }

    /// Convert sRGB to Björn Ottosson's OKLab color space.
    #[allow(clippy::excessive_precision)]
    pub fn to_oklab(&self) -> OklabColor {
        let (r_lin, g_lin, b_lin) = self.to_linear_f32();

        let l = 0.4122214708 * r_lin + 0.5363325363 * g_lin + 0.0514459929 * b_lin;
        let m = 0.2119034982 * r_lin + 0.6806995451 * g_lin + 0.1073969566 * b_lin;
        let s = 0.0883024619 * r_lin + 0.2817188376 * g_lin + 0.6299787005 * b_lin;

        let l_ = l.max(0.0).cbrt();
        let m_ = m.max(0.0).cbrt();
        let s_ = s.max(0.0).cbrt();

        OklabColor {
            l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        }
    }

    /// Squared perceptual color difference in OKLab color space.
    #[inline]
    pub fn distance_oklab(&self, other: &Self) -> f32 {
        self.to_oklab().delta_e_sq(&other.to_oklab())
    }

    /// CIEDE2000 (ΔE00) perceptual color difference.
    #[inline]
    pub fn distance_ciede2000(&self, other: &Self) -> f32 {
        self.to_lab().delta_e_ciede2000(&other.to_lab())
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

/// CIE L*a*b* perceptual color representation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LabColor {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl LabColor {
    /// CIEDE2000 (ΔE00) total color difference formula (ISO/CIE standard).
    pub fn delta_e_ciede2000(&self, other: &Self) -> f32 {
        let l1 = self.l;
        let a1 = self.a;
        let b1 = self.b;
        let l2 = other.l;
        let a2 = other.a;
        let b2 = other.b;

        let c1_star = (a1 * a1 + b1 * b1).sqrt();
        let c2_star = (a2 * a2 + b2 * b2).sqrt();
        let c_bar_star = (c1_star + c2_star) * 0.5;

        let c_bar_7 = c_bar_star.powi(7);
        let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 25.0f32.powi(7))).sqrt());

        let a1_prime = (1.0 + g) * a1;
        let a2_prime = (1.0 + g) * a2;

        let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
        let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
        let c_bar_prime = (c1_prime + c2_prime) * 0.5;

        let to_deg = |rad: f32| -> f32 {
            let deg = rad.to_degrees();
            if deg < 0.0 { deg + 360.0 } else { deg }
        };

        let h1_prime = if a1_prime == 0.0 && b1 == 0.0 { 0.0 } else { to_deg(b1.atan2(a1_prime)) };
        let h2_prime = if a2_prime == 0.0 && b2 == 0.0 { 0.0 } else { to_deg(b2.atan2(a2_prime)) };

        let delta_l_prime = l2 - l1;
        let l_bar_prime = (l1 + l2) * 0.5;

        let delta_c_prime = c2_prime - c1_prime;

        let delta_h_deg = if c1_prime * c2_prime == 0.0 {
            0.0
        } else if (h2_prime - h1_prime).abs() <= 180.0 {
            h2_prime - h1_prime
        } else if h2_prime - h1_prime > 180.0 {
            (h2_prime - h1_prime) - 360.0
        } else {
            (h2_prime - h1_prime) + 360.0
        };

        let delta_h_prime = 2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_deg.to_radians() * 0.5).sin();

        let h_bar_prime = if c1_prime * c2_prime == 0.0 {
            h1_prime + h2_prime
        } else if (h1_prime - h2_prime).abs() <= 180.0 {
            (h1_prime + h2_prime) * 0.5
        } else if h1_prime + h2_prime < 360.0 {
            (h1_prime + h2_prime + 360.0) * 0.5
        } else {
            (h1_prime + h2_prime - 360.0) * 0.5
        };

        let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
            + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
            + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
            - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();

        let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
        let c_bar_prime_7 = c_bar_prime.powi(7);
        let r_c = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 25.0f32.powi(7))).sqrt();

        let l_bar_minus_50_sq = (l_bar_prime - 50.0).powi(2);
        let s_l = 1.0 + (0.015 * l_bar_minus_50_sq) / (20.0 + l_bar_minus_50_sq).sqrt();
        let s_c = 1.0 + 0.045 * c_bar_prime;
        let s_h = 1.0 + 0.015 * c_bar_prime * t;

        let r_t = -(2.0 * delta_theta.to_radians()).sin() * r_c;

        let dl_term = delta_l_prime / s_l;
        let dc_term = delta_c_prime / s_c;
        let dh_term = delta_h_prime / s_h;

        let de_sq = dl_term * dl_term + dc_term * dc_term + dh_term * dh_term + r_t * dc_term * dh_term;
        de_sq.max(0.0).sqrt()
    }
}

/// Björn Ottosson's OKLab color representation (perceptually uniform with stable blue hues).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct OklabColor {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl OklabColor {
    #[inline]
    pub fn delta_e_sq(&self, other: &Self) -> f32 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        dl * dl + da * da + db * db
    }

    #[inline]
    pub fn delta_e(&self, other: &Self) -> f32 {
        self.delta_e_sq(other).sqrt()
    }
}

/// Color Vision Deficiency (CVD) simulation modes based on physiologically grounded
/// dichromacy reduction matrices (Brettel, Viénot & Mollon 1997; Machado et al. 2009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CvdMode {
    #[default]
    Normal,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Achromatopsia,
}

impl CvdMode {
    pub const ALL: [Self; 5] = [Self::Normal, Self::Protanopia, Self::Deuteranopia, Self::Tritanopia, Self::Achromatopsia];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal Vision",
            Self::Protanopia => "Protanopia (Red-Blind)",
            Self::Deuteranopia => "Deuteranopia (Green-Blind)",
            Self::Tritanopia => "Tritanopia (Blue-Blind)",
            Self::Achromatopsia => "Achromatopsia (Monochrome)",
        }
    }
}

impl RgbColor {
    /// Simulate how this color appears under various forms of Color Vision Deficiency (CVD).
    #[inline]
    pub fn simulate_cvd(&self, mode: CvdMode) -> Self {
        match mode {
            CvdMode::Normal => *self,
            CvdMode::Protanopia => {
                let r = 0.56667 * self.r as f32 + 0.43333 * self.g as f32;
                let g = 0.55833 * self.r as f32 + 0.44167 * self.g as f32;
                let b = 0.24167 * self.g as f32 + 0.75833 * self.b as f32;
                Self::new(r.clamp(0.0, 255.0).round() as u8, g.clamp(0.0, 255.0).round() as u8, b.clamp(0.0, 255.0).round() as u8)
            }
            CvdMode::Deuteranopia => {
                let r = 0.62500 * self.r as f32 + 0.37500 * self.g as f32;
                let g = 0.70000 * self.r as f32 + 0.30000 * self.g as f32;
                let b = 0.30000 * self.g as f32 + 0.70000 * self.b as f32;
                Self::new(r.clamp(0.0, 255.0).round() as u8, g.clamp(0.0, 255.0).round() as u8, b.clamp(0.0, 255.0).round() as u8)
            }
            CvdMode::Tritanopia => {
                let r = 0.95000 * self.r as f32 + 0.05000 * self.g as f32;
                let g = 0.43333 * self.g as f32 + 0.56667 * self.b as f32;
                let b = 0.47500 * self.g as f32 + 0.52500 * self.b as f32;
                Self::new(r.clamp(0.0, 255.0).round() as u8, g.clamp(0.0, 255.0).round() as u8, b.clamp(0.0, 255.0).round() as u8)
            }
            CvdMode::Achromatopsia => {
                let y = self.luminance();
                Self::new(y, y, y)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvd_simulation_modes() {
        let red = RgbColor::new(255, 0, 0);
        assert_eq!(red.simulate_cvd(CvdMode::Normal), red);

        let protan = red.simulate_cvd(CvdMode::Protanopia);
        // Under Protanopia, pure red shifts toward olive/yellow-brown
        assert!(protan.r > 100 && protan.g > 100);

        let achro = red.simulate_cvd(CvdMode::Achromatopsia);
        // Achromatopsia must have equal R, G, B channels
        assert_eq!(achro.r, achro.g);
        assert_eq!(achro.g, achro.b);
    }
}
