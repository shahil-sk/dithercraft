//! Generative DSP, Reaction-Diffusion Morphogenesis & Flow Field Dithering (Phase 12)
//!
//! Citations:
//! - Alan Turing (1952), *The Chemical Basis of Morphogenesis*
//! - Pearson & Gray-Scott (1984), *Complex Patterns in a Simple System*
//! - Ken Perlin (1985), *An Image Synthesizer*
//! - Robert Ulichney (1993), *Void-and-Cluster Method for Generating Blue Noise*

use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Classical parameter regimes for the Gray-Scott reaction-diffusion PDE.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ReactionDiffusionMorphology {
    /// Labyrinthine organic maze patterns
    #[default]
    Labyrinth,
    /// Leopard spots and cellular clusters
    Spots,
    /// Soliton dots and localized solitary waves
    Solitons,
    /// Pulsating rhythmic wave fronts
    PulsatingWaves,
    /// Fingerprint whorls and epidermal ridges
    Fingerprints,
    /// Branching coral reef structures
    Coral,
    /// Custom feed rate (F) and kill rate (k)
    Custom { f: f32, k: f32 },
}

impl ReactionDiffusionMorphology {
    /// Return the `(F, k)` (Feed rate, Kill rate) parameters for this morphology.
    pub fn parameters(&self) -> (f32, f32) {
        match *self {
            Self::Labyrinth => (0.0545, 0.0620),
            Self::Spots => (0.0350, 0.0650),
            Self::Solitons => (0.0300, 0.0620),
            Self::PulsatingWaves => (0.0250, 0.0600),
            Self::Fingerprints => (0.0280, 0.0620),
            Self::Coral => (0.0550, 0.0630),
            Self::Custom { f, k } => (f, k),
        }
    }
}

/// Simulation configuration for Gray-Scott reaction-diffusion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionDiffusionConfig {
    pub morphology: ReactionDiffusionMorphology,
    pub diffusion_u: f32,
    pub diffusion_v: f32,
    pub dt: f32,
    pub steps: usize,
}

impl Default for ReactionDiffusionConfig {
    fn default() -> Self {
        Self { morphology: ReactionDiffusionMorphology::Labyrinth, diffusion_u: 1.0, diffusion_v: 0.5, dt: 1.0, steps: 80 }
    }
}

/// 2D Reaction-Diffusion PDE solver simulating biological morphogenesis.
#[derive(Debug, Clone)]
pub struct ReactionDiffusionSimulator {
    pub width: usize,
    pub height: usize,
    pub u: Vec<f32>,
    pub v: Vec<f32>,
    pub next_u: Vec<f32>,
    pub next_v: Vec<f32>,
    pub config: ReactionDiffusionConfig,
}

impl ReactionDiffusionSimulator {
    /// Create a simulator grid initialized with $U = 1.0$ and localized $V$ seed disturbance.
    pub fn new(width: usize, height: usize, config: ReactionDiffusionConfig) -> Self {
        let size = width * height;
        let mut u = vec![1.0_f32; size];
        let mut v = vec![0.0_f32; size];

        // Seed a central rectangular perturbation region
        let cx = width / 2;
        let cy = height / 2;
        let rad = (width.min(height) / 8).max(3);

        for y in cy.saturating_sub(rad)..(cy + rad).min(height) {
            for x in cx.saturating_sub(rad)..(cx + rad).min(width) {
                let idx = y * width + x;
                u[idx] = 0.5;
                v[idx] = 0.25;
            }
        }

        // Add a secondary off-center seed for asymmetrical turbulence
        let off_x = (cx + rad * 2).min(width.saturating_sub(2));
        let off_y = (cy + rad).min(height.saturating_sub(2));
        for dy in 0..4 {
            for dx in 0..4 {
                let idx = (off_y + dy) * width + (off_x + dx);
                if idx < size {
                    u[idx] = 0.4;
                    v[idx] = 0.3;
                }
            }
        }

        Self { width, height, next_u: vec![1.0; size], next_v: vec![0.0; size], u, v, config }
    }

