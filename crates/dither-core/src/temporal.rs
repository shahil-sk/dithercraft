use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// 9 CRT and LCD Temporal Noise Patterns for multi-frame video and animation simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalNoisePattern {
    CrtScanBeamJitter,
    PhosphorGrainHum,
    InterlacedFieldShimmer,
    LcdSubpixelGridDrift,
    VhsTapePhaseNoise,
    ThermalAnalogDrift,
    StaticHashWhiteNoise,
    BayerPhaseStepping,
    BlueNoiseTemporalJitter,
}

impl TemporalNoisePattern {
    pub const ALL: [Self; 9] = [
        Self::CrtScanBeamJitter,
        Self::PhosphorGrainHum,
        Self::InterlacedFieldShimmer,
        Self::LcdSubpixelGridDrift,
        Self::VhsTapePhaseNoise,
        Self::ThermalAnalogDrift,
        Self::StaticHashWhiteNoise,
        Self::BayerPhaseStepping,
        Self::BlueNoiseTemporalJitter,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::CrtScanBeamJitter => "CRT Scan Beam Jitter",
            Self::PhosphorGrainHum => "Phosphor Grain Hum",
            Self::InterlacedFieldShimmer => "Interlaced Field Shimmer",
            Self::LcdSubpixelGridDrift => "LCD Subpixel Grid Drift",
            Self::VhsTapePhaseNoise => "VHS Tape Phase Noise",
            Self::ThermalAnalogDrift => "Thermal Analog Drift",
            Self::StaticHashWhiteNoise => "Static Hash White Noise",
            Self::BayerPhaseStepping => "Bayer Phase Stepping",
            Self::BlueNoiseTemporalJitter => "Blue Noise Temporal Jitter",
        }
    }

    /// Apply temporal pattern at frame index `frame_idx` with given `intensity` (0.0 to 1.0).
    pub fn apply(&self, input: &ImageBuffer, frame_idx: u32, intensity: f32) -> Result<ImageBuffer, DitherError> {
        let mut out = input.clone();
        let width = input.width as usize;
        let height = input.height as usize;
        let t = frame_idx as f32;

        match self {
            Self::CrtScanBeamJitter => {
                // High-voltage deflection jitter: periodic horizontal shift coupled with frame
                let beam_phase = (t * 0.4).sin();
                let shift = (beam_phase * 4.0 * intensity) as i32;
                for y in 0..height {
                    let y_mod = ((y as f32 * 0.1 + t).sin() * 2.0 * intensity) as i32;
                    let total_shift = shift + y_mod;
                    for x in 0..width {
                        let src_x = (x as i32 + total_shift).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        let dst_idx = (y * width + x) * 3;
                        out.data[dst_idx] = input.data[src_idx];
                        out.data[dst_idx + 1] = input.data[src_idx + 1];
                        out.data[dst_idx + 2] = input.data[src_idx + 2];
                    }
                }
            }

            Self::PhosphorGrainHum => {
                // 50/60 Hz mains hum modulating phosphor brightness across vertical raster
                let hum = ((t * 0.5 + 0.1).sin() * 0.5 + 0.5) * intensity * 24.0;
                out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row)| {
                    let band = ((y as f32 * 0.05 + t * 0.2).sin() * hum) as i32;
                    for chunk in row.chunks_exact_mut(3) {
                        chunk[0] = (chunk[0] as i32 + band).clamp(0, 255) as u8;
                        chunk[1] = (chunk[1] as i32 + band).clamp(0, 255) as u8;
                        chunk[2] = (chunk[2] as i32 + band).clamp(0, 255) as u8;
                    }
                });
            }

            Self::InterlacedFieldShimmer => {
                // Alternate field brightness offset between even and odd frames
                let field_offset = if frame_idx.is_multiple_of(2) { 1 } else { 0 };
                let delta = (intensity * 30.0) as i32;
                out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row)| {
                    if (y % 2) == field_offset {
                        for chunk in row.chunks_exact_mut(3) {
                            chunk[0] = (chunk[0] as i32 + delta).clamp(0, 255) as u8;
                            chunk[1] = (chunk[1] as i32 + delta).clamp(0, 255) as u8;
                            chunk[2] = (chunk[2] as i32 + delta).clamp(0, 255) as u8;
                        }
                    }
                });
            }

            Self::LcdSubpixelGridDrift => {
                // Subpixel crystal response latency drift
                let drift_col = ((t * 0.1).cos() * 2.0 * intensity) as i32;
                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        let src_x = (x as i32 + drift_col).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        out.data[idx] = input.data[src_idx]; // R from shifted
                    }
                }
            }

            Self::VhsTapePhaseNoise => {
                // Lower tracking head noise band
                let head_band_y = ((t * 12.0) as usize) % height;
                let band_height = (12.0 * intensity) as usize;
                for y in head_band_y..(head_band_y + band_height).min(height) {
                    let tear = ((y as f32 * 0.8).sin() * 20.0 * intensity) as i32;
                    for x in 0..width {
                        let src_x = (x as i32 + tear).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        let dst_idx = (y * width + x) * 3;
                        out.data[dst_idx] = input.data[src_idx];
                        out.data[dst_idx + 1] = input.data[src_idx + 1];
                        out.data[dst_idx + 2] = input.data[src_idx + 2];
                    }
                }
            }

            Self::ThermalAnalogDrift => {
                // Low frequency thermal color temp drift
                let r_bias = ((t * 0.05).sin() * 15.0 * intensity) as i32;
                let b_bias = (-(t * 0.05).sin() * 15.0 * intensity) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + r_bias).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + b_bias).clamp(0, 255) as u8;
                }
            }

            Self::StaticHashWhiteNoise => {
                // Pure high-frequency pseudo-random static hash
                let mut seed = ((frame_idx as u64) << 32) ^ 0x9E3779B97F4A7C15;
                let range = (intensity * 40.0) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let noise = ((seed >> 48) as i32 % (range * 2 + 1)) - range;
                    chunk[0] = (chunk[0] as i32 + noise).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + noise).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + noise).clamp(0, 255) as u8;
                }
            }

            Self::BayerPhaseStepping => {
                // Temporal rotation of Bayer dither matrix phase
                let phase_shift = ((frame_idx % 4) * 2) as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + phase_shift).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + phase_shift).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + phase_shift).clamp(0, 255) as u8;
                }
            }

            Self::BlueNoiseTemporalJitter => {
                // Blue noise golden-ratio temporal stepping (Roberts 2014)
                let phi_step = ((frame_idx as f32 * 0.618_034) % 1.0 - 0.5) * intensity * 30.0;
                let step_i = phi_step as i32;
                for chunk in out.data.chunks_exact_mut(3) {
                    chunk[0] = (chunk[0] as i32 + step_i).clamp(0, 255) as u8;
                    chunk[1] = (chunk[1] as i32 + step_i).clamp(0, 255) as u8;
                    chunk[2] = (chunk[2] as i32 + step_i).clamp(0, 255) as u8;
                }
            }
        }

        Ok(out)
    }
}

