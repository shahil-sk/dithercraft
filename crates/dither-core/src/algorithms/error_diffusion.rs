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

/// Generic error diffusion engine supporting up to 4 row lookahead with zero heap allocations in hot loop.
pub fn diffuse(input: &ImageBuffer, palette: &Palette, kernel: &[KernelEntry], divisor: i32, serpentine: bool) -> Result<ImageBuffer, DitherError> {
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }
    let width = input.width as usize;
    let height = input.height as usize;
    let mut out = ImageBuffer::new(input.width, input.height)?;

    // We keep error accumulators for current and next 3 rows with padding (+4 on left, +4 on right).
    let pad_w = width + 8;
    let mut err_row0 = vec![[0i32; 3]; pad_w];
    let mut err_row1 = vec![[0i32; 3]; pad_w];
    let mut err_row2 = vec![[0i32; 3]; pad_w];
    let mut err_row3 = vec![[0i32; 3]; pad_w];

    for y in 0..height {
        let is_odd = y % 2 == 1;
        let reverse = serpentine && is_odd;

        for step in 0..width {
            let x = if reverse { width - 1 - step } else { step };
            let in_idx = (y * width + x) * 3;
            let err = err_row0[x + 4];

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
                let target_x = (x as i32 + 4 + dx) as usize;

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
                        3 => {
                            err_row3[target_x][0] += add_r;
                            err_row3[target_x][1] += add_g;
                            err_row3[target_x][2] += add_b;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Shift rows down
        std::mem::swap(&mut err_row0, &mut err_row1);
        std::mem::swap(&mut err_row1, &mut err_row2);
        std::mem::swap(&mut err_row2, &mut err_row3);
        err_row3.fill([0i32; 3]);
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

// Stevenson-Arce (1985): divisor 200 (hexagonal sampling grid emulation)
pub static STEVENSON_ARCE_KERNEL: [KernelEntry; 12] = [
    KernelEntry { dx: 1, dy: 0, weight: 32 },
    KernelEntry { dx: -3, dy: 1, weight: 12 },
    KernelEntry { dx: -1, dy: 1, weight: 26 },
    KernelEntry { dx: 1, dy: 1, weight: 30 },
    KernelEntry { dx: 3, dy: 1, weight: 16 },
    KernelEntry { dx: -2, dy: 2, weight: 12 },
    KernelEntry { dx: 0, dy: 2, weight: 26 },
    KernelEntry { dx: 2, dy: 2, weight: 12 },
    KernelEntry { dx: -3, dy: 3, weight: 5 },
    KernelEntry { dx: -1, dy: 3, weight: 12 },
    KernelEntry { dx: 1, dy: 3, weight: 12 },
    KernelEntry { dx: 3, dy: 3, weight: 5 },
];

// Minimized Average Error (Kolpatzir & Bouman 1992): divisor 48
pub static MINIMIZED_AVERAGE_ERROR_KERNEL: [KernelEntry; 12] = [
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

// Fan (1993): divisor 16
pub static FAN_KERNEL: [KernelEntry; 4] = [
    KernelEntry { dx: 1, dy: 0, weight: 7 },
    KernelEntry { dx: -1, dy: 1, weight: 1 },
    KernelEntry { dx: 0, dy: 1, weight: 3 },
    KernelEntry { dx: 1, dy: 1, weight: 5 },
];

// Shiau-Fan 1 (1996): divisor 16
pub static SHIAU_FAN_1_KERNEL: [KernelEntry; 5] = [
    KernelEntry { dx: 1, dy: 0, weight: 8 },
    KernelEntry { dx: -2, dy: 1, weight: 1 },
    KernelEntry { dx: -1, dy: 1, weight: 1 },
    KernelEntry { dx: 0, dy: 1, weight: 2 },
    KernelEntry { dx: 1, dy: 1, weight: 4 },
];

// Shiau-Fan 2 (1996): divisor 16
pub static SHIAU_FAN_2_KERNEL: [KernelEntry; 5] = [
    KernelEntry { dx: 1, dy: 0, weight: 8 },
    KernelEntry { dx: -2, dy: 1, weight: 1 },
    KernelEntry { dx: -1, dy: 1, weight: 2 },
    KernelEntry { dx: 0, dy: 1, weight: 3 },
    KernelEntry { dx: 1, dy: 1, weight: 2 },
];

// Knuth's 8x8 Dot-Diffusion Order Matrix (Knuth 1987)
pub static KNUTH_DOT_MATRIX: [[u8; 8]; 8] = [
    [34, 48, 40, 32, 29, 15, 23, 31],
    [42, 58, 56, 50, 21, 5, 7, 13],
    [46, 62, 60, 52, 17, 1, 3, 11],
    [38, 54, 63, 44, 25, 9, 19, 27],
    [28, 14, 22, 30, 35, 49, 41, 33],
    [20, 4, 6, 12, 43, 59, 57, 51],
    [16, 0, 2, 10, 47, 61, 60, 53],
    [24, 8, 18, 26, 39, 55, 62, 45],
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

#[derive(Default)]
pub struct StevensonArce {
    pub serpentine: bool,
}
impl DitherAlgorithm for StevensonArce {
    fn name(&self) -> &'static str {
        "Stevenson-Arce"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &STEVENSON_ARCE_KERNEL, 200, self.serpentine)
    }
}

#[derive(Default)]
pub struct MinimizedAverageError {
    pub serpentine: bool,
}
impl DitherAlgorithm for MinimizedAverageError {
    fn name(&self) -> &'static str {
        "Minimized Average Error"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &MINIMIZED_AVERAGE_ERROR_KERNEL, 48, self.serpentine)
    }
}

#[derive(Default)]
pub struct Fan {
    pub serpentine: bool,
}
impl DitherAlgorithm for Fan {
    fn name(&self) -> &'static str {
        "Fan"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &FAN_KERNEL, 16, self.serpentine)
    }
}

