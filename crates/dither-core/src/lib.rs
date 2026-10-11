//! DitherCraft Core: Pure DSP mathematical engine for dithering, palette quantization,
//! vintage CRT shaders, glitch effects, and vector isolation.
//!
//! # Architecture
//! - Zero dependencies on UI, egui, eframe, or windowing libraries.
//! - Non-blocking thread-safe design (Send + Sync).
//! - Zero dynamic heap allocations in inner hot pixel processing loops.
//! - No panics: all fallible functions return `Result<T, DitherError>`.

#![allow(clippy::chunks_exact_to_as_chunks)]

pub mod algorithms;
pub mod buffer;
pub mod color;
pub mod color_depth;
pub mod effects;
pub mod error;
pub mod palette;
pub mod renderers;
pub mod temporal;
pub mod vector;

pub use algorithms::*;
pub use buffer::ImageBuffer;
pub use color::{LabColor, OklabColor, RgbColor};
pub use color_depth::{LuminosityMapping, MismatchMode};
pub use effects::{ChromaticAberration, EpsilonGlow, GaussianBlur, JpegGlitchCore, JpegSubMode, Sharpen, VintageCrtSimulation};
pub use error::DitherError;
pub use palette::{ColorMetric, Palette, PaletteKdTree, Swatch};
pub use renderers::{CmykHalftone, PopTone};
pub use temporal::{AnimatedFrame, AnimationSequence, Interpolation, Keyframe, LfoModulator, LfoWaveform, TemporalNoisePattern, TimelineTrack};
pub use vector::BlackVectorIsolation;