/// Interpolation and easing modes between timeline keyframes.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
    SmoothStep,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
    CubicBezier(f32, f32, f32, f32),
}

impl Interpolation {
    /// Evaluate normalized interpolation curve factor for $t \in [0, 1]$.
    pub fn ease(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Self::Step => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::Bounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
            Self::Elastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c4 = std::f32::consts::TAU / 3.0;
                    -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
                }
            }
            Self::CubicBezier(p1x, p1y, p2x, p2y) => Self::eval_bezier(p1x, p1y, p2x, p2y, t),
        }
    }

    /// Cubic Bézier curve evaluation using numerical interval inversion.
    pub fn eval_bezier(p1x: f32, p1y: f32, p2x: f32, p2y: f32, target_x: f32) -> f32 {
        if target_x <= 0.0 {
            return 0.0;
        }
        if target_x >= 1.0 {
            return 1.0;
        }

        let sample_x = |u: f32| -> f32 {
            let u2 = u * u;
            let u3 = u2 * u;
            let inv = 1.0 - u;
            let inv2 = inv * inv;
            3.0 * inv2 * u * p1x + 3.0 * inv * u2 * p2x + u3
        };

        let sample_y = |u: f32| -> f32 {
            let u2 = u * u;
            let u3 = u2 * u;
            let inv = 1.0 - u;
            let inv2 = inv * inv;
            3.0 * inv2 * u * p1y + 3.0 * inv * u2 * p2y + u3
        };

        let mut low = 0.0_f32;
        let mut high = 1.0_f32;
        let mut u = target_x;

        for _ in 0..12 {
            let x = sample_x(u);
            if (x - target_x).abs() < 1e-4 {
                break;
            }
            if x < target_x {
                low = u;
            } else {
                high = u;
            }
            u = 0.5 * (low + high);
        }

        sample_y(u)
    }
}