#[derive(Default)]
pub struct ShiauFan1 {
    pub serpentine: bool,
}
impl DitherAlgorithm for ShiauFan1 {
    fn name(&self) -> &'static str {
        "Shiau-Fan 1"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &SHIAU_FAN_1_KERNEL, 16, self.serpentine)
    }
}

#[derive(Default)]
pub struct ShiauFan2 {
    pub serpentine: bool,
}
impl DitherAlgorithm for ShiauFan2 {
    fn name(&self) -> &'static str {
        "Shiau-Fan 2"
    }
    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse(input, palette, &SHIAU_FAN_2_KERNEL, 16, self.serpentine)
    }
}

/// Ostromoukhov Variable-Coefficient Error Diffusion (SIGGRAPH 2001)
#[derive(Default)]
pub struct Ostromoukhov {
    pub serpentine: bool,
}

impl Ostromoukhov {
    /// Discrete piecewise-linear coefficients from Ostromoukhov (2001)
    #[inline]
    fn coeffs(lum: u8) -> (i32, i32, i32, i32) {
        let v = lum as i32;
        // Breakpoints: 0 -> (13, 5, 8), 64 -> (18, 10, 14), 128 -> (21, 11, 16), 192 -> (18, 10, 14), 255 -> (13, 5, 8)
        let (d1, d2, d3) = if v < 64 {
            let t = v;
            (13 + (5 * t) / 64, 5 + (5 * t) / 64, 8 + (6 * t) / 64)
        } else if v < 128 {
            let t = v - 64;
            (18 + (3 * t) / 64, 10 + t / 64, 14 + (2 * t) / 64)
        } else if v < 192 {
            let t = v - 128;
            (21 - (3 * t) / 64, 11 - t / 64, 16 - (2 * t) / 64)
        } else {
            let t = v - 192;
            (18 - (5 * t) / 63, 10 - (5 * t) / 63, 14 - (6 * t) / 63)
        };
        let sum = d1 + d2 + d3;
        (d1, d2, d3, sum.max(1))
    }
}

