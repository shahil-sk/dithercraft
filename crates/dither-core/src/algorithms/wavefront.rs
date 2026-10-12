use super::error_diffusion::{FLOYD_STEINBERG_KERNEL, KernelEntry};
use super::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;
use std::sync::Mutex;

/// Tile-based parallel diagonal wavefront error diffusion.
///
/// Partitions an image into tiles of size `block_size x block_size`.
/// Tiles along the same diagonal wavefront band (where `tx + ty == wave`) have no
/// horizontal or vertical error dependencies on each other and execute concurrently
struct TileWaveResult {
    tile_coords: (usize, usize),
    tile_pixels: Vec<(usize, [u8; 3])>,
    bottom_out: Vec<[i32; 3]>,
    right_out: Vec<[i32; 3]>,
}

/// Execute error diffusion across diagonal wavefront bands ($x + y = k$) in parallel
/// on Rayon worker threads.
pub fn diffuse_wavefront_parallel(
    input: &ImageBuffer,
    palette: &Palette,
    kernel: &[KernelEntry],
    divisor: i32,
    block_size: usize,
) -> Result<ImageBuffer, DitherError> {
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }

    let width = input.width as usize;
    let height = input.height as usize;
    let mut out = ImageBuffer::new(input.width, input.height)?;

    let bs = block_size.clamp(8, 128);
    let num_tx = width.div_ceil(bs);
    let num_ty = height.div_ceil(bs);

    // If small image or single tile, run standard diffusion
    if num_tx <= 1 && num_ty <= 1 {
        return super::error_diffusion::diffuse(input, palette, kernel, divisor, false);
    }

    // Boundary error storage:
    // horizontal_errors[ty][tx]: error diffused downward from tile (tx, ty) to (tx, ty + 1)
    // vertical_errors[ty][tx]: error diffused rightward from tile (tx, ty) to (tx + 1, ty)
    let horizontal_errors: Vec<Vec<Mutex<Vec<[i32; 3]>>>> = (0..num_ty).map(|_| (0..num_tx).map(|_| Mutex::new(Vec::new())).collect()).collect();

    let vertical_errors: Vec<Vec<Mutex<Vec<[i32; 3]>>>> = (0..num_ty).map(|_| (0..num_tx).map(|_| Mutex::new(Vec::new())).collect()).collect();

    let total_waves = num_tx + num_ty - 1;

    // Mutex-protected output buffer slice writer
    let out_mutex = Mutex::new(&mut out.data);

    for wave in 0..total_waves {
        // Find all tiles in this wave: tx + ty == wave
        let tiles_in_wave: Vec<(usize, usize)> =
            (0..num_tx).filter_map(|tx| if wave >= tx && (wave - tx) < num_ty { Some((tx, wave - tx)) } else { None }).collect();

        // Process all independent tiles in the wavefront concurrently
        let wave_results: Vec<TileWaveResult> = tiles_in_wave
            .into_par_iter()
            .map(|(tx, ty)| {
                let x_start = tx * bs;
                let x_end = (x_start + bs).min(width);
                let y_start = ty * bs;
                let y_end = (y_start + bs).min(height);
                let tile_w = x_end - x_start;
                let tile_h = y_end - y_start;

                // Retrieve incoming top boundary error if ty > 0
                let top_in = if ty > 0 { horizontal_errors[ty - 1][tx].lock().ok().map(|g| g.clone()).unwrap_or_default() } else { Vec::new() };

                // Retrieve incoming left boundary error if tx > 0
                let left_in = if tx > 0 { vertical_errors[ty][tx - 1].lock().ok().map(|g| g.clone()).unwrap_or_default() } else { Vec::new() };

                // Local tile error buffers with padding
                let pad_w = tile_w + 8;
                let mut err_rows = vec![vec![[0i32; 3]; pad_w]; tile_h + 4];

                // Inject top incoming errors into row 0
                for (i, err) in top_in.iter().enumerate().take(tile_w) {
                    err_rows[0][i + 4] = *err;
                }

                // Inject left incoming errors into first column of each row
                for (j, err) in left_in.iter().enumerate().take(tile_h) {
                    err_rows[j][4] = [err_rows[j][4][0] + err[0], err_rows[j][4][1] + err[1], err_rows[j][4][2] + err[2]];
                }

                let mut tile_pixels = Vec::with_capacity(tile_w * tile_h);

                for local_y in 0..tile_h {
                    let global_y = y_start + local_y;
                    for local_x in 0..tile_w {
                        let global_x = x_start + local_x;
                        let in_idx = (global_y * width + global_x) * 3;

                        let err = err_rows[local_y][local_x + 4];
                        let r_in = (input.data[in_idx] as i32 + err[0]).clamp(0, 255) as u8;
                        let g_in = (input.data[in_idx + 1] as i32 + err[1]).clamp(0, 255) as u8;
                        let b_in = (input.data[in_idx + 2] as i32 + err[2]).clamp(0, 255) as u8;

                        let cur = RgbColor::new(r_in, g_in, b_in);
                        let quantized = palette.find_nearest(&cur);

                        let out_idx = (global_y * width + global_x) * 3;
                        tile_pixels.push((out_idx, [quantized.r, quantized.g, quantized.b]));

                        let err_r = (cur.r as i32) - (quantized.r as i32);
                        let err_g = (cur.g as i32) - (quantized.g as i32);
                        let err_b = (cur.b as i32) - (quantized.b as i32);

                        for entry in kernel {
                            let target_x = (local_x as i32 + 4 + entry.dx) as usize;
                            let target_y = (local_y as i32 + entry.dy) as usize;

                            let add_r = (err_r * entry.weight) / divisor;
                            let add_g = (err_g * entry.weight) / divisor;
                            let add_b = (err_b * entry.weight) / divisor;

                            if target_y < err_rows.len() && target_x < pad_w {
                                err_rows[target_y][target_x][0] += add_r;
                                err_rows[target_y][target_x][1] += add_g;
                                err_rows[target_y][target_x][2] += add_b;
                            }
                        }
                    }
                }

                // Collect bottom outgoing error for tile below (tx, ty + 1)
                let bottom_out: Vec<[i32; 3]> = (0..tile_w).map(|x| err_rows[tile_h][x + 4]).collect();

                // Collect right outgoing error for tile to the right (tx + 1, ty)
                let right_out: Vec<[i32; 3]> = (0..tile_h).map(|y| err_rows[y][tile_w + 4]).collect();

                TileWaveResult { tile_coords: (tx, ty), tile_pixels, bottom_out, right_out }
            })
            .collect();

        // Write tile results and store boundaries
        if let Ok(mut lock) = out_mutex.lock() {
            for res in wave_results {
                let (tx, ty) = res.tile_coords;
                for (idx, rgb) in res.tile_pixels {
                    lock[idx] = rgb[0];
                    lock[idx + 1] = rgb[1];
                    lock[idx + 2] = rgb[2];
                }

                if let Ok(mut g) = horizontal_errors[ty][tx].lock() {
                    *g = res.bottom_out;
                }
                if let Ok(mut g) = vertical_errors[ty][tx].lock() {
                    *g = res.right_out;
                }
            }
        }
    }

    Ok(out)
}

