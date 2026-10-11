#![allow(clippy::chunks_exact_to_as_chunks)]

mod mcp;

use clap::{Parser, Subcommand};
use dither_core::{
    ImageBuffer, algorithms::*, color::RgbColor, palette::Palette, renderers::*, temporal::TemporalNoisePattern, vector::BlackVectorIsolation,
};
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Frame, ImageReader, RgbaImage};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "dithercraft-cli")]
#[command(about = "Studio-grade DSP dithering, CRT filters, and vector isolation CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a single image file
    Single {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value = "floyd-steinberg")]
        algo: String,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
    },
    /// Process an animated GIF file preserving timing and multi-frame animation
    Gif {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value = "floyd-steinberg")]
        algo: String,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
        #[arg(long)]
        noise: Option<String>,
        #[arg(long, default_value_t = 0.5)]
        noise_intensity: f32,
    },
    /// Export darkest pixels as an isolated vector path SVG
    Svg {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 128)]
        threshold: u8,
    },
    /// Process all images in a directory concurrently
    Batch {
        #[arg(short, long)]
        input_dir: PathBuf,
        #[arg(short, long)]
        output_dir: PathBuf,
        #[arg(short, long, default_value = "floyd-steinberg")]
        algo: String,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
        #[arg(long, default_value = "{name}_{algo}_{palette}")]
        pattern: String,
    },
    /// Run Model Context Protocol (MCP) JSON-RPC server over stdio
    Mcp,
    /// Run internal latency and throughput micro-benchmarks
    Bench,
}

fn load_image(path: &Path) -> Result<ImageBuffer, Box<dyn std::error::Error>> {
    let img = ImageReader::open(path)?.decode()?.to_rgb8();
    let (width, height) = img.dimensions();
    let data = img.into_raw();
    Ok(ImageBuffer::from_raw(width, height, data)?)
}

fn save_image(path: &Path, buffer: &ImageBuffer) -> Result<(), Box<dyn std::error::Error>> {
    image::save_buffer(path, &buffer.data, buffer.width, buffer.height, image::ExtendedColorType::Rgb8)?;
    Ok(())
}

fn select_palette(name: &str) -> Palette {
    let p = Path::new(name);
    if p.exists()
        && let Ok(pal) = Palette::import_from_file(p)
    {
        return pal;
    }

    match name.to_lowercase().as_str() {
        "mono" | "monochrome" | "1bit" => Palette::monochrome(),
        "mac" | "mac1bit" => Palette::mac_1bit(),
        "cga" | "cga1" => Palette::cga_mode_1(),
        "cyberpunk" => Palette::cyberpunk(),
        "apple" | "apple2" | "appleii" => Palette::apple_ii(),
        "c64" | "commodore" | "commodore64" => Palette::commodore_64(),
        "nes" => Palette::nes(),
        "pico8" | "pico-8" | "pico" => Palette::pico_8(),
        _ => Palette::game_boy(),
    }
}

fn select_algorithm(name: &str) -> Box<dyn DitherAlgorithm> {
    create_algorithm(name).unwrap_or_else(|| Box::new(FloydSteinberg::default()))
}

