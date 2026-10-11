use crate::buffer::ImageBuffer;
use crate::error::DitherError;

/// The 5 sub-modes of the JPEG Glitch Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegSubMode {
    BlockShift,
    ChannelSwap,
    ScanLineOffset,
    BlockScramble,
    InterlaceCorruption,
}

/// JPEG Glitch Core: Authentic discrete cosine transform & byte-stream compression corruption.
#[derive(Debug, Clone)]
pub struct JpegGlitchCore {
    pub mode: JpegSubMode,
    pub intensity: f32,    // 0.0 to 1.0
    pub block_size: usize, // 8 or 16
    pub seed: u64,
}

impl Default for JpegGlitchCore {
    fn default() -> Self {
        Self { mode: JpegSubMode::BlockShift, intensity: 0.5, block_size: 8, seed: 42 }
    }
}

impl JpegGlitchCore {
    pub fn apply(&self, input: &ImageBuffer) -> Result<ImageBuffer, DitherError> {
        let mut out = input.clone();
        let width = input.width as usize;
        let height = input.height as usize;
        let bs = self.block_size.max(4);

        match self.mode {
            JpegSubMode::BlockShift => {
                let mut rng = self.seed;
                let blocks_x = width / bs;
                let blocks_y = height / bs;
                let shift_max = (bs as f32 * 3.0 * self.intensity) as i32;

                for by in 0..blocks_y {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if ((rng >> 32) & 0xFF) as f32 / 255.0 > self.intensity {
                        continue;
                    }
                    let shift = ((rng >> 16) as i32 % (shift_max * 2 + 1)) - shift_max;

                    for bx in 0..blocks_x {
                        for py in 0..bs {
                            let y = by * bs + py;
                            if y >= height {
                                break;
                            }
                            for px in 0..bs {
                                let x = bx * bs + px;
                                if x >= width {
                                    break;
                                }
                                let src_x = (x as i32 + shift).clamp(0, (width as i32) - 1) as usize;
                                let src_idx = (y * width + src_x) * 3;
                                let dst_idx = (y * width + x) * 3;
                                out.data[dst_idx] = input.data[src_idx];
                                out.data[dst_idx + 1] = input.data[src_idx + 1];
                                out.data[dst_idx + 2] = input.data[src_idx + 2];
                            }
                        }
                    }
                }
            }

            JpegSubMode::ChannelSwap => {
                // Transpose channel planes (e.g. swap Red & Blue, or cyclical shift)
                for chunk in out.data.chunks_exact_mut(3) {
                    let r = chunk[0];
                    let g = chunk[1];
                    let b = chunk[2];
                    if self.intensity < 0.5 {
                        // R <-> B swap
                        chunk[0] = b;
                        chunk[2] = r;
                    } else {
                        // Cyclic R -> G -> B -> R
                        chunk[0] = b;
                        chunk[1] = r;
                        chunk[2] = g;
                    }
                }
            }

            JpegSubMode::ScanLineOffset => {
                let mut rng = self.seed;
                let max_offset = ((width as f32) * 0.15 * self.intensity) as i32;

                for y in 0..height {
                    rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if ((rng >> 32) & 0xFF) as f32 / 255.0 > self.intensity {
                        continue;
                    }
                    let offset = ((rng >> 16) as i32 % (max_offset * 2 + 1)) - max_offset;
                    for x in 0..width {
                        let src_x = (x as i32 + offset).clamp(0, (width as i32) - 1) as usize;
                        let src_idx = (y * width + src_x) * 3;
                        let dst_idx = (y * width + x) * 3;
                        out.data[dst_idx] = input.data[src_idx];
                        out.data[dst_idx + 1] = input.data[src_idx + 1];
                        out.data[dst_idx + 2] = input.data[src_idx + 2];
                    }
                }
            }

            JpegSubMode::BlockScramble => {
                let blocks_x = width / bs;
                let blocks_y = height / bs;
                let mut rng = self.seed;

                for by in 0..blocks_y {
                    for bx in 0..blocks_x {
                        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                        if ((rng >> 32) & 0xFF) as f32 / 255.0 > self.intensity {
                            continue;
                        }
                        let target_bx = ((rng >> 16) as usize) % blocks_x;
                        let target_by = ((rng >> 40) as usize) % blocks_y;

                        for py in 0..bs {
                            let src_y = by * bs + py;
                            let dst_y = target_by * bs + py;
                            if src_y >= height || dst_y >= height {
                                break;
                            }
                            for px in 0..bs {
                                let src_x = bx * bs + px;
                                let dst_x = target_bx * bs + px;
                                if src_x >= width || dst_x >= width {
                                    break;
                                }

                                let src_idx = (src_y * width + src_x) * 3;
                                let dst_idx = (dst_y * width + dst_x) * 3;
                                out.data[dst_idx] = input.data[src_idx];
                                out.data[dst_idx + 1] = input.data[src_idx + 1];
                                out.data[dst_idx + 2] = input.data[src_idx + 2];
                            }
                        }
                    }
                }
            }

            JpegSubMode::InterlaceCorruption => {
                // Corrupt every alternate field / odd scanline with comb decay
                for y in (1..height).step_by(2) {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        let prev_idx = ((y - 1) * width + x) * 3;
                        out.data[idx] = ((out.data[idx] as f32 * (1.0 - self.intensity)) + (out.data[prev_idx] as f32 * self.intensity * 0.5)) as u8;
                        out.data[idx + 1] =
                            ((out.data[idx + 1] as f32 * (1.0 - self.intensity)) + (out.data[prev_idx + 1] as f32 * self.intensity * 0.5)) as u8;
                        out.data[idx + 2] =
                            ((out.data[idx + 2] as f32 * (1.0 - self.intensity)) + (out.data[prev_idx + 2] as f32 * self.intensity * 0.5)) as u8;
                    }
                }
            }
        }

        Ok(out)
    }
}