/// A parameter keyframe at a specific animation frame index.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    pub frame: u32,
    pub value: f32,
}

impl Keyframe {
    pub const fn new(frame: u32, value: f32) -> Self {
        Self { frame, value }
    }
}

/// Timeline animation track for interpolating parameters over time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelineTrack {
    pub name: String,
    pub interpolation: Interpolation,
    pub keyframes: Vec<Keyframe>,
    pub default_value: f32,
}

impl TimelineTrack {
    pub fn new(name: impl Into<String>, default_value: f32) -> Self {
        Self { name: name.into(), interpolation: Interpolation::Linear, keyframes: Vec::new(), default_value }
    }

    /// Add a keyframe, maintaining ascending order by frame index.
    pub fn add_keyframe(&mut self, frame: u32, value: f32) {
        if let Some(pos) = self.keyframes.iter().position(|k| k.frame == frame) {
            self.keyframes[pos].value = value;
        } else {
            let pos = self.keyframes.partition_point(|k| k.frame < frame);
            self.keyframes.insert(pos, Keyframe::new(frame, value));
        }
    }

    /// Evaluate the track value at a given frame index.
    pub fn eval(&self, frame: u32) -> f32 {
        if self.keyframes.is_empty() {
            return self.default_value;
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].value;
        }

        let first = &self.keyframes[0];
        if frame <= first.frame {
            return first.value;
        }

        let last = &self.keyframes[self.keyframes.len() - 1];
        if frame >= last.frame {
            return last.value;
        }

        let idx = self.keyframes.partition_point(|k| k.frame <= frame);
        let k0 = &self.keyframes[idx - 1];
        let k1 = &self.keyframes[idx];

        let span = (k1.frame - k0.frame) as f32;
        if span <= 0.0 {
            return k0.value;
        }

        let t = ((frame - k0.frame) as f32 / span).clamp(0.0, 1.0);
        let factor = self.interpolation.ease(t);
        k0.value + factor * (k1.value - k0.value)
    }
}

/// Low-Frequency Oscillator (LFO) waveforms for automated parameter modulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LfoWaveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
    SampleAndHold,
}

/// Automated LFO parameter modulator over time.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LfoModulator {
    pub waveform: LfoWaveform,
    pub frequency: f32,
    pub amplitude: f32,
    pub offset: f32,
    pub phase: f32,
}

impl LfoModulator {
    pub const fn new(waveform: LfoWaveform, frequency: f32, amplitude: f32, offset: f32) -> Self {
        Self { waveform, frequency, amplitude, offset, phase: 0.0 }
    }

