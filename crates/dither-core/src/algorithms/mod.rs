pub mod bitmap;
pub mod error_diffusion;
pub mod ordered;
pub mod traits;

pub use bitmap::{FixedThreshold, MeanThreshold, MultiLevelStep, OtsuThreshold};
pub use error_diffusion::{Atkinson, Burkes, FloydSteinberg, JarvisJudiceNinke, Sierra2, Sierra3, SierraLite, Stucki};
pub use ordered::{
    Bayer2x2, Bayer4x4, Bayer8x8, Bayer16x16, Checkerboard45, ClusteredDot4x4, ClusteredDot8x8, DiagonalLines, HorizontalLines,
    InterleavedGradientNoise, VerticalLines,
};
pub use traits::DitherAlgorithm;

/// Canonical classification of the 6 major mathematical algorithm families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmCategory {
    ErrorDiffusion,
    OrderedDithering,
    BitmapConversion,
    HalftoneArrays,
    ModulationSystems,
    CustomArtisticTones,
}

impl AlgorithmCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ErrorDiffusion => "Error-Diffusion",
            Self::OrderedDithering => "Ordered Dithering",
            Self::BitmapConversion => "Bitmap Conversion",
            Self::HalftoneArrays => "Halftone Arrays",
            Self::ModulationSystems => "Modulation Systems",
            Self::CustomArtisticTones => "Custom Artistic Tones",
        }
    }
}
