use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;

/// Generic diffusion kernel coefficient: (dx, dy, numerator)
#[derive(Debug, Clone, Copy)]
pub struct KernelEntry {
    pub dx: i32,
    pub dy: i32,
    pub weight: i32,
}

/// Generic error diffusion engine supporting up to 3 row lookahead with zero heap allocations in hot loop.
pub fn diffuse(input: &ImageBuffer, palette: &Palette, kernel: &[KernelEntry], divisor: i32, serpentine: bool) -> Result<ImageBuffer, DitherError> {
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }
    let width = input.width as usize;
    let height = input.height as usize;
    let mut out = ImageBuffer::new(input.width, input.height)?;

    // We keep error accumulators for current and next 2 rows with padding on both ends.
    let pad_w = width + 4;
    let mut err_row0 = vec![[0i32; 3]; pad_w];
    let mut err_row1 = vec![[0i32; 3]; pad_w];
    let mut err_row2 = vec![[0i32; 3]; pad_w];

    for y in 0..height {
        let is_odd = y % 2 == 1;
        let reverse = serpentine && is_odd;

        let x_indices: Box<dyn Iterator<Item = usize>> = if reverse { Box::new((0..width).rev()) } else { Box::new(0..width) };

        for x in x_indices {
            let in_idx = (y * width + x) * 3;
            let err = err_row0[x + 2];

            let r_in = (input.data[in_idx] as i32 + err[0]).clamp(0, 255) as u8;
            let g_in = (input.data[in_idx + 1] as i32 + err[1]).clamp(0, 255) as u8;
            let b_in = (input.data[in_idx + 2] as i32 + err[2]).clamp(0, 255) as u8;

            let current_color = RgbColor::new(r_in, g_in, b_in);
            let quantized = palette.find_nearest(&current_color);

            let out_idx = (y * width + x) * 3;
            out.data[out_idx] = quantized.r;
            out.data[out_idx + 1] = quantized.g;
            out.data[out_idx + 2] = quantized.b;

            // Residual error
            let err_r = (current_color.r as i32) - (quantized.r as i32);
            let err_g = (current_color.g as i32) - (quantized.g as i32);
            let err_b = (current_color.b as i32) - (quantized.b as i32);

            // Diffuse according to kernel
            for entry in kernel {
                let dx = if reverse { -entry.dx } else { entry.dx };
                let target_x = (x as i32 + 2 + dx) as usize;

                let add_r = (err_r * entry.weight) / divisor;
                let add_g = (err_g * entry.weight) / divisor;
                let add_b = (err_b * entry.weight) / divisor;

                if target_x < pad_w {
                    match entry.dy {
                        0 => {
                            err_row0[target_x][0] += add_r;
                            err_row0[target_x][1] += add_g;
                            err_row0[target_x][2] += add_b;
                        }
                        1 => {
                            err_row1[target_x][0] += add_r;
                            err_row1[target_x][1] += add_g;
                            err_row1[target_x][2] += add_b;
                        }
                        2 => {
                            err_row2[target_x][0] += add_r;
                            err_row2[target_x][1] += add_g;
                            err_row2[target_x][2] += add_b;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Shift rows down
        std::mem::swap(&mut err_row0, &mut err_row1);
        std::mem::swap(&mut err_row1, &mut err_row2);
        err_row2.fill([0i32; 3]);
    }

    Ok(out)
}

// --- Kernels ---

// Floyd-Steinberg (1976): divisor 16
pub static FLOYD_STEINBERG_KERNEL: [KernelEntry; 4] = [
    KernelEntry { dx: 1, dy: 0, weight: 7 },
    KernelEntry { dx: -1, dy: 1, weight: 3 },
    KernelEntry { dx: 0, dy: 1, weight: 5 },
    KernelEntry { dx: 1, dy: 1, weight: 1 },
];

// Atkinson (1984): divisor 8 (diffuses 6/8, discards 2/8)
pub static ATKINSON_KERNEL: [KernelEntry; 6] = [
    KernelEntry { dx: 1, dy: 0, weight: 1 },
    KernelEntry { dx: 2, dy: 0, weight: 1 },
    KernelEntry { dx: -1, dy: 1, weight: 1 },
    KernelEntry { dx: 0, dy: 1, weight: 1 },
    KernelEntry { dx: 1, dy: 1, weight: 1 },
    KernelEntry { dx: 0, dy: 2, weight: 1 },
];

// Burkes (1988): divisor 32
pub static BURKES_KERNEL: [KernelEntry; 7] = [
    KernelEntry { dx: 1, dy: 0, weight: 8 },
    KernelEntry { dx: 2, dy: 0, weight: 4 },
    KernelEntry { dx: -2, dy: 1, weight: 2 },
    KernelEntry { dx: -1, dy: 1, weight: 4 },
    KernelEntry { dx: 0, dy: 1, weight: 8 },
    KernelEntry { dx: 1, dy: 1, weight: 4 },
    KernelEntry { dx: 2, dy: 1, weight: 2 },
];

// Sierra-3 (1989): divisor 32
pub static SIERRA3_KERNEL: [KernelEntry; 10] = [
    KernelEntry { dx: 1, dy: 0, weight: 5 },
    KernelEntry { dx: 2, dy: 0, weight: 3 },
    KernelEntry { dx: -2, dy: 1, weight: 2 },
    KernelEntry { dx: -1, dy: 1, weight: 4 },
    KernelEntry { dx: 0, dy: 1, weight: 5 },
    KernelEntry { dx: 1, dy: 1, weight: 4 },
    KernelEntry { dx: 2, dy: 1, weight: 2 },
    KernelEntry { dx: -1, dy: 2, weight: 2 },
    KernelEntry { dx: 0, dy: 2, weight: 3 },
    KernelEntry { dx: 1, dy: 2, weight: 2 },
];

// Sierra-2: divisor 16
pub static SIERRA2_KERNEL: [KernelEntry; 7] = [
    KernelEntry { dx: 1, dy: 0, weight: 4 },
    KernelEntry { dx: 2, dy: 0, weight: 3 },
    KernelEntry { dx: -2, dy: 1, weight: 1 },
    KernelEntry { dx: -1, dy: 1, weight: 2 },
    KernelEntry { dx: 0, dy: 1, weight: 3 },
    KernelEntry { dx: 1, dy: 1, weight: 2 },
    KernelEntry { dx: 2, dy: 1, weight: 1 },
];

// Sierra-Lite: divisor 4
pub static SIERRA_LITE_KERNEL: [KernelEntry; 3] =
    [KernelEntry { dx: 1, dy: 0, weight: 2 }, KernelEntry { dx: -1, dy: 1, weight: 1 }, KernelEntry { dx: 0, dy: 1, weight: 1 }];

// Stucki (1981): divisor 42
pub static STUCKI_KERNEL: [KernelEntry; 12] = [
    KernelEntry { dx: 1, dy: 0, weight: 8 },
    KernelEntry { dx: 2, dy: 0, weight: 4 },
    KernelEntry { dx: -2, dy: 1, weight: 2 },
    KernelEntry { dx: -1, dy: 1, weight: 4 },
    KernelEntry { dx: 0, dy: 1, weight: 8 },
    KernelEntry { dx: 1, dy: 1, weight: 4 },
    KernelEntry { dx: 2, dy: 1, weight: 2 },
    KernelEntry { dx: -2, dy: 2, weight: 1 },
    KernelEntry { dx: -1, dy: 2, weight: 2 },
    KernelEntry { dx: 0, dy: 2, weight: 4 },
    KernelEntry { dx: 1, dy: 2, weight: 2 },
    KernelEntry { dx: 2, dy: 2, weight: 1 },
];

// Jarvis-Judice-Ninke (1976): divisor 48
pub static JJN_KERNEL: [KernelEntry; 12] = [
    KernelEntry { dx: 1, dy: 0, weight: 7 },
    KernelEntry { dx: 2, dy: 0, weight: 5 },
    KernelEntry { dx: -2, dy: 1, weight: 3 },
    KernelEntry { dx: -1, dy: 1, weight: 5 },
    KernelEntry { dx: 0, dy: 1, weight: 7 },
    KernelEntry { dx: 1, dy: 1, weight: 5 },
    KernelEntry { dx: 2, dy: 1, weight: 3 },
    KernelEntry { dx: -2, dy: 2, weight: 1 },
    KernelEntry { dx: -1, dy: 2, weight: 3 },
    KernelEntry { dx: 0, dy: 2, weight: 5 },
    KernelEntry { dx: 1, dy: 2, weight: 3 },
    KernelEntry { dx: 2, dy: 2, weight: 1 },
];

// --- Struct implementations ---

#[derive(Default)]
pub struct FloydSteinberg {
    pub serpentine: bool,
}
impl DitherAlgorithm for FloydSteinberg {
    fn name(&self) -> &'static str {
        "Floyd-Steinberg"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &FLOYD_STEINBERG_KERNEL, 16, self.serpentine)
    }
}

#[derive(Default)]
pub struct Atkinson {
    pub serpentine: bool,
}
impl DitherAlgorithm for Atkinson {
    fn name(&self) -> &'static str {
        "Atkinson"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &ATKINSON_KERNEL, 8, self.serpentine)
    }
}

