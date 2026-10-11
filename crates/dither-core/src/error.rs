use std::fmt;

/// Errors produced during image processing, color quantization, and dithering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DitherError {
    /// Provided buffer dimensions do not match data byte length.
    InvalidDimensions { width: u32, height: u32, expected_len: usize, actual_len: usize },
    /// The target color palette has no colors.
    EmptyPalette,
    /// An algorithmic parameter is out of valid bounds.
    InvalidParameter(String),
    /// Image dimensions must be greater than zero.
    ZeroDimension,
}

impl fmt::Display for DitherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height, expected_len, actual_len } => {
                write!(f, "invalid image dimensions {width}x{height}: expected {expected_len} bytes, got {actual_len}")
            }
            Self::EmptyPalette => write!(f, "color palette cannot be empty"),
            Self::InvalidParameter(msg) => write!(f, "invalid parameter: {msg}"),
            Self::ZeroDimension => write!(f, "image width and height must be non-zero"),
        }
    }
}

impl std::error::Error for DitherError {}