/// Floyd-Steinberg error diffusion executing along parallel diagonal wavefront bands.
#[derive(Debug, Clone)]
pub struct ParallelWavefrontFloydSteinberg {
    pub block_size: usize,
}

impl Default for ParallelWavefrontFloydSteinberg {
    fn default() -> Self {
        Self { block_size: 16 }
    }
}

impl DitherAlgorithm for ParallelWavefrontFloydSteinberg {
    fn name(&self) -> &'static str {
        "Wavefront Parallel Floyd-Steinberg"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        diffuse_wavefront_parallel(input, palette, &FLOYD_STEINBERG_KERNEL, 16, self.block_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wavefront_parallel_floyd_steinberg() {
        let mut img = ImageBuffer::new(64, 64).expect("buf");
        for y in 0..64 {
            for x in 0..64 {
                let v = ((x + y) * 2).min(255) as u8;
                img.set_pixel(x, y, RgbColor::new(v, v, v));
            }
        }

        let palette = Palette::game_boy();
        let algo = ParallelWavefrontFloydSteinberg { block_size: 16 };
        let out = algo.apply(&img, &palette).expect("wavefront out");

        assert_eq!(out.width, 64);
        assert_eq!(out.height, 64);
        assert!(!out.data.is_empty());
    }

    #[test]
    fn test_wavefront_non_multiple_dimensions() {
        let mut img = ImageBuffer::new(37, 23).expect("buf");
        for y in 0..23 {
            for x in 0..37 {
                img.set_pixel(x, y, RgbColor::new(100, 150, 200));
            }
        }

        let palette = Palette::game_boy();
        let algo = ParallelWavefrontFloydSteinberg { block_size: 16 };
        let out = algo.apply(&img, &palette).expect("wavefront out");

        assert_eq!(out.width, 37);
        assert_eq!(out.height, 23);
    }
}
