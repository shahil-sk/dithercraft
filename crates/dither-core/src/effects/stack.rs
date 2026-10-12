use super::chromatic::ChromaticAberration;
use super::crt::VintageCrtSimulation;
use super::glow::EpsilonGlow;
use super::jpeg_glitch::JpegGlitchCore;
use super::spatial::{GaussianBlur, Sharpen};
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::lut::Cube3dLut;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Blend modes for compositing downstream effect layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    ColorDodge,
    Difference,
}

impl BlendMode {
    pub const ALL: [Self; 6] = [Self::Normal, Self::Multiply, Self::Screen, Self::Overlay, Self::ColorDodge, Self::Difference];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::ColorDodge => "Color Dodge",
            Self::Difference => "Difference",
        }
    }

    /// Blend single 8-bit channel value.
    #[inline]
    pub fn blend_channel(&self, base: u8, fx: u8) -> u8 {
        let b = base as f32;
        let f = fx as f32;
        let res = match self {
            Self::Normal => f,
            Self::Multiply => (b * f) / 255.0,
            Self::Screen => 255.0 - ((255.0 - b) * (255.0 - f)) / 255.0,
            Self::Overlay => {
                if b < 128.0 {
                    (2.0 * b * f) / 255.0
                } else {
                    255.0 - (2.0 * (255.0 - b) * (255.0 - f)) / 255.0
                }
            }
            Self::ColorDodge => {
                if fx == 255 {
                    255.0
                } else {
                    ((b * 255.0) / (255.0 - f)).min(255.0)
                }
            }
            Self::Difference => (b - f).abs(),
        };
        res.clamp(0.0, 255.0).round() as u8
    }

    /// Composite base and effect pixel with blend mode and opacity.
    #[inline]
    pub fn blend_pixel(&self, base: RgbColor, fx: RgbColor, opacity: f32) -> RgbColor {
        let op = opacity.clamp(0.0, 1.0);
        if op <= 0.0 {
            return base;
        }

        let blended_r = self.blend_channel(base.r, fx.r);
        let blended_g = self.blend_channel(base.g, fx.g);
        let blended_b = self.blend_channel(base.b, fx.b);

        if (op - 1.0).abs() < 0.001 {
            RgbColor::new(blended_r, blended_g, blended_b)
        } else {
            let one_minus = 1.0 - op;
            let r = ((base.r as f32 * one_minus) + (blended_r as f32 * op)).round().clamp(0.0, 255.0) as u8;
            let g = ((base.g as f32 * one_minus) + (blended_g as f32 * op)).round().clamp(0.0, 255.0) as u8;
            let b = ((base.b as f32 * one_minus) + (blended_b as f32 * op)).round().clamp(0.0, 255.0) as u8;
            RgbColor::new(r, g, b)
        }
    }
}

/// The specific effect kind and parameters encapsulated in an effect node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectKind {
    Glow(EpsilonGlow),
    Chromatic(ChromaticAberration),
    Glitch(JpegGlitchCore),
    Crt(VintageCrtSimulation),
    Blur(GaussianBlur),
    Sharpen(Sharpen),
    Lut3d(Cube3dLut),
}

/// Individual effect node in the reorderable effect stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectNode {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub opacity: f32, // 0.0 to 1.0
    pub blend_mode: BlendMode,
    pub kind: EffectKind,
}

impl EffectNode {
    pub fn new(id: u64, name: impl Into<String>, kind: EffectKind) -> Self {
        Self { id, name: name.into(), enabled: true, opacity: 1.0, blend_mode: BlendMode::Normal, kind }
    }

    /// Process input buffer through this node, respecting enabled flag, opacity, and blend mode.
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        if !self.enabled || self.opacity <= 0.0001 {
            return Ok(input.clone());
        }

        let raw_effect = match &self.kind {
            EffectKind::Glow(glow) => glow.apply(input)?,
            EffectKind::Chromatic(chromatic) => chromatic.apply(input)?,
            EffectKind::Glitch(glitch) => glitch.apply(input)?,
            EffectKind::Crt(crt) => crt.apply(input)?,
            EffectKind::Blur(blur) => blur.apply(input)?,
            EffectKind::Sharpen(sharpen) => sharpen.apply(input)?,
            EffectKind::Lut3d(lut) => {
                let mut out = input.clone();
                lut.apply_buffer(&mut out);
                out
            }
        };

        if self.blend_mode == BlendMode::Normal && (self.opacity - 1.0).abs() < 0.001 {
            return Ok(raw_effect);
        }

        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;
        let mode = self.blend_mode;
        let opacity = self.opacity;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_offset = y * width * 3;
            let base_row = &input.data[row_offset..row_offset + width * 3];
            let fx_row = &raw_effect.data[row_offset..row_offset + width * 3];

            for x in 0..width {
                let idx = x * 3;
                let base_px = RgbColor::new(base_row[idx], base_row[idx + 1], base_row[idx + 2]);
                let fx_px = RgbColor::new(fx_row[idx], fx_row[idx + 1], fx_row[idx + 2]);
                let blended = mode.blend_pixel(base_px, fx_px, opacity);
                row_out[idx] = blended.r;
                row_out[idx + 1] = blended.g;
                row_out[idx + 2] = blended.b;
            }
        });

        Ok(out)
    }
}