    /// Evaluate the modulated value at frame `frame_idx`.
    pub fn eval(&self, frame_idx: u32) -> f32 {
        let t = frame_idx as f32;
        let cycles = (t * self.frequency + self.phase).rem_euclid(1.0);

        let wave = match self.waveform {
            LfoWaveform::Sine => (cycles * std::f32::consts::TAU).sin(),
            LfoWaveform::Triangle => {
                if cycles < 0.5 {
                    cycles * 4.0 - 1.0
                } else {
                    3.0 - cycles * 4.0
                }
            }
            LfoWaveform::Sawtooth => cycles * 2.0 - 1.0,
            LfoWaveform::Square => {
                if cycles < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoWaveform::SampleAndHold => {
                let step = (t * self.frequency + self.phase).floor() as u64;
                let hash = step.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((hash >> 32) as f32 / u32::MAX as f32) * 2.0 - 1.0
            }
        };

        self.offset + wave * self.amplitude
    }
}

/// A single animated frame with image data and display delay in milliseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedFrame {
    pub buffer: ImageBuffer,
    pub delay_ms: u32,
}

impl AnimatedFrame {
    pub const fn new(buffer: ImageBuffer, delay_ms: u32) -> Self {
        Self { buffer, delay_ms }
    }
}

/// Multi-frame animation sequence supporting playback, timing, and parallel processing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimationSequence {
    pub frames: Vec<AnimatedFrame>,
}

impl AnimationSequence {
    pub const fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn with_frames(frames: Vec<AnimatedFrame>) -> Self {
        Self { frames }
    }

    pub fn add_frame(&mut self, buffer: ImageBuffer, delay_ms: u32) {
        self.frames.push(AnimatedFrame::new(buffer, delay_ms));
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Total duration of the animation sequence in milliseconds.
    pub fn total_duration_ms(&self) -> u32 {
        self.frames.iter().map(|f| f.delay_ms).sum()
    }

    /// Retrieve frame index corresponding to a given time offset in milliseconds.
    pub fn frame_index_at_time(&self, mut time_ms: u32, looping: bool) -> usize {
        if self.frames.is_empty() {
            return 0;
        }

        let total = self.total_duration_ms();
        if total == 0 {
            return 0;
        }

        if looping {
            time_ms %= total;
        } else if time_ms >= total {
            return self.frames.len() - 1;
        }

        let mut acc = 0;
        for (i, f) in self.frames.iter().enumerate() {
            acc += f.delay_ms;
            if time_ms < acc {
                return i;
            }
        }

        self.frames.len().saturating_sub(1)
    }

    /// Process each frame in the sequence in parallel across Rayon worker threads.
    pub fn process_frames<F>(&self, process_fn: F) -> Result<Self, DitherError>
    where
        F: Fn(&ImageBuffer, usize) -> Result<ImageBuffer, DitherError> + Sync + Send,
    {
        let processed_frames: Result<Vec<AnimatedFrame>, DitherError> = self
            .frames
            .par_iter()
            .enumerate()
            .map(|(idx, frame)| {
                let out_buf = process_fn(&frame.buffer, idx)?;
                Ok(AnimatedFrame { buffer: out_buf, delay_ms: frame.delay_ms })
            })
            .collect();

        Ok(Self { frames: processed_frames? })
    }
}

/// Cinematic, vintage, and animation frame rate decimation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CadenceMode {
    /// Full frame rate (no decimation)
    #[default]
    Passthrough,
    /// 24 FPS theatrical cinematic cadence with 3:2 pulldown emulation
    Cinematic24,
    /// 12 FPS "on-twos" anime and traditional hand-drawn animation cadence
    AnimeOnTwos12,
    /// 8 FPS crunchy early computer / 8-bit game cadence
    RetroCrunch8,
    /// 4 FPS claymation / stop-motion cadence
    StopMotion4,
    /// Interlaced field cadence (alternating even/odd scanlines on successive frames)
    InterlacedFields,
}

