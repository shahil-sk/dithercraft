use crate::buffer::ImageBuffer;
use crate::color::{ColorHarmony, OklchColor, RgbColor};
use crate::error::DitherError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported color distance evaluation metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColorMetric {
    #[default]
    Redmean,
    EuclideanRgb,
    CieLab,
    Oklab,
    Ciede2000,
}

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
    #[serde(default)]
    pub metric: ColorMetric,
}

impl Palette {
    pub fn new(name: impl Into<String>, colors: Vec<RgbColor>) -> Result<Self, DitherError> {
        if colors.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let swatches = colors.into_iter().map(|c| Swatch::new(c, false)).collect();
        Ok(Self { name: name.into(), swatches, metric: ColorMetric::default() })
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

    /// Generate a harmonious palette from a base color using OKLCH color wheel rules.
    pub fn from_harmony(base: RgbColor, harmony: ColorHarmony, count: usize) -> Self {
        let count = count.clamp(2, 64);
        let base_oklch = base.to_oklch();
        let offsets = harmony.hue_offsets();

        let mut swatches = Vec::with_capacity(count);
        let num_hues = offsets.len();
        let shades_per_hue = count.div_ceil(num_hues);

        for &offset in offsets {
            let hue = (base_oklch.h + offset).rem_euclid(360.0);
            for s in 0..shades_per_hue {
                if swatches.len() >= count {
                    break;
                }
                let l_factor = if shades_per_hue > 1 { 0.20 + 0.65 * (s as f32 / (shades_per_hue - 1) as f32) } else { base_oklch.l };
                let c_factor = (base_oklch.c * (0.6 + 0.4 * (1.0 - (l_factor - 0.5).abs() * 2.0))).clamp(0.02, 0.25);
                let col = OklchColor::new(l_factor, c_factor, hue).to_rgb();
                swatches.push(Swatch::new(col, false));
            }
        }

        Self { name: format!("{} ({})", harmony.name(), base.to_hex()), swatches, metric: ColorMetric::Oklab }
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

    /// Find the nearest palette color using the active color distance metric.
    pub fn find_nearest(&self, target: &RgbColor) -> RgbColor {
        self.find_nearest_with_metric(target, self.metric)
    }

    /// Find the nearest palette color using the specified color metric.
    pub fn find_nearest_with_metric(&self, target: &RgbColor, metric: ColorMetric) -> RgbColor {
        if self.swatches.is_empty() {
            return *target;
        }
        for swatch in &self.swatches {
            if swatch.color == *target {
                return *target;
            }
        }
        let mut best_color = self.swatches[0].color;

        match metric {
            ColorMetric::Redmean => {
                let mut min_dist = u32::MAX;
                for swatch in &self.swatches {
                    let dist = target.distance_redmean(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_color = swatch.color;
                    }
                }
            }
            ColorMetric::EuclideanRgb => {
                let mut min_dist = u32::MAX;
                for swatch in &self.swatches {
                    let dist = target.distance_sq(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_color = swatch.color;
                    }
                }
            }
            ColorMetric::CieLab => {
                let mut min_dist = f32::MAX;
                for swatch in &self.swatches {
                    let dist = target.distance_cielab_de(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_color = swatch.color;
                    }
                }
            }
            ColorMetric::Oklab => {
                let mut min_dist = f32::MAX;
                for swatch in &self.swatches {
                    let dist = target.distance_oklab(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_color = swatch.color;
                    }
                }
            }
            ColorMetric::Ciede2000 => {
                let mut min_dist = f32::MAX;
                for swatch in &self.swatches {
                    let dist = target.distance_ciede2000(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_color = swatch.color;
                    }
                }
            }
        }

        best_color
    }

    /// Find the nearest swatch index using the active color metric.
    pub fn find_nearest_index(&self, target: &RgbColor) -> usize {
        if self.swatches.is_empty() {
            return 0;
        }
        for (idx, swatch) in self.swatches.iter().enumerate() {
            if swatch.color == *target {
                return idx;
            }
        }
        let mut best_idx = 0;

        match self.metric {
            ColorMetric::Redmean => {
                let mut min_dist = u32::MAX;
                for (idx, swatch) in self.swatches.iter().enumerate() {
                    let dist = target.distance_redmean(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
            }
            ColorMetric::EuclideanRgb => {
                let mut min_dist = u32::MAX;
                for (idx, swatch) in self.swatches.iter().enumerate() {
                    let dist = target.distance_sq(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
            }
            ColorMetric::CieLab => {
                let mut min_dist = f32::MAX;
                for (idx, swatch) in self.swatches.iter().enumerate() {
                    let dist = target.distance_cielab_de(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
            }
            ColorMetric::Oklab => {
                let mut min_dist = f32::MAX;
                for (idx, swatch) in self.swatches.iter().enumerate() {
                    let dist = target.distance_oklab(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
            }
            ColorMetric::Ciede2000 => {
                let mut min_dist = f32::MAX;
                for (idx, swatch) in self.swatches.iter().enumerate() {
                    let dist = target.distance_ciede2000(&swatch.color);
                    if dist < min_dist {
                        min_dist = dist;
                        best_idx = idx;
                    }
                }
            }
        }
        best_idx
    }

    /// Construct a fast spatial 3D KD-Tree for O(log K) nearest neighbor color queries.
    pub fn to_kd_tree(&self) -> PaletteKdTree {
        PaletteKdTree::build(self)
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

    // --- Multi-Format Parsers ---

    /// Parse palette from JSON string (either `Palette` JSON or string array of hex codes).
    pub fn parse_json(content: &str) -> Result<Self, DitherError> {
        if let Ok(pal) = serde_json::from_str::<Palette>(content)
            && !pal.is_empty()
        {
            return Ok(pal);
        }

        #[derive(Deserialize)]
        struct HexArrayPalette {
            name: Option<String>,
            colors: Option<Vec<String>>,
        }

        if let Ok(parsed) = serde_json::from_str::<HexArrayPalette>(content)
            && let Some(colors) = parsed.colors
        {
            let rgb_colors: Vec<RgbColor> = colors.iter().filter_map(|h| RgbColor::from_hex(h)).collect();
            if !rgb_colors.is_empty() {
                return Self::new(parsed.name.unwrap_or_else(|| "Imported JSON".into()), rgb_colors);
            }
        }

        if let Ok(hex_list) = serde_json::from_str::<Vec<String>>(content) {
            let rgb_colors: Vec<RgbColor> = hex_list.iter().filter_map(|h| RgbColor::from_hex(h)).collect();
            if !rgb_colors.is_empty() {
                return Self::new("Imported JSON", rgb_colors);
            }
        }

        Err(DitherError::PresetLoadFailed("Invalid JSON palette format".into()))
    }

    /// Parse palette from plain hex codes file (one hex string per line).
    pub fn parse_hex(name: &str, content: &str) -> Result<Self, DitherError> {
        let mut colors = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }
            if let Some(col) = RgbColor::from_hex(trimmed) {
                colors.push(col);
            }
        }
        if colors.is_empty() {
            return Err(DitherError::PresetLoadFailed("No valid hex colors found in file".into()));
        }
        Self::new(name, colors)
    }

    /// Parse GIMP Palette (.gpl) standard file format.
    pub fn parse_gpl(content: &str) -> Result<Self, DitherError> {
        let mut name = "GIMP Palette".to_string();
        let mut colors = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("GIMP Palette") {
                continue;
            }
            if let Some(name_str) = trimmed.strip_prefix("Name:") {
                name = name_str.trim().to_string();
                continue;
            }
            if trimmed.starts_with('#') || trimmed.starts_with("Columns:") {
                continue;
            }

            // Line format: R G B [Name]
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3
                && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>(), parts[2].parse::<u8>())
            {
                colors.push(RgbColor::new(r, g, b));
            }
        }

        if colors.is_empty() {
            return Err(DitherError::PresetLoadFailed("No colors found in GPL file".into()));
        }
        Self::new(name, colors)
    }

    /// Parse JASC-PAL (.pal) palette file format.
    pub fn parse_pal(content: &str) -> Result<Self, DitherError> {
        let mut lines = content.lines().map(str::trim).filter(|l| !l.is_empty());
        let header = lines.next().ok_or_else(|| DitherError::PresetLoadFailed("Empty PAL file".into()))?;

        if !header.eq_ignore_ascii_case("JASC-PAL") {
            // Try fallback: plain RGB triplets
            let mut colors = Vec::new();
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3
                    && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>(), parts[2].parse::<u8>())
                {
                    colors.push(RgbColor::new(r, g, b));
                }
            }
            if colors.is_empty() {
                return Err(DitherError::PresetLoadFailed("Invalid PAL format".into()));
            }
            return Self::new("Imported PAL", colors);
        }

        let _version = lines.next(); // 0100
        let count_str = lines.next().ok_or_else(|| DitherError::PresetLoadFailed("Missing count in JASC-PAL".into()))?;
        let _expected_count: usize = count_str.parse().unwrap_or(0);

        let mut colors = Vec::new();
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3
                && let (Ok(r), Ok(g), Ok(b)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>(), parts[2].parse::<u8>())
            {
                colors.push(RgbColor::new(r, g, b));
            }
        }

        if colors.is_empty() {
            return Err(DitherError::PresetLoadFailed("No valid colors in JASC-PAL".into()));
        }
        Self::new("JASC-PAL", colors)
    }

    /// Parse Adobe Photoshop Color Table (.act) binary file format.
    pub fn parse_act(name: &str, bytes: &[u8]) -> Result<Self, DitherError> {
        if bytes.len() < 3 {
            return Err(DitherError::PresetLoadFailed("ACT file too short".into()));
        }
        let total_triplets = bytes.len() / 3;
        let count = if bytes.len() >= 772 {
            let declared = u16::from_be_bytes([bytes[768], bytes[769]]) as usize;
            if declared > 0 && declared <= 256 { declared } else { 256.min(total_triplets) }
        } else if bytes.len() >= 768 {
            256
        } else {
            total_triplets
        };

        let mut colors = Vec::with_capacity(count);
        for i in 0..count {
            if i * 3 + 2 < bytes.len() {
                colors.push(RgbColor::new(bytes[i * 3], bytes[i * 3 + 1], bytes[i * 3 + 2]));
            }
        }
        if colors.is_empty() {
            return Err(DitherError::PresetLoadFailed("No valid colors found in ACT file".into()));
        }
        Self::new(name, colors)
    }

    /// Export palette in GIMP Palette (.gpl) standard format.
    pub fn export_gpl(&self) -> String {
        let mut out = format!("GIMP Palette\nName: {}\nColumns: 4\n#\n", self.name);
        for s in &self.swatches {
            out.push_str(&format!("{:3} {:3} {:3} Swatch\n", s.color.r, s.color.g, s.color.b));
        }
        out
    }

    /// Export palette in plain Hex (.hex) format (one '#RRGGBB' per line).
    pub fn export_hex(&self) -> String {
        let mut out = String::new();
        for s in &self.swatches {
            out.push_str(&format!("#{:02x}{:02x}{:02x}\n", s.color.r, s.color.g, s.color.b));
        }
        out
    }

    /// Export palette in JASC-PAL (.pal) format.
    pub fn export_pal(&self) -> String {
        let mut out = format!("JASC-PAL\n0100\n{}\n", self.swatches.len());
        for s in &self.swatches {
            out.push_str(&format!("{} {} {}\n", s.color.r, s.color.g, s.color.b));
        }
        out
    }

    /// Export palette in Adobe Photoshop Color Table (.act) 772-byte binary format.
    pub fn export_act(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 772];
        let count = self.swatches.len().min(256);
        for (i, s) in self.swatches.iter().take(count).enumerate() {
            bytes[i * 3] = s.color.r;
            bytes[i * 3 + 1] = s.color.g;
            bytes[i * 3 + 2] = s.color.b;
        }
        let count_be = (count as u16).to_be_bytes();
        bytes[768] = count_be[0];
        bytes[769] = count_be[1];
        bytes[770] = 0xFF; // transparent index high byte (0xFFFF for none)
        bytes[771] = 0xFF; // transparent index low byte
        bytes
    }

    /// Export palette to a file path auto-detecting format by extension (.gpl, .hex, .pal, .act, .json).
    pub fn export_to_file(&self, path: &Path) -> Result<(), DitherError> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "act" => std::fs::write(path, self.export_act()).map_err(|e| DitherError::PresetLoadFailed(e.to_string())),
            "hex" => std::fs::write(path, self.export_hex()).map_err(|e| DitherError::PresetLoadFailed(e.to_string())),
            "pal" => std::fs::write(path, self.export_pal()).map_err(|e| DitherError::PresetLoadFailed(e.to_string())),
            "json" => {
                let json = serde_json::to_string_pretty(self).map_err(|e| DitherError::PresetLoadFailed(e.to_string()))?;
                std::fs::write(path, json).map_err(|e| DitherError::PresetLoadFailed(e.to_string()))
            }
            _ => std::fs::write(path, self.export_gpl()).map_err(|e| DitherError::PresetLoadFailed(e.to_string())),
        }
    }

    /// Import palette from a file path auto-detecting format by extension or content (.json, .hex, .gpl, .pal, .act).
    pub fn import_from_file(path: &Path) -> Result<Self, DitherError> {
        let bytes = std::fs::read(path).map_err(|e| DitherError::PresetLoadFailed(e.to_string()))?;
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported Palette");

        if (ext == "act" || (bytes.len() >= 768 && bytes.len() <= 772))
            && let Ok(pal) = Self::parse_act(name, &bytes)
        {
            return Ok(pal);
        }

        let content = String::from_utf8_lossy(&bytes);
        match ext.as_str() {
            "json" => Self::parse_json(&content),
            "hex" => Self::parse_hex(name, &content),
            "gpl" => Self::parse_gpl(&content),
            "pal" => Self::parse_pal(&content),
            _ => Self::parse_json(&content)
                .or_else(|_| Self::parse_gpl(&content))
                .or_else(|_| Self::parse_pal(&content))
                .or_else(|_| Self::parse_hex(name, &content))
                .or_else(|_| Self::parse_act(name, &bytes)),
        }
    }

    // --- Color Extraction Engine ---

    /// Median Cut color quantization (Paul Heckbert 1982, ACM SIGGRAPH).
    /// Extracts `num_colors` representative palette tones from an image.
    pub fn from_image_median_cut(name: impl Into<String>, image: &ImageBuffer, num_colors: usize) -> Result<Self, DitherError> {
        if image.width == 0 || image.height == 0 {
            return Err(DitherError::ZeroDimension);
        }
        let target_k = num_colors.clamp(2, 256);

        // Subsample pixels for speed (up to 16,384 samples)
        let total_pixels = image.pixel_count();
        let step = (total_pixels / 16384).max(1);
        let mut pixels = Vec::with_capacity(total_pixels / step + 1);

        for chunk in image.data.chunks_exact(3).step_by(step) {
            pixels.push(RgbColor::new(chunk[0], chunk[1], chunk[2]));
        }

        if pixels.is_empty() {
            return Err(DitherError::ZeroDimension);
        }

        // Bounding boxes in RGB space
        let mut boxes: Vec<Vec<RgbColor>> = vec![pixels];

        while boxes.len() < target_k {
            // Pick the box with the greatest channel range
            let mut best_box_idx = 0;
            let mut max_range = 0i32;
            let mut best_axis = 0usize; // 0=R, 1=G, 2=B

            for (idx, b) in boxes.iter().enumerate() {
                if b.len() < 2 {
                    continue;
                }
                let (mut min_r, mut max_r) = (255u8, 0u8);
                let (mut min_g, mut max_g) = (255u8, 0u8);
                let (mut min_b, mut max_b) = (255u8, 0u8);
                for c in b {
                    min_r = min_r.min(c.r);
                    max_r = max_r.max(c.r);
                    min_g = min_g.min(c.g);
                    max_g = max_g.max(c.g);
                    min_b = min_b.min(c.b);
                    max_b = max_b.max(c.b);
                }

                let range_r = (max_r as i32) - (min_r as i32);
                let range_g = (max_g as i32) - (min_g as i32);
                let range_b = (max_b as i32) - (min_b as i32);
                let range = range_r.max(range_g).max(range_b);

                if range > max_range {
                    max_range = range;
                    best_box_idx = idx;
                    best_axis = if range == range_r {
                        0
                    } else if range == range_g {
                        1
                    } else {
                        2
                    };
                }
            }

            if max_range <= 0 {
                break; // Cannot subdivide further
            }

            let mut target_box = boxes.swap_remove(best_box_idx);
            match best_axis {
                0 => target_box.sort_unstable_by_key(|c| c.r),
                1 => target_box.sort_unstable_by_key(|c| c.g),
                _ => target_box.sort_unstable_by_key(|c| c.b),
            }

            let mid = target_box.len() / 2;
            let right = target_box.split_off(mid);
            boxes.push(target_box);
            boxes.push(right);
        }

        // Average RGB per box
        let mut result_colors = Vec::with_capacity(boxes.len());
        for b in boxes {
            if b.is_empty() {
                continue;
            }
            let (mut sum_r, mut sum_g, mut sum_b) = (0u64, 0u64, 0u64);
            for c in &b {
                sum_r += c.r as u64;
                sum_g += c.g as u64;
                sum_b += c.b as u64;
            }
            let n = b.len() as u64;
            result_colors.push(RgbColor::new((sum_r / n) as u8, (sum_g / n) as u8, (sum_b / n) as u8));
        }

        Self::new(name, result_colors)
    }

    /// K-Means color clustering (Stuart Lloyd 1982).
    /// Finds `k` optimal centroids iteratively.
    pub fn from_image_kmeans(name: impl Into<String>, image: &ImageBuffer, k: usize, max_iterations: usize) -> Result<Self, DitherError> {
        let initial_pal = Self::from_image_median_cut("initial", image, k)?;
        let mut centroids = initial_pal.colors();
        let k = centroids.len();

        // Sample pixels
        let total = image.pixel_count();
        let step = (total / 8192).max(1);
        let samples: Vec<RgbColor> = image.data.chunks_exact(3).step_by(step).map(|chunk| RgbColor::new(chunk[0], chunk[1], chunk[2])).collect();

        if samples.is_empty() {
            return Err(DitherError::ZeroDimension);
        }

        for _ in 0..max_iterations.clamp(1, 30) {
            let mut cluster_sums = vec![(0u64, 0u64, 0u64, 0u64); k];

            for px in &samples {
                let mut best_idx = 0;
                let mut min_d = u32::MAX;
                for (ci, cent) in centroids.iter().enumerate() {
                    let d = px.distance_redmean(cent);
                    if d < min_d {
                        min_d = d;
                        best_idx = ci;
                    }
                }
                cluster_sums[best_idx].0 += px.r as u64;
                cluster_sums[best_idx].1 += px.g as u64;
                cluster_sums[best_idx].2 += px.b as u64;
                cluster_sums[best_idx].3 += 1;
            }

            let mut changed = false;
            for (ci, cent) in centroids.iter_mut().enumerate() {
                let count = cluster_sums[ci].3;
                if let (Some(r), Some(g), Some(b)) =
                    (cluster_sums[ci].0.checked_div(count), cluster_sums[ci].1.checked_div(count), cluster_sums[ci].2.checked_div(count))
                {
                    let new_col = RgbColor::new(r as u8, g as u8, b as u8);
                    if *cent != new_col {
                        *cent = new_col;
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        Self::new(name, centroids)
    }

    // --- Bundled Retro Presets ---

    /// 1-bit Monochrome Black & White
    pub fn monochrome() -> Self {
        Self {
            name: "Monochrome 1-bit".into(),
            swatches: vec![Swatch::new(RgbColor::BLACK, false), Swatch::new(RgbColor::WHITE, false)],
            metric: ColorMetric::default(),
        }
    }

    /// Macintosh 1-bit Monochrome
    pub fn mac_1bit() -> Self {
        Self {
            name: "Macintosh 1-bit".into(),
            swatches: vec![Swatch::new(RgbColor::BLACK, false), Swatch::new(RgbColor::WHITE, false)],
            metric: ColorMetric::default(),
        }
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
            metric: ColorMetric::default(),
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
            metric: ColorMetric::default(),
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
            metric: ColorMetric::default(),
        }
    }

    /// Apple II (16 colors composite palette)
    pub fn apple_ii() -> Self {
        let hexes = [
            "#000000", "#901740", "#402CA5", "#D043E5", "#006940", "#808080", "#2F95E5", "#BFABFF", "#405400", "#D06A1A", "#8A8A8A", "#FF89E5",
            "#2FD340", "#BFE880", "#6FEFB5", "#FFFFFF",
        ];
        let colors = hexes.into_iter().filter_map(RgbColor::from_hex).collect();
        Self::new("Apple II", colors).unwrap_or_else(|_| Self::monochrome())
    }

    /// Commodore 64 (16 authentic Pepto colors)
    pub fn commodore_64() -> Self {
        let hexes = [
            "#000000", "#FFFFFF", "#880000", "#AAFFEE", "#CC44CC", "#00CC55", "#0000AA", "#EEEE77", "#DD8855", "#664400", "#FF7777", "#333333",
            "#777777", "#AAFF66", "#0088FF", "#BBBBBB",
        ];
        let colors = hexes.into_iter().filter_map(RgbColor::from_hex).collect();
        Self::new("Commodore 64", colors).unwrap_or_else(|_| Self::monochrome())
    }

    /// Nintendo Entertainment System (NES 16-color iconic subset)
    pub fn nes() -> Self {
        let hexes = [
            "#000000", "#FCFCFC", "#F83800", "#0058F8", "#00A800", "#FCA044", "#F87858", "#9878F8", "#58D854", "#00E8D8", "#F8B800", "#B8B8F8",
            "#7C7C7C", "#6888FC", "#F85898", "#983800",
        ];
        let colors = hexes.into_iter().filter_map(RgbColor::from_hex).collect();
        Self::new("NES", colors).unwrap_or_else(|_| Self::monochrome())
    }

    /// PICO-8 (16 iconic fantasy console colors)
    pub fn pico_8() -> Self {
        let hexes = [
            "#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F", "#C2C3C7", "#FFF1E8", "#FF004D", "#FFA300", "#FFEC27", "#00E436",
            "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA",
        ];
        let colors = hexes.into_iter().filter_map(RgbColor::from_hex).collect();
        Self::new("PICO-8", colors).unwrap_or_else(|_| Self::monochrome())
    }
}

/// 3D KD-Tree for O(log K) accelerated nearest neighbor search in RGB color space.
#[derive(Debug, Clone)]
pub struct PaletteKdTree {
    root: Option<Box<KdNode>>,
}

#[derive(Debug, Clone)]
struct KdNode {
    point: [i32; 3],
    color: RgbColor,
    axis: usize,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

impl PaletteKdTree {
    /// Builds a balanced 3D KD-Tree from the palette swatches.
    pub fn build(palette: &Palette) -> Self {
        let mut points: Vec<([i32; 3], RgbColor)> =
            palette.swatches.iter().map(|s| ([s.color.r as i32, s.color.g as i32, s.color.b as i32], s.color)).collect();
        let root = Self::build_recursive(&mut points, 0);
        Self { root }
    }

    fn build_recursive(points: &mut [([i32; 3], RgbColor)], depth: usize) -> Option<Box<KdNode>> {
        if points.is_empty() {
            return None;
        }
        let axis = depth % 3;
        points.sort_by_key(|p| p.0[axis]);
        let mid = points.len() / 2;
        let (point, color) = points[mid];

        let left = Self::build_recursive(&mut points[..mid], depth + 1);
        let right = Self::build_recursive(&mut points[mid + 1..], depth + 1);

        Some(Box::new(KdNode { point, color, axis, left, right }))
    }

    /// Finds the nearest color in O(log K) expected time without heap allocations.
    pub fn find_nearest(&self, target: &RgbColor) -> Option<RgbColor> {
        let root = self.root.as_ref()?;
        let target_pt = [target.r as i32, target.g as i32, target.b as i32];
        let mut best_dist = i32::MAX;
        let mut best_color = root.color;

        Self::search_recursive(root, &target_pt, &mut best_dist, &mut best_color);
        Some(best_color)
    }

    fn search_recursive(node: &KdNode, target: &[i32; 3], best_dist: &mut i32, best_color: &mut RgbColor) {
        let dx = target[0] - node.point[0];
        let dy = target[1] - node.point[1];
        let dz = target[2] - node.point[2];
        let dist = dx * dx + dy * dy + dz * dz;

        if dist < *best_dist {
            *best_dist = dist;
            *best_color = node.color;
        }

        let axis = node.axis;
        let diff = target[axis] - node.point[axis];
        let (first, second) = if diff <= 0 { (&node.left, &node.right) } else { (&node.right, &node.left) };

        if let Some(child) = first {
            Self::search_recursive(child, target, best_dist, best_color);
        }

        if diff * diff < *best_dist
            && let Some(child) = second
        {
            Self::search_recursive(child, target, best_dist, best_color);
        }
    }
}
