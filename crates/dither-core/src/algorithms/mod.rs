pub mod artistic;
pub mod bitmap;
pub mod custom_matrix;
pub mod error_diffusion;
pub mod halftone;
pub mod modulation;
pub mod ordered;
pub mod spatial_mask;
pub mod traits;
pub mod wavefront;

pub use artistic::{EtchingCrosshatch, MatrixGlitchDispersion, Mezzotint, NewsprintGrain, Stippling, WoodcutSimulation};
pub use bitmap::{FixedThreshold, GaussianLocalThreshold, MeanThreshold, MultiLevelStep, NiblackBinarization, OtsuThreshold, SauvolaBinarization};
pub use custom_matrix::CustomMatrixDither;
pub use error_diffusion::{
    Atkinson, Burkes, DotDiffusion, Fan, FloydSteinberg, JarvisJudiceNinke, MinimizedAverageError, Ostromoukhov, Riemersma, ShiauFan1, ShiauFan2,
    Sierra2, Sierra3, SierraLite, StevensonArce, Stucki, VariableError,
};
pub use halftone::{CircularScreen, CrosshatchHalftone, DiamondDot, EllipticalDot, LineScreen, RotatedScreen};
pub use modulation::{
    AmplitudeModulation, BlueNoiseFM, ProceduralBlueNoise, StochasticScreening, TriangularNoise, UniformRandomNoise, VoidAndCluster,
};
pub use ordered::{
    Bayer2x2, Bayer4x4, Bayer8x8, Bayer16x16, CentralWhitePoint, Checkerboard45, ClusteredDot4x4, ClusteredDot8x8, DiagonalLines, DispersedDot,
    HorizontalLines, InterleavedGradientNoise, Spiral, VerticalLines,
};
pub use spatial_mask::{AlphaMaskDither, TonalBandDither};
pub use traits::DitherAlgorithm;
pub use wavefront::{ParallelWavefrontFloydSteinberg, diffuse_wavefront_parallel};

/// Canonical classification of the 6 major mathematical algorithm families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmCategory {
    ErrorDiffusion,
    OrderedDithering,
    BitmapConversion,
    HalftoneArrays,
    ModulationSystems,
    CustomArtisticTones,
    SpecializedRenderers,
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
            Self::SpecializedRenderers => "Specialized Renderers",
        }
    }
}

/// Metadata descriptor for an algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgorithmInfo {
    pub name: &'static str,
    pub category: AlgorithmCategory,
}