impl DitherAlgorithm for Ostromoukhov {
    fn name(&self) -> &'static str {
        "Ostromoukhov"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        let pad_w = width + 4;
        let mut err_row0 = vec![[0i32; 3]; pad_w];
        let mut err_row1 = vec![[0i32; 3]; pad_w];

        for y in 0..height {
            let is_odd = y % 2 == 1;
            let reverse = self.serpentine && is_odd;

            for step in 0..width {
                let x = if reverse { width - 1 - step } else { step };
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

                let lum = current_color.luminance();
                let (d1, d2, d3, divisor) = Self::coeffs(lum);

                let err_r = (current_color.r as i32) - (quantized.r as i32);
                let err_g = (current_color.g as i32) - (quantized.g as i32);
                let err_b = (current_color.b as i32) - (quantized.b as i32);

                let (dx_right, dx_down_left, dx_down) = if reverse { (-1, 1, 0) } else { (1, -1, 0) };

                // (1, 0) right
                let t1 = (x as i32 + 2 + dx_right) as usize;
                if t1 < pad_w {
                    err_row0[t1][0] += (err_r * d1) / divisor;
                    err_row0[t1][1] += (err_g * d1) / divisor;
                    err_row0[t1][2] += (err_b * d1) / divisor;
                }
                // (-1, 1) down-left
                let t2 = (x as i32 + 2 + dx_down_left) as usize;
                if t2 < pad_w {
                    err_row1[t2][0] += (err_r * d2) / divisor;
                    err_row1[t2][1] += (err_g * d2) / divisor;
                    err_row1[t2][2] += (err_b * d2) / divisor;
                }
                // (0, 1) down
                let t3 = (x as i32 + 2 + dx_down) as usize;
                if t3 < pad_w {
                    err_row1[t3][0] += (err_r * d3) / divisor;
                    err_row1[t3][1] += (err_g * d3) / divisor;
                    err_row1[t3][2] += (err_b * d3) / divisor;
                }
            }

            std::mem::swap(&mut err_row0, &mut err_row1);
            err_row1.fill([0i32; 3]);
        }

        Ok(out)
    }
}

/// Variable-Error Diffusion (Eschbach & Knox 1991 threshold modulation)
#[derive(Default)]
pub struct VariableError {
    pub serpentine: bool,
}

impl DitherAlgorithm for VariableError {
    fn name(&self) -> &'static str {
        "Variable-Error"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        let pad_w = width + 4;
        let mut err_row0 = vec![[0i32; 3]; pad_w];
        let mut err_row1 = vec![[0i32; 3]; pad_w];