fn select_temporal_pattern(name: &str) -> Option<TemporalNoisePattern> {
    match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
        "crtscanbeamjitter" | "crt" | "beam" => Some(TemporalNoisePattern::CrtScanBeamJitter),
        "phosphorgrainhum" | "phosphor" | "hum" => Some(TemporalNoisePattern::PhosphorGrainHum),
        "interlacedfieldshimmer" | "interlace" | "shimmer" => Some(TemporalNoisePattern::InterlacedFieldShimmer),
        "lcdsubpixelgriddrift" | "lcd" | "drift" => Some(TemporalNoisePattern::LcdSubpixelGridDrift),
        "vhstapephasenoise" | "vhs" | "tape" => Some(TemporalNoisePattern::VhsTapePhaseNoise),
        "thermalanalogdrift" | "thermal" | "analog" => Some(TemporalNoisePattern::ThermalAnalogDrift),
        "statichashwhitenoise" | "static" | "hash" | "white" => Some(TemporalNoisePattern::StaticHashWhiteNoise),
        "bayerphasestepping" | "bayerstep" => Some(TemporalNoisePattern::BayerPhaseStepping),
        "bluenoisetemporaljitter" | "bluenoise" | "jitter" => Some(TemporalNoisePattern::BlueNoiseTemporalJitter),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Single { input, output, algo, palette } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let pal = select_palette(&palette);
            let algorithm = select_algorithm(&algo);

            let dithered = algorithm.apply(&img, &pal)?;
            save_image(&output, &dithered)?;
            let elapsed = start.elapsed();
            println!(
                "Successfully processed {} ({}x{}) using {} [{}] in {:.2}ms -> {}",
                input.display(),
                img.width,
                img.height,
                algorithm.name(),
                pal.name,
                elapsed.as_secs_f64() * 1000.0,
                output.display()
            );
        }

        Commands::Gif { input, output, algo, palette, noise, noise_intensity } => {
            let start = Instant::now();
            let pal = select_palette(&palette);
            let algo_name = algo.clone();
            let temporal_pattern = noise.as_deref().and_then(select_temporal_pattern);

            let file = std::fs::File::open(&input)?;
            let reader = std::io::BufReader::new(file);
            let decoder = GifDecoder::new(reader)?;
            let raw_frames = decoder.into_frames().collect_frames()?;
            let total_frames = raw_frames.len();
            if total_frames == 0 {
                return Err("GIF file contains no frames".into());
            }

            println!("==> Processing animated GIF {} ({} frames) with {} [{}]:", input.display(), total_frames, algo_name, pal.name);

            // Convert each frame to ImageBuffer and extract delay
            let frames_data: Result<Vec<(ImageBuffer, image::Delay)>, Box<dyn std::error::Error>> = raw_frames
                .into_iter()
                .map(|frame| {
                    let delay = frame.delay();
                    let (width, height) = (frame.buffer().width(), frame.buffer().height());
                    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                    for pixel in frame.buffer().pixels() {
                        rgb_data.push(pixel[0]);
                        rgb_data.push(pixel[1]);
                        rgb_data.push(pixel[2]);
                    }
                    let buf = ImageBuffer::from_raw(width, height, rgb_data)?;
                    Ok((buf, delay))
                })
                .collect();
            let frames_data = frames_data?;

            use rayon::prelude::*;
            let processed_frames: Result<Vec<(ImageBuffer, image::Delay)>, dither_core::DitherError> = frames_data
                .into_par_iter()
                .enumerate()
                .map(|(frame_idx, (buf, delay))| {
                    let mut current_buf = buf;
                    if let Some(ref pattern) = temporal_pattern {
                        current_buf = pattern.apply(&current_buf, frame_idx as u32, noise_intensity)?;
                    }
                    let algorithm = select_algorithm(&algo_name);
                    let dithered = algorithm.apply(&current_buf, &pal)?;
                    Ok((dithered, delay))
                })
                .collect();
            let processed_frames = processed_frames?;

            let out_file = std::fs::File::create(&output)?;
            let mut encoder = GifEncoder::new(std::io::BufWriter::new(out_file));
            encoder.set_repeat(Repeat::Infinite)?;

            for (dithered_buf, delay) in processed_frames {
                let width = dithered_buf.width;
                let height = dithered_buf.height;
                let mut rgba_raw = Vec::with_capacity((width * height * 4) as usize);
                for chunk in dithered_buf.data.chunks_exact(3) {
                    rgba_raw.push(chunk[0]);
                    rgba_raw.push(chunk[1]);
                    rgba_raw.push(chunk[2]);
                    rgba_raw.push(255);
                }
                let rgba_img = RgbaImage::from_raw(width, height, rgba_raw).ok_or("Failed to construct RGBA frame")?;
                let frame = Frame::from_parts(rgba_img, 0, 0, delay);
                encoder.encode_frame(frame)?;
            }

            let elapsed = start.elapsed();
            println!(
                "GIF complete: {} frames processed in {:.2}s ({:.1} fps) -> {}",
                total_frames,
                elapsed.as_secs_f64(),
                (total_frames as f64) / elapsed.as_secs_f64().max(0.001),
                output.display()
            );
        }

        Commands::Svg { input, output, threshold } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let isolator = BlackVectorIsolation { max_luminance_threshold: threshold, pixel_size: 1.0 };
            let svg = isolator.trace_to_svg(&img)?;
            std::fs::write(&output, svg)?;
            let elapsed = start.elapsed();
            println!("Isolated black vectors from {} into {} in {:.2}ms", input.display(), output.display(), elapsed.as_secs_f64() * 1000.0);
        }

        Commands::Batch { input_dir, output_dir, algo, palette, pattern } => {
            let start = Instant::now();
            let pal = select_palette(&palette);
            let algo_name = algo.clone();

            std::fs::create_dir_all(&output_dir)?;

            let mut paths = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&input_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && let Some(ext) = path.extension().and_then(|s| s.to_str())
                    {
                        match ext.to_lowercase().as_str() {
                            "png" | "jpg" | "jpeg" | "webp" | "bmp" => paths.push(path),
                            _ => {}
                        }
                    }
                }
            }

            let total_count = paths.len();
            println!("==> Batch processing {} images with {} [{}]:", total_count, algo_name, pal.name);

            use rayon::prelude::*;
            let processed_count: usize = paths
                .par_iter()
                .map(|in_path| {
                    let file_stem = in_path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
                    let timestamp =
                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default();
                    let safe_palette = pal.name.to_lowercase().replace(' ', "-");
                    let base_name = pattern
                        .replace("{name}", file_stem)
                        .replace("{algo}", &algo_name)
                        .replace("{palette}", &safe_palette)
                        .replace("{timestamp}", &timestamp);
                    let out_path = output_dir.join(format!("{base_name}.png"));

                    if let Ok(img) = load_image(in_path) {
                        let algorithm = select_algorithm(&algo_name);
                        if let Ok(dithered) = algorithm.apply(&img, &pal)
                            && save_image(&out_path, &dithered).is_ok()
                        {
                            return 1;
                        }
                    }
                    0
                })
                .sum();

            let elapsed = start.elapsed();
            println!(
                "Batch complete: {}/{} images processed in {:.2}s ({:.1} images/sec)",
                processed_count,
                total_count,
                elapsed.as_secs_f64(),
                (processed_count as f64) / elapsed.as_secs_f64().max(0.001)
            );
        }

        Commands::Mcp => {
            mcp::run_mcp_server()?;
        }

        Commands::Bench => {
            println!("==> Running DitherCraft Core Micro-Benchmarks (1080p RGB)...");
            let mut test_img = ImageBuffer::new(1920, 1080)?;
            for y in 0..1080 {
                for x in 0..1920 {
                    let r = ((x * 255) / 1920) as u8;
                    let g = ((y * 255) / 1080) as u8;
                    let b = (((x + y) * 255) / (1920 + 1080)) as u8;
                    test_img.set_pixel(x, y, RgbColor::new(r, g, b));
                }
            }
            let palette = Palette::game_boy();

            let algos: Vec<Box<dyn DitherAlgorithm>> = vec![
                Box::new(Bayer8x8),
                Box::new(InterleavedGradientNoise),
                Box::new(FloydSteinberg::default()),
                Box::new(Atkinson::default()),
                Box::new(PopTone::default()),
                Box::new(CmykHalftone::default()),
            ];

            for algo in algos {
                let start = Instant::now();
                let _ = algo.apply(&test_img, &palette)?;
                let duration = start.elapsed();
                println!("  {:<28}: {:.2} ms (FPS: {:.1})", algo.name(), duration.as_secs_f64() * 1000.0, 1.0 / duration.as_secs_f64());
            }
            println!("Benchmark complete.");
        }
    }

    Ok(())
}