/// Static registry of all 64 algorithms in DitherCraft.
pub static ALL_ALGORITHMS: &[AlgorithmInfo] = &[
    // 1. Error-Diffusion (17)
    AlgorithmInfo { name: "Floyd-Steinberg", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Atkinson", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Sierra-3", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Sierra-2", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Sierra-Lite", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Jarvis-Judice-Ninke", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Stucki", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Burkes", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Stevenson-Arce", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Minimized Average Error", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Fan", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Shiau-Fan 1", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Shiau-Fan 2", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Ostromoukhov", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Variable-Error", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Dot-Diffusion", category: AlgorithmCategory::ErrorDiffusion },
    AlgorithmInfo { name: "Riemersma", category: AlgorithmCategory::ErrorDiffusion },
    // 2. Ordered Dithering (14)
    AlgorithmInfo { name: "Bayer 2x2", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Bayer 4x4", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Bayer 8x8", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Bayer 16x16", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Clustered Dot 4x4", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Clustered Dot 8x8", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Central White Point", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Dispersed Dot", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Spiral", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Horizontal Lines", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Vertical Lines", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Diagonal Lines", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Checkerboard 45°", category: AlgorithmCategory::OrderedDithering },
    AlgorithmInfo { name: "Interleaved Gradient Noise", category: AlgorithmCategory::OrderedDithering },
    // 3. Bitmap Conversion (7)
    AlgorithmInfo { name: "Fixed Threshold", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Adaptive Otsu Threshold", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Mean Threshold", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Gaussian Local Threshold", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Niblack Binarization", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Sauvola Binarization", category: AlgorithmCategory::BitmapConversion },
    AlgorithmInfo { name: "Multi-Level Step", category: AlgorithmCategory::BitmapConversion },
    // 4. Halftone Arrays (9)
    AlgorithmInfo { name: "Rotated Screen 0°", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Rotated Screen 15°", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Rotated Screen 45°", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Rotated Screen 75°", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Circular Screen", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Diamond Dot", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Elliptical Dot", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Line Screen", category: AlgorithmCategory::HalftoneArrays },
    AlgorithmInfo { name: "Crosshatch Halftone", category: AlgorithmCategory::HalftoneArrays },
    // 5. Modulation Systems (7)
    AlgorithmInfo { name: "Void-and-Cluster", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Frequency Modulation (Blue Noise)", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Amplitude Modulation", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Stochastic Screening", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Uniform Random Noise", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Triangular Noise (TPDF)", category: AlgorithmCategory::ModulationSystems },
    AlgorithmInfo { name: "Procedural Blue Noise", category: AlgorithmCategory::ModulationSystems },
    // 6. Custom Artistic Tones (6)
    AlgorithmInfo { name: "Mezzotint", category: AlgorithmCategory::CustomArtisticTones },
    AlgorithmInfo { name: "Stippling", category: AlgorithmCategory::CustomArtisticTones },
    AlgorithmInfo { name: "Woodcut Simulation", category: AlgorithmCategory::CustomArtisticTones },
    AlgorithmInfo { name: "Newsprint Grain", category: AlgorithmCategory::CustomArtisticTones },
    AlgorithmInfo { name: "Etching Crosshatch", category: AlgorithmCategory::CustomArtisticTones },
    AlgorithmInfo { name: "Matrix Glitch Dispersion", category: AlgorithmCategory::CustomArtisticTones },
    // 7. Specialized Renderers & Color Depth (4)
    AlgorithmInfo { name: "CMYK Halftone", category: AlgorithmCategory::SpecializedRenderers },
    AlgorithmInfo { name: "Pop Tone", category: AlgorithmCategory::SpecializedRenderers },
    AlgorithmInfo { name: "Luminosity Mapping", category: AlgorithmCategory::SpecializedRenderers },
    AlgorithmInfo { name: "Color Depth Mismatch", category: AlgorithmCategory::SpecializedRenderers },
];

/// Factory creating a boxed `DitherAlgorithm` instance by canonical or normalized name.
pub fn create_algorithm(name: &str) -> Option<Box<dyn DitherAlgorithm>> {
    let norm: String = name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();

    match norm.as_str() {
        // Error-Diffusion
        "floydsteinberg" => Some(Box::new(FloydSteinberg::default())),
        "atkinson" => Some(Box::new(Atkinson::default())),
        "sierra3" | "sierra" => Some(Box::new(Sierra3::default())),
        "sierra2" => Some(Box::new(Sierra2::default())),
        "sierralite" => Some(Box::new(SierraLite::default())),
        "jarvisjudiceninke" | "jjn" => Some(Box::new(JarvisJudiceNinke::default())),
        "stucki" => Some(Box::new(Stucki::default())),
        "burkes" => Some(Box::new(Burkes::default())),
        "stevensonarce" => Some(Box::new(StevensonArce::default())),
        "minimizedaverageerror" | "mae" => Some(Box::new(MinimizedAverageError::default())),
        "fan" => Some(Box::new(Fan::default())),
        "shiaufan1" => Some(Box::new(ShiauFan1::default())),
        "shiaufan2" => Some(Box::new(ShiauFan2::default())),
        "ostromoukhov" => Some(Box::new(Ostromoukhov::default())),
        "variableerror" => Some(Box::new(VariableError::default())),
        "dotdiffusion" => Some(Box::new(DotDiffusion)),
        "riemersma" => Some(Box::new(Riemersma)),

        // Ordered Dithering
        "bayer2x2" | "bayer2" => Some(Box::new(Bayer2x2)),
        "bayer4x4" | "bayer4" => Some(Box::new(Bayer4x4)),
        "bayer8x8" | "bayer8" | "bayer" => Some(Box::new(Bayer8x8)),
        "bayer16x16" | "bayer16" => Some(Box::new(Bayer16x16)),
        "clustereddot4x4" => Some(Box::new(ClusteredDot4x4)),
        "clustereddot8x8" => Some(Box::new(ClusteredDot8x8)),
        "centralwhitepoint" => Some(Box::new(CentralWhitePoint)),
        "disperseddot" => Some(Box::new(DispersedDot)),
        "spiral" => Some(Box::new(Spiral)),
        "horizontallines" => Some(Box::new(HorizontalLines)),
        "verticallines" => Some(Box::new(VerticalLines)),
        "diagonallines" => Some(Box::new(DiagonalLines)),
        "checkerboard45" | "checkerboard" | "checkerboard45°" => Some(Box::new(Checkerboard45)),
        "interleavedgradientnoise" | "ign" => Some(Box::new(InterleavedGradientNoise)),

        // Bitmap Conversion
        "fixedthreshold" => Some(Box::new(FixedThreshold::default())),
        "adaptiveotsuthreshold" | "adaptiveotsu" | "otsu" => Some(Box::new(OtsuThreshold)),
        "meanthreshold" | "mean" => Some(Box::new(MeanThreshold)),
        "gaussianlocalthreshold" | "gaussianlocal" => Some(Box::new(GaussianLocalThreshold::default())),
        "niblackbinarization" | "niblack" => Some(Box::new(NiblackBinarization::default())),
        "sauvolabinarization" | "sauvola" => Some(Box::new(SauvolaBinarization::default())),
        "multilevelstep" | "multilevel" => Some(Box::new(MultiLevelStep::default())),

        // Halftone Arrays
        "rotatedscreen0" | "rotatedscreen0°" => Some(Box::new(RotatedScreen::screen_0())),
        "rotatedscreen15" | "rotatedscreen15°" => Some(Box::new(RotatedScreen::screen_15())),
        "rotatedscreen45" | "rotatedscreen45°" => Some(Box::new(RotatedScreen::screen_45())),
        "rotatedscreen75" | "rotatedscreen75°" => Some(Box::new(RotatedScreen::screen_75())),
        "circularscreen" => Some(Box::new(CircularScreen::default())),
        "diamonddot" => Some(Box::new(DiamondDot::default())),
        "ellipticaldot" => Some(Box::new(EllipticalDot::default())),
        "linescreen" => Some(Box::new(LineScreen::default())),
        "crosshatchhalftone" => Some(Box::new(CrosshatchHalftone::default())),

        // Modulation Systems
        "voidandcluster" => Some(Box::new(VoidAndCluster)),
        "frequencymodulationbluenoise" | "bluenoisefm" | "bluenoise" | "fm" => Some(Box::new(BlueNoiseFM)),
        "amplitudemodulation" | "am" => Some(Box::new(AmplitudeModulation)),
        "stochasticscreening" | "stochastic" => Some(Box::new(StochasticScreening)),
        "uniformrandomnoise" | "uniformnoise" => Some(Box::new(UniformRandomNoise)),
        "triangularnoisetpdf" | "triangularnoise" | "tpdf" => Some(Box::new(TriangularNoise)),
        "proceduralbluenoise" | "proceduralbluenoisevoidandcluster" => Some(Box::new(ProceduralBlueNoise)),

        // Custom Artistic Tones
        "mezzotint" => Some(Box::new(Mezzotint::default())),
        "stippling" => Some(Box::new(Stippling::default())),
        "woodcutsimulation" | "woodcut" => Some(Box::new(WoodcutSimulation::default())),
        "newsprintgrain" | "newsprint" => Some(Box::new(NewsprintGrain::default())),
        "etchingcrosshatch" | "etching" => Some(Box::new(EtchingCrosshatch::default())),
        "matrixglitchdispersion" | "matrixglitch" => Some(Box::new(MatrixGlitchDispersion::default())),

        // Specialized Renderers
        "cmykhalftone" | "cmyk" => Some(Box::new(crate::renderers::CmykHalftone::default())),
        "poptone" => Some(Box::new(crate::renderers::PopTone::default())),
        "luminositymapping" => Some(Box::new(crate::color_depth::LuminosityMapping::default())),
        "colordepthmismatch" | "mismatch" => Some(Box::new(crate::color_depth::MismatchMode::default())),
        "tonalbandmultidither" | "tonalband" => {
            Some(Box::new(TonalBandDither::new(85, 170, 16, Box::new(Bayer8x8), Box::new(FloydSteinberg::default()), Box::new(Atkinson::default()))))
        }
        "custommatrix" | "custom" => Some(Box::new(CustomMatrixDither::default())),
        "morphogenesis" | "reactiondiffusion" => Some(Box::new(crate::generative::ReactionDiffusionDither::default())),
        "vectorflowfield" | "flowfield" => Some(Box::new(crate::generative::FlowFieldSteeredDither::default())),
        "wavefront" | "parallelwavefront" | "wavefrontfloydsteinberg" => Some(Box::new(ParallelWavefrontFloydSteinberg::default())),

        _ => None,
    }
}