    /// Advance the simulation by `n` iterations.
    pub fn step(&mut self, n: usize) {
        let (f, k) = self.config.morphology.parameters();
        let du = self.config.diffusion_u;
        let dv = self.config.diffusion_v;
        let dt = self.config.dt;
        let w = self.width;
        let h = self.height;

        for _ in 0..n {
            self.next_u.par_chunks_exact_mut(w).zip(self.next_v.par_chunks_exact_mut(w)).enumerate().for_each(|(y, (row_nu, row_nv))| {
                let y_up = if y == 0 { h - 1 } else { y - 1 };
                let y_dn = if y == h - 1 { 0 } else { y + 1 };

                let row_idx = y * w;
                let up_idx = y_up * w;
                let dn_idx = y_dn * w;

                for x in 0..w {
                    let x_lf = if x == 0 { w - 1 } else { x - 1 };
                    let x_rt = if x == w - 1 { 0 } else { x + 1 };

                    let cur = row_idx + x;
                    let u_cur = self.u[cur];
                    let v_cur = self.v[cur];

                    // 5-point discrete Laplacian with toroidal wrapping
                    let lap_u = self.u[row_idx + x_lf] + self.u[row_idx + x_rt] + self.u[up_idx + x] + self.u[dn_idx + x] - 4.0 * u_cur;

                    let lap_v = self.v[row_idx + x_lf] + self.v[row_idx + x_rt] + self.v[up_idx + x] + self.v[dn_idx + x] - 4.0 * v_cur;

                    let uvv = u_cur * v_cur * v_cur;

                    let delta_u = du * lap_u - uvv + f * (1.0 - u_cur);
                    let delta_v = dv * lap_v + uvv - (f + k) * v_cur;

                    row_nu[x] = (u_cur + delta_u * dt).clamp(0.0, 1.0);
                    row_nv[x] = (v_cur + delta_v * dt).clamp(0.0, 1.0);
                }
            });

            std::mem::swap(&mut self.u, &mut self.next_u);
            std::mem::swap(&mut self.v, &mut self.next_v);
        }
    }

    /// Render current chemical field $V$ as a grayscale `ImageBuffer`.
    pub fn to_image_buffer(&self) -> Result<ImageBuffer, DitherError> {
        let mut out = ImageBuffer::new(self.width as u32, self.height as u32)?;
        for (idx, &v_val) in self.v.iter().enumerate() {
            let px = (v_val * 4.0 * 255.0).clamp(0.0, 255.0) as u8;
            let o = idx * 3;
            out.data[o] = px;
            out.data[o + 1] = px;
            out.data[o + 2] = px;
        }
        Ok(out)
    }
}

/// Morphogenetic biological reaction-diffusion dither combinator.
#[derive(Debug, Clone, Default)]
pub struct ReactionDiffusionDither {
    pub config: ReactionDiffusionConfig,
}

impl ReactionDiffusionDither {
    pub fn new(config: ReactionDiffusionConfig) -> Self {
        Self { config }
    }

    /// Apply organic morphogenetic threshold modulation before palette quantization.
    pub fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        let mut sim = ReactionDiffusionSimulator::new(input.width as usize, input.height as usize, self.config.clone());
        sim.step(self.config.steps);

        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_in = y * width * 3;
            let row_grid = y * width;

            for x in 0..width {
                let v_val = sim.v[row_grid + x];
                let bias = ((v_val * 2.0 - 0.5) * 120.0) as i32;

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

impl crate::algorithms::traits::DitherAlgorithm for ReactionDiffusionDither {
    fn name(&self) -> &'static str {
        "Morphogenesis (Reaction-Diffusion)"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        self.apply(input, palette)
    }
}

/// 2D Vector Flow Field Generator for organic directional error diffusion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlowFieldConfig {
    pub scale: f32,
    pub swirl: f32,
    pub angle_offset: f32,
}

impl Default for FlowFieldConfig {
    fn default() -> Self {
        Self { scale: 0.04, swirl: 1.5, angle_offset: 0.0 }
    }
}

/// Directional Vector Flow Field Steered Error Diffusion.
#[derive(Debug, Clone, Default)]
pub struct FlowFieldSteeredDither {
    pub config: FlowFieldConfig,
}

impl FlowFieldSteeredDither {
    pub fn new(config: FlowFieldConfig) -> Self {
        Self { config }
    }

    /// Compute 2D continuous flow field angle $\theta(x, y) \in [-\pi, \pi]$ at pixel $(x, y)$.
    pub fn sample_angle(&self, x: f32, y: f32) -> f32 {
        let s = self.config.scale;
        let nx = (x * s).sin() * (y * s * 0.7).cos();
        let ny = (x * s * 0.8).cos() * (y * s).sin();
        let swirl = ((x * s * 0.5).hypot(y * s * 0.5) * self.config.swirl).sin();
        (nx + ny + swirl + self.config.angle_offset).atan2(1.0)
    }