impl CadenceMode {
    /// Compute which source frame index should be displayed at timeline step `frame_idx`
    /// given a nominal sequence frame rate (e.g. 60 or 30 fps).
    pub fn map_frame_index(&self, frame_idx: usize, nominal_fps: u32) -> usize {
        match self {
            Self::Passthrough => frame_idx,
            Self::Cinematic24 => {
                let cycle = frame_idx % 5;
                let base = (frame_idx / 5) * 2;
                if cycle < 3 { base } else { base + 1 }
            }
            Self::AnimeOnTwos12 => {
                let step = (nominal_fps / 12).max(2) as usize;
                (frame_idx / step) * step
            }
            Self::RetroCrunch8 => {
                let step = (nominal_fps / 8).max(1) as usize;
                (frame_idx / step) * step
            }
            Self::StopMotion4 => {
                let step = (nominal_fps / 4).max(1) as usize;
                (frame_idx / step) * step
            }
            Self::InterlacedFields => frame_idx,
        }
    }

    /// Apply interlaced field blanking or line hold if in InterlacedFields mode.
    pub fn apply_interlacing(&self, frame: &ImageBuffer, frame_idx: usize) -> ImageBuffer {
        if *self != Self::InterlacedFields {
            return frame.clone();
        }
        let mut out = frame.clone();
        let field_parity = frame_idx % 2;
        for y in 0..out.height {
            if (y as usize % 2) != field_parity {
                for x in 0..out.width {
                    if let Some(px) = out.get_pixel(x, y) {
                        out.set_pixel(x, y, RgbColor::new(px.r / 3, px.g / 3, px.b / 3));
                    }
                }
            }
        }
        out
    }
}

/// Onion skinning / multi-frame ghosting for rotoscoping and animation auditing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OnionSkinConfig {
    pub prev_frames: usize,
    pub next_frames: usize,
    pub opacity: f32,
    pub tint_previous: [u8; 3],
    pub tint_next: [u8; 3],
}

impl Default for OnionSkinConfig {
    fn default() -> Self {
        Self { prev_frames: 1, next_frames: 1, opacity: 0.35, tint_previous: [255, 60, 60], tint_next: [60, 220, 255] }
    }
}

/// Composite adjacent frames as transparent ghost overlays for rotoscoping and animation auditing.
pub fn render_onion_skin<B: std::borrow::Borrow<ImageBuffer>>(
    frames: &[B],
    current_idx: usize,
    config: &OnionSkinConfig,
) -> Result<ImageBuffer, DitherError> {
    if frames.is_empty() {
        return Err(DitherError::InvalidDimensions { width: 0, height: 0, expected_len: 0, actual_len: 0 });
    }

    let current = frames[current_idx.min(frames.len() - 1)].borrow();
    let width = current.width;
    let height = current.height;
    let mut out = current.clone();

    let opacity = config.opacity.clamp(0.0, 1.0);
    if opacity <= 0.001 {
        return Ok(out);
    }

    // Blend previous frames (tinted red by default)
    let start_prev = current_idx.saturating_sub(config.prev_frames);
    for (offset, item) in frames[start_prev..current_idx].iter().enumerate() {
        let prev = item.borrow();
        if prev.width != width || prev.height != height {
            continue;
        }
        let distance = current_idx - (start_prev + offset);
        let weight = opacity / distance as f32;
        let tint = config.tint_previous;

        for y in 0..height {
            for x in 0..width {
                if let (Some(cur_px), Some(prev_px)) = (out.get_pixel(x, y), prev.get_pixel(x, y)) {
                    let lum = prev_px.luminance() as f32 / 255.0;
                    let pr = tint[0] as f32 * lum;
                    let pg = tint[1] as f32 * lum;
                    let pb = tint[2] as f32 * lum;

                    let r = ((1.0 - weight) * cur_px.r as f32 + weight * pr).clamp(0.0, 255.0) as u8;
                    let g = ((1.0 - weight) * cur_px.g as f32 + weight * pg).clamp(0.0, 255.0) as u8;
                    let b = ((1.0 - weight) * cur_px.b as f32 + weight * pb).clamp(0.0, 255.0) as u8;
                    out.set_pixel(x, y, RgbColor::new(r, g, b));
                }
            }
        }
    }

    // Blend next frames (tinted cyan by default)
    let start_next = (current_idx + 1).min(frames.len());
    let end_next = (current_idx + 1 + config.next_frames).min(frames.len());
    for (offset, item) in frames[start_next..end_next].iter().enumerate() {
        let next = item.borrow();
        if next.width != width || next.height != height {
            continue;
        }
        let distance = offset + 1;
        let weight = opacity / distance as f32;
        let tint = config.tint_next;

        for y in 0..height {
            for x in 0..width {
                if let (Some(cur_px), Some(next_px)) = (out.get_pixel(x, y), next.get_pixel(x, y)) {
                    let lum = next_px.luminance() as f32 / 255.0;
                    let nr = tint[0] as f32 * lum;
                    let ng = tint[1] as f32 * lum;
                    let nb = tint[2] as f32 * lum;

                    let r = ((1.0 - weight) * cur_px.r as f32 + weight * nr).clamp(0.0, 255.0) as u8;
                    let g = ((1.0 - weight) * cur_px.g as f32 + weight * ng).clamp(0.0, 255.0) as u8;
                    let b = ((1.0 - weight) * cur_px.b as f32 + weight * nb).clamp(0.0, 255.0) as u8;
                    out.set_pixel(x, y, RgbColor::new(r, g, b));
                }
            }
        }
    }

    Ok(out)
}