        for y in 0..height {
            let is_odd = y % 2 == 1;
            let reverse = self.serpentine && is_odd;

            for step in 0..width {
                let x = if reverse { width - 1 - step } else { step };
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

                // Tonal dampening factor: reduces worms in extreme shadows and highlights
                let lum = current_color.luminance() as i32;
                let dev = (lum - 128).abs();
                let damp_num = 256 - (dev * dev) / 128; // in [128, 256]

                let err_r = (((current_color.r as i32 - quantized.r as i32) * damp_num) >> 8).clamp(-128, 128);
                let err_g = (((current_color.g as i32 - quantized.g as i32) * damp_num) >> 8).clamp(-128, 128);
                let err_b = (((current_color.b as i32 - quantized.b as i32) * damp_num) >> 8).clamp(-128, 128);

                // Floyd-Steinberg kernel distribution
                let dx_r = if reverse { -1 } else { 1 };
                let dx_dl = if reverse { 1 } else { -1 };
                let dx_dr = if reverse { -1 } else { 1 };

                let t1 = (x as i32 + 2 + dx_r) as usize;
                if t1 < pad_w {
                    err_row0[t1][0] += (err_r * 7) / 16;
                    err_row0[t1][1] += (err_g * 7) / 16;
                    err_row0[t1][2] += (err_b * 7) / 16;
                }
                let t2 = (x as i32 + 2 + dx_dl) as usize;
                if t2 < pad_w {
                    err_row1[t2][0] += (err_r * 3) / 16;
                    err_row1[t2][1] += (err_g * 3) / 16;
                    err_row1[t2][2] += (err_b * 3) / 16;
                }
                let t3 = (x as i32 + 2) as usize;
                if t3 < pad_w {
                    err_row1[t3][0] += (err_r * 5) / 16;
                    err_row1[t3][1] += (err_g * 5) / 16;
                    err_row1[t3][2] += (err_b * 5) / 16;
                }
                let t4 = (x as i32 + 2 + dx_dr) as usize;
                if t4 < pad_w {
                    err_row1[t4][0] += err_r / 16;
                    err_row1[t4][1] += err_g / 16;
                    err_row1[t4][2] += err_b / 16;
                }
            }

            std::mem::swap(&mut err_row0, &mut err_row1);
            err_row1.fill([0i32; 3]);
        }

        Ok(out)
    }
}

/// Dot-Diffusion Halftoning (Donald E. Knuth, ACM TOG 1987)
#[derive(Default)]
pub struct DotDiffusion;

impl DitherAlgorithm for DotDiffusion {
    fn name(&self) -> &'static str {
        "Dot-Diffusion"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        // Full image error buffer
        let mut err_buf = vec![[0i32; 3]; width * height];