/// Draggable and reorderable downstream effects stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectStack {
    pub nodes: Vec<EffectNode>,
    next_id: u64,
}

impl Default for EffectStack {
    fn default() -> Self {
        Self::default_stack()
    }
}

impl EffectStack {
    /// Create an empty effect stack.
    pub fn empty() -> Self {
        Self { nodes: Vec::new(), next_id: 1 }
    }

    /// Default production effect stack with standard layers initialized in optimal sequence.
    pub fn default_stack() -> Self {
        let nodes = vec![
            EffectNode::new(1, "Epsilon Glow", EffectKind::Glow(EpsilonGlow::default())),
            EffectNode::new(2, "Chromatic Aberration", EffectKind::Chromatic(ChromaticAberration::default())),
            EffectNode::new(3, "JPEG Glitch Core", EffectKind::Glitch(JpegGlitchCore::default())),
            EffectNode::new(4, "Vintage CRT", EffectKind::Crt(VintageCrtSimulation::default())),
            EffectNode::new(5, "Gaussian Blur", EffectKind::Blur(GaussianBlur::default())),
            EffectNode::new(6, "Unsharp Mask", EffectKind::Sharpen(Sharpen::default())),
        ];
        Self { nodes, next_id: 7 }
    }

    /// Execute the complete effect stack sequentially across enabled layers.
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let mut current = input.clone();
        for node in &self.nodes {
            if node.enabled && node.opacity > 0.0001 {
                current = node.apply(&current)?;
            }
        }
        Ok(current)
    }

    /// Move node at `idx` up one position towards the beginning of the pipeline.
    pub fn move_up(&mut self, idx: usize) -> bool {
        if idx > 0 && idx < self.nodes.len() {
            self.nodes.swap(idx, idx - 1);
            true
        } else {
            false
        }
    }

    /// Move node at `idx` down one position towards the end of the pipeline.
    pub fn move_down(&mut self, idx: usize) -> bool {
        if idx + 1 < self.nodes.len() {
            self.nodes.swap(idx, idx + 1);
            true
        } else {
            false
        }
    }

    /// Duplicate node at `idx`, appending the clone immediately below it.
    pub fn duplicate(&mut self, idx: usize) -> Option<usize> {
        if idx < self.nodes.len() {
            let mut clone = self.nodes[idx].clone();
            clone.id = self.next_id;
            self.next_id += 1;
            clone.name = format!("{} (Copy)", clone.name);
            let insert_idx = idx + 1;
            self.nodes.insert(insert_idx, clone);
            Some(insert_idx)
        } else {
            None
        }
    }

    /// Remove node at `idx`.
    pub fn remove(&mut self, idx: usize) -> Option<EffectNode> {
        if idx < self.nodes.len() { Some(self.nodes.remove(idx)) } else { None }
    }

    /// Append a new effect node with unique ID assignment.
    pub fn add(&mut self, mut node: EffectNode) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        node.id = id;
        self.nodes.push(node);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_modes() {
        let base = RgbColor::new(100, 150, 200);
        let fx = RgbColor::new(50, 100, 150);

        // Normal blend mode
        let normal = BlendMode::Normal.blend_pixel(base, fx, 1.0);
        assert_eq!(normal, fx);

        // Opacity 0 returns base
        let zero_op = BlendMode::Normal.blend_pixel(base, fx, 0.0);
        assert_eq!(zero_op, base);

        // Multiply
        let mult = BlendMode::Multiply.blend_pixel(base, fx, 1.0);
        assert_eq!(mult.r, ((100.0f32 * 50.0) / 255.0).round() as u8);

        // Difference
        let diff = BlendMode::Difference.blend_pixel(base, fx, 1.0);
        assert_eq!(diff.r, 50);
        assert_eq!(diff.g, 50);
        assert_eq!(diff.b, 50);
    }

    #[test]
    fn test_effect_stack_reorder_and_duplicate() {
        let mut stack = EffectStack::default_stack();
        assert_eq!(stack.nodes.len(), 6);
        assert_eq!(stack.nodes[0].name, "Epsilon Glow");

        // Move second node up
        assert!(stack.move_up(1));
        assert_eq!(stack.nodes[0].name, "Chromatic Aberration");
        assert_eq!(stack.nodes[1].name, "Epsilon Glow");

        // Duplicate
        let dup_idx = stack.duplicate(0).expect("dup");
        assert_eq!(dup_idx, 1);
        assert_eq!(stack.nodes.len(), 7);
        assert!(stack.nodes[1].name.contains("(Copy)"));

        // Remove
        let removed = stack.remove(1).expect("removed");
        assert!(removed.name.contains("(Copy)"));
        assert_eq!(stack.nodes.len(), 6);
    }

    #[test]
    fn test_effect_stack_apply() {
        let mut stack = EffectStack::default_stack();
        // Disable all but unsharp mask
        for node in &mut stack.nodes {
            node.enabled = false;
        }
        stack.nodes[5].enabled = true;

        let mut buf = ImageBuffer::new(8, 8).expect("buf");
        buf.set_pixel(4, 4, RgbColor::new(255, 255, 255));

        let res = stack.apply(&buf).expect("apply");
        assert_eq!(res.width, 8);
        assert_eq!(res.height, 8);
    }
}