/// Configuration for inter-frame temporal coherence and flicker suppression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TemporalCoherenceConfig {
    /// Residual error damping factor (0.0 = no temporal blending, 1.0 = maximum flicker suppression).
    pub damping: f32,
    /// Normalized motion threshold below which pixels are treated as static/quasi-static (0.0 to 1.0).
    pub motion_threshold: f32,
}

impl Default for TemporalCoherenceConfig {
    fn default() -> Self {
        Self { damping: 0.65, motion_threshold: 0.06 }
    }
}

/// Temporal coherence filter for video frames and animations.
///
/// Dampens high-frequency dither flicker on static backgrounds while maintaining crispness
/// and instant responsiveness on moving subjects without ghosting or smear.
#[derive(Debug, Clone, Default)]
pub struct TemporalCoherenceFilter {
    pub config: TemporalCoherenceConfig,
}

impl TemporalCoherenceFilter {
    pub fn new(config: TemporalCoherenceConfig) -> Self {
        Self { config }
    }

    /// Blend current frame with previous frame based on local motion detection.
    pub fn apply(
        &self,
        prev_source: &ImageBuffer,
        curr_source: &ImageBuffer,
        prev_output: &ImageBuffer,
        curr_output: &ImageBuffer,
    ) -> Result<ImageBuffer, DitherError> {
        if curr_source.width != prev_source.width
            || curr_source.height != prev_source.height
            || curr_output.width != curr_source.width
            || curr_output.height != curr_source.height
            || prev_output.width != prev_source.width
            || prev_output.height != prev_source.height
        {
            return Err(DitherError::InvalidParameter("Frame dimensions must match for temporal coherence".into()));
        }

        let width = curr_source.width as usize;
        let _height = curr_source.height as usize;
        let mut out = ImageBuffer::new(curr_source.width, curr_source.height)?;

        let damping = self.config.damping.clamp(0.0, 1.0);
        let threshold = self.config.motion_threshold.clamp(0.001, 1.0);

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(y, row_out)| {
            let row_offset = y * width * 3;
            let prev_src_row = &prev_source.data[row_offset..row_offset + width * 3];
            let curr_src_row = &curr_source.data[row_offset..row_offset + width * 3];
            let prev_out_row = &prev_output.data[row_offset..row_offset + width * 3];
            let curr_out_row = &curr_output.data[row_offset..row_offset + width * 3];

            for x in 0..width {
                let idx = x * 3;
                let dr = (curr_src_row[idx] as i32 - prev_src_row[idx] as i32).abs();
                let dg = (curr_src_row[idx + 1] as i32 - prev_src_row[idx + 1] as i32).abs();
                let db = (curr_src_row[idx + 2] as i32 - prev_src_row[idx + 2] as i32).abs();
                let motion = (dr + dg + db) as f32 / (3.0 * 255.0);

                if motion < threshold {
                    let factor = damping * (1.0 - motion / threshold);
                    let one_minus = 1.0 - factor;

                    row_out[idx] = ((curr_out_row[idx] as f32 * one_minus) + (prev_out_row[idx] as f32 * factor)).round().clamp(0.0, 255.0) as u8;
                    row_out[idx + 1] =
                        ((curr_out_row[idx + 1] as f32 * one_minus) + (prev_out_row[idx + 1] as f32 * factor)).round().clamp(0.0, 255.0) as u8;
                    row_out[idx + 2] =
                        ((curr_out_row[idx + 2] as f32 * one_minus) + (prev_out_row[idx + 2] as f32 * factor)).round().clamp(0.0, 255.0) as u8;
                } else {
                    row_out[idx] = curr_out_row[idx];
                    row_out[idx + 1] = curr_out_row[idx + 1];
                    row_out[idx + 2] = curr_out_row[idx + 2];
                }
            }
        });