        // Process pixels in increasing dot-matrix rank k in 0..64
        for k in 0..64u8 {
            for y in 0..height {
                let my = y % 8;
                for x in 0..width {
                    let mx = x % 8;
                    if KNUTH_DOT_MATRIX[my][mx] == k {
                        let idx = y * width + x;
                        let in_idx = idx * 3;
                        let err = err_buf[idx];

                        let r_in = (input.data[in_idx] as i32 + err[0]).clamp(0, 255) as u8;
                        let g_in = (input.data[in_idx + 1] as i32 + err[1]).clamp(0, 255) as u8;
                        let b_in = (input.data[in_idx + 2] as i32 + err[2]).clamp(0, 255) as u8;

                        let current_color = RgbColor::new(r_in, g_in, b_in);
                        let quantized = palette.find_nearest(&current_color);

                        out.data[in_idx] = quantized.r;
                        out.data[in_idx + 1] = quantized.g;
                        out.data[in_idx + 2] = quantized.b;

                        let err_r = current_color.r as i32 - quantized.r as i32;
                        let err_g = current_color.g as i32 - quantized.g as i32;
                        let err_b = current_color.b as i32 - quantized.b as i32;

                        // Distribute to 8 neighbors with rank > k
                        // Orthogonal weight = 2, diagonal weight = 1
                        let mut total_w = 0i32;
                        for dy in -1i32..=1 {
                            for dx in -1i32..=1 {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                let nx = x as i32 + dx;
                                let ny = y as i32 + dy;
                                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                                    let neighbor_rank = KNUTH_DOT_MATRIX[(ny as usize) % 8][(nx as usize) % 8];
                                    if neighbor_rank > k {
                                        total_w += if dx.abs() + dy.abs() == 1 { 2 } else { 1 };
                                    }
                                }
                            }
                        }

                        if total_w > 0 {
                            for dy in -1i32..=1 {
                                for dx in -1i32..=1 {
                                    if dx == 0 && dy == 0 {
                                        continue;
                                    }
                                    let nx = x as i32 + dx;
                                    let ny = y as i32 + dy;
                                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                                        let neighbor_rank = KNUTH_DOT_MATRIX[(ny as usize) % 8][(nx as usize) % 8];
                                        if neighbor_rank > k {
                                            let w = if dx.abs() + dy.abs() == 1 { 2 } else { 1 };
                                            let n_idx = (ny as usize) * width + (nx as usize);
                                            err_buf[n_idx][0] += (err_r * w) / total_w;
                                            err_buf[n_idx][1] += (err_g * w) / total_w;
                                            err_buf[n_idx][2] += (err_b * w) / total_w;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(out)
    }
}

/// Riemersma Space-Filling Hilbert Curve Dithering (Thiadmer Riemersma, Dr. Dobb's Journal 1998)
#[derive(Default)]
pub struct Riemersma;

impl Riemersma {
    /// Convert 1D Hilbert step d to 2D coordinate (x, y) on an n x n grid
    fn d2xy(n: usize, mut d: usize) -> (usize, usize) {
        let mut x = 0;
        let mut y = 0;
        let mut s = 1;
        while s < n {
            let rx = 1 & (d / 2);
            let ry = 1 & (d ^ rx);
            if ry == 0 {
                if rx == 1 {
                    x = s - 1 - x;
                    y = s - 1 - y;
                }
                std::mem::swap(&mut x, &mut y);
            }
            x += s * rx;
            y += s * ry;
            d /= 4;
            s *= 2;
        }
        (x, y)
    }
}

// 16-element exponential decay weights: r = exp(-ln(16)/15) ≈ 0.8304
const RIEMERSMA_WEIGHTS: [i32; 16] = [256, 213, 177, 147, 122, 101, 84, 70, 58, 48, 40, 33, 27, 23, 19, 16];
const RIEMERSMA_WEIGHT_SUM: i32 = 1434;

impl DitherAlgorithm for Riemersma {
    fn name(&self) -> &'static str {
        "Riemersma"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }
        let width = input.width as usize;
        let height = input.height as usize;
        let mut out = ImageBuffer::new(input.width, input.height)?;

        // Find bounding power-of-two size for Hilbert curve
        let max_dim = width.max(height);
        let mut n = 1;
        while n < max_dim {
            n *= 2;
        }

        let mut history = [[0i32; 3]; 16];
        let mut hist_head = 0usize;

        let total_steps = n * n;
        for d in 0..total_steps {
            let (x, y) = Self::d2xy(n, d);
            if x < width && y < height {
                // Compute decayed accumulated error from history queue
                let mut err_r = 0i32;
                let mut err_g = 0i32;
                let mut err_b = 0i32;
                for i in 0..16 {
                    let w = RIEMERSMA_WEIGHTS[i];
                    let h = history[(hist_head + 16 - i) % 16];
                    err_r += h[0] * w;
                    err_g += h[1] * w;
                    err_b += h[2] * w;
                }
                err_r /= RIEMERSMA_WEIGHT_SUM;
                err_g /= RIEMERSMA_WEIGHT_SUM;
                err_b /= RIEMERSMA_WEIGHT_SUM;

                let in_idx = (y * width + x) * 3;
                let r_in = (input.data[in_idx] as i32 + err_r).clamp(0, 255) as u8;
                let g_in = (input.data[in_idx + 1] as i32 + err_g).clamp(0, 255) as u8;
                let b_in = (input.data[in_idx + 2] as i32 + err_b).clamp(0, 255) as u8;

                let current_color = RgbColor::new(r_in, g_in, b_in);
                let quantized = palette.find_nearest(&current_color);

                let out_idx = (y * width + x) * 3;
                out.data[out_idx] = quantized.r;
                out.data[out_idx + 1] = quantized.g;
                out.data[out_idx + 2] = quantized.b;

                // Push residual error into circular buffer
                hist_head = (hist_head + 1) % 16;
                history[hist_head] = [
                    (current_color.r as i32) - (quantized.r as i32),
                    (current_color.g as i32) - (quantized.g as i32),
                    (current_color.b as i32) - (quantized.b as i32),
                ];
            }
        }

        Ok(out)
    }
}
