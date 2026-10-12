use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::{ColorMetric, Palette};
use rayon::prelude::*;

/// SIMD-friendly 8-wide batch quantization kernel.
///
/// Evaluates 8 pixels simultaneously against palette color swatches using packed
/// vector arithmetic without inner loop branches, enabling LLVM auto-vectorization
/// into AVX2 / NEON SIMD vector lanes.
pub fn quantize_batch_8(pixels: &[RgbColor; 8], swatches: &[RgbColor], metric: ColorMetric) -> [RgbColor; 8] {
    if swatches.is_empty() {
        return *pixels;
    }

    let mut best_colors = [swatches[0]; 8];

    match metric {
        ColorMetric::EuclideanRgb => {
            let r = [
                pixels[0].r as i32,
                pixels[1].r as i32,
                pixels[2].r as i32,
                pixels[3].r as i32,
                pixels[4].r as i32,
                pixels[5].r as i32,
                pixels[6].r as i32,
                pixels[7].r as i32,
            ];
            let g = [
                pixels[0].g as i32,
                pixels[1].g as i32,
                pixels[2].g as i32,
                pixels[3].g as i32,
                pixels[4].g as i32,
                pixels[5].g as i32,
                pixels[6].g as i32,
                pixels[7].g as i32,
            ];
            let b = [
                pixels[0].b as i32,
                pixels[1].b as i32,
                pixels[2].b as i32,
                pixels[3].b as i32,
                pixels[4].b as i32,
                pixels[5].b as i32,
                pixels[6].b as i32,
                pixels[7].b as i32,
            ];
            let mut min_dists = [u32::MAX; 8];

            for swatch in swatches {
                let sr = swatch.r as i32;
                let sg = swatch.g as i32;
                let sb = swatch.b as i32;

                for i in 0..8 {
                    let dr = r[i] - sr;
                    let dg = g[i] - sg;
                    let db = b[i] - sb;
                    let dist = (dr * dr + dg * dg + db * db) as u32;
                    if dist < min_dists[i] {
                        min_dists[i] = dist;
                        best_colors[i] = *swatch;
                    }
                }
            }
        }

        ColorMetric::Oklab => {
            let lab: [crate::color::OklabColor; 8] = [
                pixels[0].to_oklab(),
                pixels[1].to_oklab(),
                pixels[2].to_oklab(),
                pixels[3].to_oklab(),
                pixels[4].to_oklab(),
                pixels[5].to_oklab(),
                pixels[6].to_oklab(),
                pixels[7].to_oklab(),
            ];
            let mut min_dists = [f32::MAX; 8];

            for swatch in swatches {
                let s_lab = swatch.to_oklab();
                for i in 0..8 {
                    let dl = lab[i].l - s_lab.l;
                    let da = lab[i].a - s_lab.a;
                    let db = lab[i].b - s_lab.b;
                    let dist = dl * dl + da * da + db * db;
                    if dist < min_dists[i] {
                        min_dists[i] = dist;
                        best_colors[i] = *swatch;
                    }
                }
            }
        }

        _ => {
            // Fallback for Redmean, CIELAB, CIEDE2000
            for i in 0..8 {
                let mut min_dist = f32::MAX;
                for swatch in swatches {
                    let dist = match metric {
                        ColorMetric::Redmean => pixels[i].distance_redmean(swatch) as f32,
                        ColorMetric::CieLab => pixels[i].distance_cielab_de(swatch),
                        ColorMetric::Ciede2000 => pixels[i].distance_ciede2000(swatch),
                        _ => pixels[i].distance_sq(swatch) as f32,
                    };
                    if dist < min_dist {
                        min_dist = dist;
                        best_colors[i] = *swatch;
                    }
                }
            }
        }
    }

    best_colors
}

/// Hardware-accelerated batch quantization mapping an image buffer to palette colors.
///
/// Dispatches 8-wide SIMD chunks in parallel across Rayon worker threads.
pub fn quantize_buffer_simd(input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
    if palette.is_empty() {
        return Err(DitherError::EmptyPalette);
    }

    let mut out = ImageBuffer::new(input.width, input.height)?;
    let swatches = palette.colors();
    let metric = palette.metric;
    let total_pixels = (input.width as usize) * (input.height as usize);

    // Process in batches of 64 pixels (8 lanes of 8 pixels)
    const CHUNK_PIXELS: usize = 64;
    let chunk_bytes = CHUNK_PIXELS * 3;

    let in_chunks = input.data.chunks_exact(chunk_bytes);
    let in_remainder = in_chunks.remainder();

    out.data.par_chunks_exact_mut(chunk_bytes).enumerate().for_each(|(chunk_idx, out_chunk)| {
        let in_offset = chunk_idx * chunk_bytes;
        let in_slice = &input.data[in_offset..in_offset + chunk_bytes];

        for sub in 0..8 {
            let p_start = sub * 8 * 3;
            let mut batch = [RgbColor::new(0, 0, 0); 8];
            for (lane, item) in batch.iter_mut().enumerate() {
                let off = p_start + lane * 3;
                *item = RgbColor::new(in_slice[off], in_slice[off + 1], in_slice[off + 2]);
            }

            let quantized = quantize_batch_8(&batch, &swatches, metric);
            for (lane, q) in quantized.iter().enumerate() {
                let off = p_start + lane * 3;
                out_chunk[off] = q.r;
                out_chunk[off + 1] = q.g;
                out_chunk[off + 2] = q.b;
            }
        }
    });

    // Handle tail pixels
    let processed_pixels = (total_pixels / CHUNK_PIXELS) * CHUNK_PIXELS;
    let tail_offset = processed_pixels * 3;
    let out_tail = &mut out.data[tail_offset..];

    for (i, px_chunk) in in_remainder.chunks_exact(3).enumerate() {
        let px = RgbColor::new(px_chunk[0], px_chunk[1], px_chunk[2]);
        let q = palette.find_nearest(&px);
        out_tail[i * 3] = q.r;
        out_tail[i * 3 + 1] = q.g;
        out_tail[i * 3 + 2] = q.b;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_quantization_equivalence() {
        let palette = Palette::game_boy();
        let mut buf = ImageBuffer::new(16, 16).expect("buf");
        for y in 0..16 {
            for x in 0..16 {
                buf.set_pixel(x, y, RgbColor::new((x * 16) as u8, (y * 16) as u8, 128));
            }
        }

        let mut pal_rgb = palette.clone();
        pal_rgb.metric = ColorMetric::EuclideanRgb;
        let simd_out = quantize_buffer_simd(&buf, &pal_rgb).expect("simd");

        // Verify every pixel matches palette.find_nearest
        for y in 0..16 {
            for x in 0..16 {
                let src_px = buf.get_pixel(x, y).expect("src");
                let expected = pal_rgb.find_nearest(&src_px);
                let actual = simd_out.get_pixel(x, y).expect("simd");
                assert_eq!(actual, expected, "Mismatch at ({x}, {y})");
            }
        }
    }
}
