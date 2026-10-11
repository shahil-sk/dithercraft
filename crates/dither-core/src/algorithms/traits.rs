use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use crate::palette::Palette;

/// Universal interface for all dithering and quantization algorithms.
pub trait DitherAlgorithm: Send + Sync {
    /// Canonical human-readable name of the algorithm.
    fn name(&self) -> &'static str;

    /// Apply the algorithm transforming input image into quantized output buffer.
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError>;
}
