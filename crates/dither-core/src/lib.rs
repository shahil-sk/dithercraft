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
pub mod audio;
pub mod buffer;
pub mod color;
pub mod color_depth;
pub mod effects;
pub mod error;
pub mod ffi;
pub mod generative;
pub mod lut;
pub mod palette;
pub mod renderers;
pub mod shaders;
pub mod simd;
pub mod temporal;
pub mod vector;
pub mod video;

pub use algorithms::*;
pub use audio::{AudioSpectrum, AudioTrack};
pub use buffer::{HilbertCurve, ImageBuffer, ImageBufferPool};
pub use color::{ColorHarmony, CvdMode, LabColor, OklabColor, OklchColor, RgbColor};
pub use color_depth::{LuminosityMapping, MismatchMode};
pub use effects::{
    BlendMode, ChromaticAberration, EffectKind, EffectNode, EffectStack, EpsilonGlow, GaussianBlur, JpegGlitchCore, JpegSubMode, Sharpen,
    VintageCrtSimulation,
};
pub use error::DitherError;
pub use generative::{
    FlowFieldConfig, FlowFieldSteeredDither, ReactionDiffusionConfig, ReactionDiffusionDither, ReactionDiffusionMorphology,
    ReactionDiffusionSimulator,
};
pub use lut::Cube3dLut;
pub use palette::{ColorMetric, Palette, PaletteKdTree, Swatch};
pub use renderers::{CmykChannel, CmykHalftone, CmykSeparationPlates, HalftoneDotShape, PopTone};
pub use simd::{quantize_batch_8, quantize_buffer_simd};
pub use temporal::{
    AnimatedFrame, AnimationSequence, CadenceMode, Interpolation, Keyframe, LfoModulator, LfoWaveform, OnionSkinConfig, TemporalNoisePattern,
    TimelineTrack, render_onion_skin,
};
pub use vector::{
    BlackVectorIsolation, CubicBezierSegment, GCodeConfig, GCodeExporter, HatchStyle, MultiColorSvgSeparation, Point2D, SpotColorPlateSeparation,
    ToolpathStroke, export_smooth_bezier_svg, fit_cubic_spline, rdp_simplify, spline_to_svg_path, visvalingam_simplify,
};
pub use video::{Mp4Demuxer, Mp4Sample, Mp4Track, VideoCodec, WebmDemuxer, WebmSample, WebmTrack, WebpAnimFrame, WebpAnimationMuxer};