#[derive(Default)]
pub struct Burkes {
    pub serpentine: bool,
}
impl DitherAlgorithm for Burkes {
    fn name(&self) -> &'static str {
        "Burkes"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &BURKES_KERNEL, 32, self.serpentine)
    }
}

#[derive(Default)]
pub struct Sierra3 {
    pub serpentine: bool,
}
impl DitherAlgorithm for Sierra3 {
    fn name(&self) -> &'static str {
        "Sierra-3"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &SIERRA3_KERNEL, 32, self.serpentine)
    }
}

#[derive(Default)]
pub struct Sierra2 {
    pub serpentine: bool,
}
impl DitherAlgorithm for Sierra2 {
    fn name(&self) -> &'static str {
        "Sierra-2"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &SIERRA2_KERNEL, 16, self.serpentine)
    }
}

#[derive(Default)]
pub struct SierraLite {
    pub serpentine: bool,
}
impl DitherAlgorithm for SierraLite {
    fn name(&self) -> &'static str {
        "Sierra-Lite"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &SIERRA_LITE_KERNEL, 4, self.serpentine)
    }
}

#[derive(Default)]
pub struct Stucki {
    pub serpentine: bool,
}
impl DitherAlgorithm for Stucki {
    fn name(&self) -> &'static str {
        "Stucki"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &STUCKI_KERNEL, 42, self.serpentine)
    }
}

#[derive(Default)]
pub struct JarvisJudiceNinke {
    pub serpentine: bool,
}
impl DitherAlgorithm for JarvisJudiceNinke {
    fn name(&self) -> &'static str {
        "Jarvis-Judice-Ninke"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &JJN_KERNEL, 48, self.serpentine)
    }
}