        Ok(out)
    }

    /// Apply temporal coherence sequentially across a list of frames.
    pub fn apply_sequence(&self, source_frames: &[ImageBuffer], dithered_frames: &[ImageBuffer]) -> Result<Vec<ImageBuffer>, DitherError> {
        if source_frames.is_empty() || dithered_frames.is_empty() {
            return Ok(Vec::new());
        }
        if source_frames.len() != dithered_frames.len() {
            return Err(DitherError::InvalidParameter("Source and dithered frame counts must match".into()));
        }

        let mut out_frames = Vec::with_capacity(dithered_frames.len());
        out_frames.push(dithered_frames[0].clone());

        for i in 1..dithered_frames.len() {
            let filtered = self.apply(&source_frames[i - 1], &source_frames[i], &out_frames[i - 1], &dithered_frames[i])?;
            out_frames.push(filtered);
        }

        Ok(out_frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cadence_mode_mapping_and_interlacing() {
        let mode_passthrough = CadenceMode::Passthrough;
        assert_eq!(mode_passthrough.map_frame_index(7, 30), 7);

        let mode_anime = CadenceMode::AnimeOnTwos12;
        assert_eq!(mode_anime.map_frame_index(0, 24), 0);
        assert_eq!(mode_anime.map_frame_index(1, 24), 0);
        assert_eq!(mode_anime.map_frame_index(2, 24), 2);
        assert_eq!(mode_anime.map_frame_index(3, 24), 2);

        let mode_cinematic = CadenceMode::Cinematic24;
        assert_eq!(mode_cinematic.map_frame_index(0, 60), 0);
        assert_eq!(mode_cinematic.map_frame_index(1, 60), 0);
        assert_eq!(mode_cinematic.map_frame_index(2, 60), 0);
        assert_eq!(mode_cinematic.map_frame_index(3, 60), 1);
        assert_eq!(mode_cinematic.map_frame_index(4, 60), 1);

        let mut buf = ImageBuffer::new(4, 4).expect("buf");
        for y in 0..4 {
            for x in 0..4 {
                buf.set_pixel(x, y, RgbColor::new(240, 240, 240));
            }
        }
        let interlaced = CadenceMode::InterlacedFields.apply_interlacing(&buf, 0);
        // Field parity 0: even scanlines active (y=0, y=2 full), odd dim (y=1, y=3 dimmed)
        assert_eq!(interlaced.get_pixel(0, 0).expect("px").r, 240);
        assert_eq!(interlaced.get_pixel(0, 1).expect("px").r, 80);
    }

    #[test]
    fn test_onion_skinning_overlay() {
        let mut f0 = ImageBuffer::new(4, 4).expect("f0");
        let mut f1 = ImageBuffer::new(4, 4).expect("f1");
        let mut f2 = ImageBuffer::new(4, 4).expect("f2");

        for y in 0..4 {
            for x in 0..4 {
                f0.set_pixel(x, y, RgbColor::new(255, 255, 255));
                f1.set_pixel(x, y, RgbColor::new(100, 100, 100));
                f2.set_pixel(x, y, RgbColor::new(255, 255, 255));
            }
        }

        let frames = vec![f0, f1, f2];
        let cfg = OnionSkinConfig::default();
        let skinned = render_onion_skin(&frames, 1, &cfg).expect("onion skin");
        assert_eq!(skinned.width, 4);
        assert_eq!(skinned.height, 4);
    }

    #[test]
    fn test_easing_curves_and_cubic_bezier() {
        assert_eq!(Interpolation::Linear.ease(0.0), 0.0);
        assert_eq!(Interpolation::Linear.ease(1.0), 1.0);
        assert_eq!(Interpolation::Linear.ease(0.5), 0.5);

        assert_eq!(Interpolation::Step.ease(0.4), 0.0);
        assert_eq!(Interpolation::Step.ease(1.0), 1.0);

        assert_eq!(Interpolation::EaseIn.ease(0.5), 0.125);
        assert_eq!(Interpolation::EaseOut.ease(0.5), 0.875);

        let bezier = Interpolation::CubicBezier(0.42, 0.0, 0.58, 1.0);
        assert!((bezier.ease(0.5) - 0.5).abs() < 0.05);

        let mut track = TimelineTrack::new("test", 0.0);
        track.interpolation = Interpolation::EaseInOut;
        track.add_keyframe(0, 0.0);
        track.add_keyframe(10, 100.0);
        assert_eq!(track.eval(0), 0.0);
        assert_eq!(track.eval(10), 100.0);
        assert_eq!(track.eval(5), 50.0);
    }

    #[test]
    fn test_temporal_coherence_filter() {
        // Create 2 identical source frames (static scene)
        let mut s0 = ImageBuffer::new(2, 2).expect("s0");
        let mut s1 = ImageBuffer::new(2, 2).expect("s1");
        for y in 0..2 {
            for x in 0..2 {
                s0.set_pixel(x, y, RgbColor::new(128, 128, 128));
                s1.set_pixel(x, y, RgbColor::new(128, 128, 128));
            }
        }

        // Two dither outputs with high-frequency dither flicker
        let mut o0 = ImageBuffer::new(2, 2).expect("o0");
        let mut o1 = ImageBuffer::new(2, 2).expect("o1");
        o0.set_pixel(0, 0, RgbColor::new(0, 0, 0));
        o0.set_pixel(1, 0, RgbColor::new(255, 255, 255));
        o1.set_pixel(0, 0, RgbColor::new(255, 255, 255)); // flickered
        o1.set_pixel(1, 0, RgbColor::new(0, 0, 0)); // flickered

        let filter = TemporalCoherenceFilter::new(TemporalCoherenceConfig { damping: 0.8, motion_threshold: 0.1 });

        let stabilized = filter.apply(&s0, &s1, &o0, &o1).expect("stabilized");
        // Because source was identical (motion = 0), damping = 0.8 blends heavily toward previous output
        let p00 = stabilized.get_pixel(0, 0).expect("px");
        // curr was 255, prev was 0. With 0.8 blend towards prev: 255 * 0.2 + 0 * 0.8 = 51
        assert!(p00.r < 100);

        // Now test moving subject (pixel (1, 1) has high source motion)
        s1.set_pixel(1, 1, RgbColor::new(255, 255, 255));
        s0.set_pixel(1, 1, RgbColor::new(0, 0, 0));
        o1.set_pixel(1, 1, RgbColor::new(200, 200, 200));
        o0.set_pixel(1, 1, RgbColor::new(10, 10, 10));

        let motion_out = filter.apply(&s0, &s1, &o0, &o1).expect("motion");
        // Because motion exceeds threshold (255 delta), current output is retained without blending
        assert_eq!(motion_out.get_pixel(1, 1).expect("px").r, 200);

        // Test apply_sequence
        let seq = filter.apply_sequence(&[s0.clone(), s1.clone()], &[o0.clone(), o1.clone()]).expect("seq");
        assert_eq!(seq.len(), 2);
    }
}
