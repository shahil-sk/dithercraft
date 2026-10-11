use crate::color::RgbColor;
use crate::error::DitherError;
use serde::{Deserialize, Serialize};

/// Color swatch entry with lock state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Swatch {
    pub color: RgbColor,
    pub locked: bool,
}

impl Swatch {
    pub const fn new(color: RgbColor, locked: bool) -> Self {
        Self { color, locked }
    }
}

/// Palette holding quantized target colors and swatch lock controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub swatches: Vec<Swatch>,
}

impl Palette {
    pub fn new(name: impl Into<String>, colors: Vec<RgbColor>) -> Result<Self, DitherError> {
        if colors.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let swatches = colors.into_iter().map(|c| Swatch::new(c, false)).collect();
        Ok(Self { name: name.into(), swatches })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.swatches.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.swatches.is_empty()
    }

    pub fn colors(&self) -> Vec<RgbColor> {
        self.swatches.iter().map(|s| s.color).collect()
    }

    /// Set locked state for a swatch slot.
    pub fn set_locked(&mut self, index: usize, locked: bool) -> bool {
        if let Some(swatch) = self.swatches.get_mut(index) {
            swatch.locked = locked;
            true
        } else {
            false
        }
    }

    /// Find the nearest palette color using Redmean perceptual color distance.
    pub fn find_nearest(&self, target: &RgbColor) -> RgbColor {
        let mut best_color = self.swatches[0].color;
        let mut min_dist = u32::MAX;

        for swatch in &self.swatches {
            let dist = target.distance_redmean(&swatch.color);
            if dist < min_dist {
                min_dist = dist;
                best_color = swatch.color;
            }
        }
        best_color
    }

    /// Find the nearest swatch index.
    pub fn find_nearest_index(&self, target: &RgbColor) -> usize {
        let mut best_idx = 0;
        let mut min_dist = u32::MAX;

        for (idx, swatch) in self.swatches.iter().enumerate() {
            let dist = target.distance_redmean(&swatch.color);
            if dist < min_dist {
                min_dist = dist;
                best_idx = idx;
            }
        }
        best_idx
    }

    /// Tonal sorting: Sort swatches strictly by luminance (ascending or descending).
    pub fn sort_by_luminance(&mut self, ascending: bool) {
        if ascending {
            self.swatches.sort_by_key(|a| a.color.luminance());
        } else {
            self.swatches.sort_by_key(|a| std::cmp::Reverse(a.color.luminance()));
        }
    }

    /// Tonal cycling: Rotates active palette colors by step while preserving swatch lock slots.
    pub fn cycle_tonal(&self, step: usize) -> Self {
        if self.swatches.is_empty() {
            return self.clone();
        }
        let n = self.swatches.len();
        let shift = step % n;
        let mut cycled = self.clone();

        for i in 0..n {
            if !self.swatches[i].locked {
                let target_idx = (i + shift) % n;
                cycled.swatches[i].color = self.swatches[target_idx].color;
            }
        }
        cycled
    }

    /// Randomize unlocked swatch slots using a lightweight zero-dependency LCG.
    pub fn randomize_unlocked(&mut self, mut seed: u64) {
        for swatch in &mut self.swatches {
            if !swatch.locked {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let r = ((seed >> 48) & 0xFF) as u8;
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let g = ((seed >> 48) & 0xFF) as u8;
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let b = ((seed >> 48) & 0xFF) as u8;
                swatch.color = RgbColor::new(r, g, b);
            }
        }
    }

    // --- Bundled Presets ---

    /// 1-bit Monochrome Black & White
    pub fn monochrome() -> Self {
        Self { name: "Monochrome 1-bit".into(), swatches: vec![Swatch::new(RgbColor::BLACK, false), Swatch::new(RgbColor::WHITE, false)] }
    }

    /// Game Boy 4-shade olive green
    pub fn game_boy() -> Self {
        Self {
            name: "Nintendo Game Boy".into(),
            swatches: vec![
                Swatch::new(RgbColor::new(15, 56, 15), false),
                Swatch::new(RgbColor::new(48, 98, 48), false),
                Swatch::new(RgbColor::new(139, 172, 15), false),
                Swatch::new(RgbColor::new(155, 188, 15), false),
            ],
        }
    }

    /// IBM CGA Mode 1 (Black, Cyan, Magenta, White)
    pub fn cga_mode_1() -> Self {
        Self {
            name: "CGA Mode 1".into(),
            swatches: vec![
                Swatch::new(RgbColor::new(0, 0, 0), false),
                Swatch::new(RgbColor::new(85, 255, 255), false),
                Swatch::new(RgbColor::new(255, 85, 255), false),
                Swatch::new(RgbColor::new(255, 255, 255), false),
            ],
        }
    }

    /// Cyberpunk Neon (Dark Navy, Magenta, Cyan, Yellow)
    pub fn cyberpunk() -> Self {
        Self {
            name: "Cyberpunk Neon".into(),
            swatches: vec![
                Swatch::new(RgbColor::new(10, 10, 30), false),
                Swatch::new(RgbColor::new(255, 0, 128), false),
                Swatch::new(RgbColor::new(0, 240, 255), false),
                Swatch::new(RgbColor::new(255, 230, 0), false),
            ],
        }
    }
}