    /// Apply steered error diffusion along local flow field vectors.
    pub fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        // Residual error buffers (R, G, B in f32)
        let mut err_r = vec![0.0_f32; width * height];
        let mut err_g = vec![0.0_f32; width * height];
        let mut err_b = vec![0.0_f32; width * height];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let in_idx = idx * 3;

                let pr = (input.data[in_idx] as f32 + err_r[idx]).clamp(0.0, 255.0) as u8;
                let pg = (input.data[in_idx + 1] as f32 + err_g[idx]).clamp(0.0, 255.0) as u8;
                let pb = (input.data[in_idx + 2] as f32 + err_b[idx]).clamp(0.0, 255.0) as u8;

                let q = palette.find_nearest(&RgbColor::new(pr, pg, pb));
                out.set_pixel(x as u32, y as u32, q);

                let dr = (pr as f32) - (q.r as f32);
                let dg = (pg as f32) - (q.g as f32);
                let db = (pb as f32) - (q.b as f32);

                // Sample flow field direction vector (vx, vy)
                let angle = self.sample_angle(x as f32, y as f32);
                let vx = angle.cos();
                let vy = angle.sin().abs(); // Constrain forward to avoid non-causal reverse sweeps

                let right_weight = (0.5 + 0.3 * vx).clamp(0.1, 0.9);
                let down_weight = (0.5 + 0.3 * vy).clamp(0.1, 0.9);
                let total = right_weight + down_weight;
                let w_right = (7.0 / 16.0) * (right_weight / total);
                let w_down = (5.0 / 16.0) * (down_weight / total);
                let w_diag_r = 3.0 / 16.0;
                let w_diag_l = 1.0 / 16.0;

                // Propagate forward
                if x + 1 < width {
                    let n = idx + 1;
                    err_r[n] += dr * w_right;
                    err_g[n] += dg * w_right;
                    err_b[n] += db * w_right;
                }
                if y + 1 < height {
                    if x > 0 {
                        let n = (y + 1) * width + (x - 1);
                        err_r[n] += dr * w_diag_l;
                        err_g[n] += dg * w_diag_l;
                        err_b[n] += db * w_diag_l;
                    }
                    let n = (y + 1) * width + x;
                    err_r[n] += dr * w_down;
                    err_g[n] += dg * w_down;
                    err_b[n] += db * w_down;

                    if x + 1 < width {
                        let n = (y + 1) * width + (x + 1);
                        err_r[n] += dr * w_diag_r;
                        err_g[n] += dg * w_diag_r;
                        err_b[n] += db * w_diag_r;
                    }
                }
            }
        }

        Ok(out)
    }
}

impl crate::algorithms::traits::DitherAlgorithm for FlowFieldSteeredDither {
    fn name(&self) -> &'static str {
        "Vector Flow Field"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        self.apply(input, palette)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reaction_diffusion_simulation() {
        let config = ReactionDiffusionConfig { steps: 10, ..Default::default() };
        let mut sim = ReactionDiffusionSimulator::new(32, 32, config);
        sim.step(10);
        let buf = sim.to_image_buffer().expect("buf");
        assert_eq!(buf.width, 32);
        assert_eq!(buf.height, 32);
    }

    #[test]
    fn test_reaction_diffusion_dither() {
        let mut img = ImageBuffer::new(16, 16).expect("img");
        for y in 0..16 {
            for x in 0..16 {
                img.set_pixel(x, y, RgbColor::new(120, 120, 120));
            }
        }
        let pal = Palette::monochrome();
        let dither = ReactionDiffusionDither { config: ReactionDiffusionConfig { steps: 5, ..Default::default() } };
        let out = dither.apply(&img, &pal).expect("dither");
        assert_eq!(out.width, 16);
        assert_eq!(out.height, 16);
    }

    #[test]
    fn test_flow_field_steered_dither() {
        let mut img = ImageBuffer::new(16, 16).expect("img");
        for y in 0..16 {
            for x in 0..16 {
                img.set_pixel(x, y, RgbColor::new(150, 150, 150));
            }
        }
        let pal = Palette::game_boy();
        let steered = FlowFieldSteeredDither::default();
        let out = steered.apply(&img, &pal).expect("steered");
        assert_eq!(out.width, 16);
        assert_eq!(out.height, 16);
    }
}
