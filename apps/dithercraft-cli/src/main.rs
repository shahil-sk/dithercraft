#![allow(clippy::chunks_exact_to_as_chunks)]

mod mcp;

use clap::{Parser, Subcommand};
use dither_core::{
    AudioTrack, ColorHarmony, Cube3dLut, FlowFieldConfig, FlowFieldSteeredDither, GCodeConfig, GCodeExporter, HatchStyle, ImageBuffer, Mp4Demuxer,
    ReactionDiffusionConfig, ReactionDiffusionDither, ReactionDiffusionMorphology, ReactionDiffusionSimulator, SpotColorPlateSeparation, WebmDemuxer,
    algorithms::*, color::RgbColor, palette::Palette, renderers::*, temporal::TemporalNoisePattern, vector::BlackVectorIsolation,
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
    /// Process and export an animation as full 24-bit color APNG
    Apng {
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
    /// Inspect an MP4 container and list tracks, dimensions, codecs, and sample counts
    Mp4Info {
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Inspect a WebM / Matroska container and list tracks, dimensions, codecs, and blocks
    WebmInfo {
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Trim or slice a WAV audio file synchronized to target animation frames and FPS
    TrimAudio {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 60)]
        frames: u32,
        #[arg(long, default_value_t = 30)]
        fps: u32,
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
    /// Extract a quantized palette from an image and export to .gpl, .hex, .pal, .act, or .json
    /// Extract a quantized palette from an image and export to .gpl, .hex, .pal, .act, or .json
    ExtractPalette {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 16)]
        colors: usize,
        #[arg(long, default_value = "median-cut")]
        method: String,
    },
    /// Generate an OKLCH perceptual color harmony palette and export to .hex, .gpl, or .json
    GenerateHarmony {
        #[arg(short, long, default_value = "#ff4500")]
        base: String,
        #[arg(long, default_value = "triadic")]
        harmony: String,
        #[arg(short, long, default_value_t = 6)]
        count: usize,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Apply a 3D LUT (.cube file or built-in retro profile) to an image
    ApplyLut {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        cube: Option<PathBuf>,
        #[arg(long, default_value = "technicolor")]
        preset: String,
    },
    /// Export silkscreen or lithographic film positive spot color plates (.svg)
    ExportPlates {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output_dir: PathBuf,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
        #[arg(long, default_value_t = 1.0)]
        pixel_size: f32,
        #[arg(long, default_value_t = 30.0)]
        margin: f32,
    },
    /// Export process separation positive plates (.svg) for 4-color CMYK offset printing
    CmykSeparations {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output_dir: PathBuf,
        #[arg(long, default_value_t = 6.0)]
        freq: f32,
        #[arg(long, default_value = "round")]
        dot_shape: String,
        #[arg(long, default_value_t = 0.0)]
        gain: f32,
        #[arg(long, default_value_t = 1.0)]
        pixel_size: f32,
        #[arg(long, default_value_t = 30.0)]
        margin: f32,
    },
    /// Apply selective tonal band dithering (distinct algorithms for shadows, midtones, highlights)
    TonalSplit {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "bayer8x8")]
        shadow_algo: String,
        #[arg(long, default_value = "floyd-steinberg")]
        mid_algo: String,
        #[arg(long, default_value = "atkinson")]
        hi_algo: String,
        #[arg(short, long, default_value = "monochrome")]
        palette: String,
        #[arg(long, default_value_t = 85)]
        shadow_threshold: u8,
        #[arg(long, default_value_t = 170)]
        highlight_threshold: u8,
        #[arg(long, default_value_t = 16)]
        feather: u8,
    },
    /// Export CNC / pen plotter toolpath program (.gcode / .nc) with arc fitting and hatch infills
    Gcode {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 0.5)]
        scale: f32,
        #[arg(long, default_value_t = 128)]
        threshold: u8,
        #[arg(long, default_value = "horizontal")]
        hatch: String,
        #[arg(long, default_value_t = 0.5)]
        spacing: f32,
        #[arg(long, default_value_t = false)]
        arc_fitting: bool,
        #[arg(long, default_value_t = 0.1)]
        arc_tolerance: f32,
        #[arg(long, default_value_t = 0.5)]
        nib_width: f32,
        #[arg(long, default_value_t = false)]
        bi_directional: bool,
    },
    /// Synthesize biological reaction-diffusion patterns (Gray-Scott PDE) or dither an image with organic morphogenesis
    Morphogenesis {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "labyrinth")]
        morphology: String,
        #[arg(long, default_value_t = 80)]
        steps: usize,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
        #[arg(long, default_value_t = 256)]
        width: u32,
        #[arg(long, default_value_t = 256)]
        height: u32,
    },
    /// Apply vector flow field steered error diffusion along continuous streamlines
    FlowField {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value_t = 0.04)]
        scale: f32,
        #[arg(long, default_value_t = 1.5)]
        swirl: f32,
        #[arg(short, long, default_value = "gameboy")]
        palette: String,
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

fn select_dot_shape(name: &str) -> HalftoneDotShape {
    match name.to_lowercase().as_str() {
        "elliptical" | "ellipse" => HalftoneDotShape::Elliptical,
        "diamond" => HalftoneDotShape::Diamond,
        "line" => HalftoneDotShape::Line,
        _ => HalftoneDotShape::EuclideanRound,
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

        Commands::Apng { input, output, algo, palette, noise, noise_intensity } => {
            let start = Instant::now();
            let pal = select_palette(&palette);
            let algo_name = algo.clone();
            let temporal_noise = noise.as_deref().and_then(select_temporal_pattern);

            let file_in = std::fs::File::open(&input)?;
            let decoder = GifDecoder::new(std::io::BufReader::new(file_in))?;
            let raw_frames = decoder.into_frames().collect_frames()?;
            let total_frames = raw_frames.len();
            if total_frames == 0 {
                return Err("Input animation contains no frames".into());
            }

            let mut frames_data = Vec::with_capacity(total_frames);
            for frame in raw_frames {
                let (w, h) = (frame.buffer().width(), frame.buffer().height());
                let (num, denom) = frame.delay().numer_denom_ms();
                let delay_ms = num.checked_div(denom).map(|d| d.max(10)).unwrap_or(100);
                let mut rgb = Vec::with_capacity((w * h * 3) as usize);
                for px in frame.buffer().pixels() {
                    rgb.push(px[0]);
                    rgb.push(px[1]);
                    rgb.push(px[2]);
                }
                let buf = ImageBuffer::from_raw(w, h, rgb)?;
                frames_data.push((buf, delay_ms));
            }

            let first_w = frames_data[0].0.width;
            let first_h = frames_data[0].0.height;

            use rayon::prelude::*;
            let processed_frames: Result<Vec<(ImageBuffer, u32)>, dither_core::DitherError> = frames_data
                .into_par_iter()
                .enumerate()
                .map(|(idx, (buf, delay))| {
                    let mut current_buf = buf;
                    if let Some(ref pattern) = temporal_noise {
                        current_buf = pattern.apply(&current_buf, idx as u32, noise_intensity)?;
                    }
                    let algorithm = select_algorithm(&algo_name);
                    let dithered = algorithm.apply(&current_buf, &pal)?;
                    Ok((dithered, delay))
                })
                .collect();
            let processed = processed_frames?;

            let out_file = std::fs::File::create(&output)?;
            let mut writer_buf = std::io::BufWriter::new(out_file);
            let mut encoder = png::Encoder::new(&mut writer_buf, first_w, first_h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(total_frames as u32, 0)?;
            let mut png_writer = encoder.write_header()?;

            for (dithered_buf, delay_ms) in processed {
                let mut rgba = Vec::with_capacity((dithered_buf.width * dithered_buf.height * 4) as usize);
                for chunk in dithered_buf.data.chunks_exact(3) {
                    rgba.push(chunk[0]);
                    rgba.push(chunk[1]);
                    rgba.push(chunk[2]);
                    rgba.push(255);
                }
                png_writer.set_frame_delay(delay_ms as u16, 1000)?;
                png_writer.write_image_data(&rgba)?;
            }

            let elapsed = start.elapsed();
            println!(
                "APNG complete: {} frames processed in {:.2}s ({:.1} fps) -> {}",
                total_frames,
                elapsed.as_secs_f64(),
                (total_frames as f64) / elapsed.as_secs_f64().max(0.001),
                output.display()
            );
        }

        Commands::Mp4Info { input } => {
            let bytes = std::fs::read(&input)?;
            let demuxer = Mp4Demuxer::parse(&bytes)?;
            println!("MP4 Container: {}", input.display());
            println!("  Brand: {:?}", String::from_utf8_lossy(&demuxer.major_brand));
            println!("  Timescale: {}, Duration: {}", demuxer.timescale, demuxer.duration);
            println!("  Tracks: {}", demuxer.tracks.len());
            for (i, track) in demuxer.tracks.iter().enumerate() {
                println!(
                    "    Track #{}: ID={}, {}x{}, Codec={}, Samples={}, FPS={:.2}",
                    i + 1,
                    track.track_id,
                    track.width,
                    track.height,
                    track.codec.as_str(),
                    track.samples.len(),
                    track.nominal_fps()
                );
            }
        }

        Commands::WebmInfo { input } => {
            let bytes = std::fs::read(&input)?;
            let demuxer = WebmDemuxer::parse(&bytes)?;
            println!("WebM Container: {}", input.display());
            println!("  Timecode Scale: {} ns", demuxer.timecode_scale);
            println!("  Tracks: {}", demuxer.tracks.len());
            for track in &demuxer.tracks {
                let type_str = if track.track_type == 1 {
                    "Video"
                } else if track.track_type == 2 {
                    "Audio"
                } else {
                    "Other"
                };
                println!(
                    "    Track #{}: Type={}, {}x{}, Codec={}, Blocks={}",
                    track.track_number,
                    type_str,
                    track.width,
                    track.height,
                    track.codec_id,
                    track.samples.len()
                );
            }
        }

        Commands::TrimAudio { input, output, frames, fps } => {
            let bytes = std::fs::read(&input)?;
            let track = AudioTrack::parse_wav(&bytes)?;
            let slice = track.export_wav_slice(frames, fps);
            std::fs::write(&output, slice)?;
            let duration_sec = frames as f64 / fps.max(1) as f64;
            println!("Trimmed WAV audio from {} -> {} ({} frames @ {} FPS = {:.2}s)", input.display(), output.display(), frames, fps, duration_sec);
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

        Commands::ExtractPalette { input, output, colors, method } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let pal = match method.to_lowercase().as_str() {
                "kmeans" | "k-means" => Palette::from_image_kmeans("Extracted Palette", &img, colors, 10)?,
                _ => Palette::from_image_median_cut("Extracted Palette", &img, colors)?,
            };
            pal.export_to_file(&output)?;
            let elapsed = start.elapsed();
            println!(
                "Extracted {} colors using {} from {} -> {} in {:.2}ms",
                pal.len(),
                method,
                input.display(),
                output.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::GenerateHarmony { base, harmony, count, output } => {
            let start = Instant::now();
            let base_rgb = RgbColor::from_hex(&base).ok_or("invalid hex color string")?;
            let harm = match harmony.to_lowercase().as_str() {
                "complementary" | "comp" => ColorHarmony::Complementary,
                "analogous" => ColorHarmony::Analogous,
                "split" | "split-complementary" | "splitcomplementary" => ColorHarmony::SplitComplementary,
                "tetradic" | "square" => ColorHarmony::Tetradic,
                "monochromatic" | "mono" => ColorHarmony::Monochromatic,
                _ => ColorHarmony::Triadic,
            };
            let pal = Palette::from_harmony(base_rgb, harm, count);
            pal.export_to_file(&output)?;
            let elapsed = start.elapsed();
            println!(
                "Generated {} color palette using {} harmony around {} -> {} in {:.2}ms",
                pal.len(),
                harm.name(),
                base,
                output.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::ApplyLut { input, output, cube, preset } => {
            let start = Instant::now();
            let mut img = load_image(&input)?;
            let lut = if let Some(cube_path) = cube {
                let content = std::fs::read_to_string(&cube_path)?;
                Cube3dLut::from_cube_str(&content)?
            } else {
                match preset.to_lowercase().replace(['-', '_'], "").as_str() {
                    "vintage" | "vintagewarm" | "1970" => Cube3dLut::vintage_warm_1970(32),
                    "amber" | "amberphosphor" => Cube3dLut::amber_phosphor(32),
                    "cyber" | "cybermatrix" | "matrix" => Cube3dLut::cyber_matrix(32),
                    "bleach" | "bleachbypass" => Cube3dLut::bleach_bypass(32),
                    _ => Cube3dLut::technicolor_2strip(32),
                }
            };
            println!("Applying 3D LUT '{}' (size {}^3)...", lut.title, lut.size);
            lut.apply_buffer(&mut img);
            save_image(&output, &img)?;
            let elapsed = start.elapsed();
            println!("Processed {} -> {} in {:.2}ms", input.display(), output.display(), elapsed.as_secs_f64() * 1000.0);
        }

        Commands::ExportPlates { input, output_dir, palette, pixel_size, margin } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let pal = select_palette(&palette);
            let sep = SpotColorPlateSeparation {
                pixel_size,
                margin,
                include_registration_marks: true,
                include_calibration_wedge: true,
                include_slug: true,
            };
            std::fs::create_dir_all(&output_dir)?;
            let plates = sep.generate_all_plates(&img, &pal)?;
            for (filename, svg_data) in &plates {
                let out_file = output_dir.join(filename);
                std::fs::write(&out_file, svg_data)?;
            }
            let elapsed = start.elapsed();
            println!("Exported {} spot color plates to {} in {:.2}ms", plates.len(), output_dir.display(), elapsed.as_secs_f64() * 1000.0);
        }

        Commands::CmykSeparations { input, output_dir, freq, dot_shape, gain, pixel_size, margin } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let shape = select_dot_shape(&dot_shape);
            let cmyk = CmykHalftone { frequency: freq, dot_shape: shape, dot_gain: gain, ..Default::default() };

            let seps = cmyk.generate_separations(&img)?;
            std::fs::create_dir_all(&output_dir)?;
            let plates = seps.export_all_svg_plates(&cmyk, margin, pixel_size)?;
            for (filename, svg_data) in &plates {
                let out_file = output_dir.join(filename);
                std::fs::write(&out_file, svg_data)?;
            }
            let elapsed = start.elapsed();
            println!(
                "Exported 4 CMYK separation plates (shape: {:?}, gain: {:.1}%) to {} in {:.2}ms",
                shape,
                gain * 100.0,
                output_dir.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::TonalSplit { input, output, shadow_algo, mid_algo, hi_algo, palette, shadow_threshold, highlight_threshold, feather } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let pal = select_palette(&palette);
            let shadow = select_algorithm(&shadow_algo);
            let mid = select_algorithm(&mid_algo);
            let highlight = select_algorithm(&hi_algo);

            let tonal = TonalBandDither::new(shadow_threshold, highlight_threshold, feather, shadow, mid, highlight);
            let dithered = tonal.apply(&img, &pal)?;
            save_image(&output, &dithered)?;
            let elapsed = start.elapsed();
            println!(
                "Successfully applied tonal split (shadows: {}, mids: {}, highlights: {}) to {} -> {} in {:.2}ms",
                shadow_algo,
                mid_algo,
                hi_algo,
                input.display(),
                output.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::Gcode { input, output, scale, threshold, hatch, spacing, arc_fitting, arc_tolerance, nib_width, bi_directional } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let hatch_style = match hatch.to_lowercase().as_str() {
                "vertical" | "vert" => HatchStyle::Vertical,
                "cross" | "crosshatch" | "crosshatch45" => HatchStyle::CrossHatch45,
                "stipple" | "poisson" => HatchStyle::StipplePoisson,
                _ => HatchStyle::Horizontal,
            };

            let config = GCodeConfig {
                scale_mm_per_pixel: scale,
                threshold,
                hatch_style,
                hatch_spacing_mm: spacing,
                arc_fitting,
                arc_tolerance,
                nib_width_mm: nib_width,
                bi_directional_offset: bi_directional,
                ..Default::default()
            };

            let gcode = GCodeExporter::generate_gcode(&img, &config)?;
            std::fs::write(&output, gcode)?;
            let elapsed = start.elapsed();
            println!(
                "Generated CNC G-code ({:?} hatch, arcs: {}) from {} -> {} in {:.2}ms",
                hatch_style,
                arc_fitting,
                input.display(),
                output.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::Morphogenesis { input, output, morphology, steps, palette, width, height } => {
            let start = Instant::now();
            let pal = select_palette(&palette);
            let morph = match morphology.to_lowercase().as_str() {
                "spots" => ReactionDiffusionMorphology::Spots,
                "solitons" => ReactionDiffusionMorphology::Solitons,
                "waves" => ReactionDiffusionMorphology::PulsatingWaves,
                "fingerprints" => ReactionDiffusionMorphology::Fingerprints,
                "coral" => ReactionDiffusionMorphology::Coral,
                _ => ReactionDiffusionMorphology::Labyrinth,
            };
            let config = ReactionDiffusionConfig { morphology: morph, steps, ..Default::default() };

            let out_buf = if let Some(in_path) = input {
                let img = load_image(&in_path)?;
                let dither = ReactionDiffusionDither::new(config);
                dither.apply(&img, &pal)?
            } else {
                let mut sim = ReactionDiffusionSimulator::new(width as usize, height as usize, config);
                sim.step(steps);
                let raw_img = sim.to_image_buffer()?;
                let algo = FloydSteinberg::default();
                algo.apply(&raw_img, &pal)?
            };

            save_image(&output, &out_buf)?;
            let elapsed = start.elapsed();
            println!(
                "Generated Reaction-Diffusion morphogenesis ({morphology}, {steps} steps) -> {} in {:.2}ms",
                output.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }

        Commands::FlowField { input, output, scale, swirl, palette } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let pal = select_palette(&palette);
            let config = FlowFieldConfig { scale, swirl, angle_offset: 0.0 };
            let dither = FlowFieldSteeredDither::new(config);
            let out_buf = dither.apply(&img, &pal)?;
            save_image(&output, &out_buf)?;
            let elapsed = start.elapsed();
            println!(
                "Applied Vector Flow Field steered dithering (scale: {scale}, swirl: {swirl}) -> {} in {:.2}ms",
                output.display(),
                elapsed.as_secs_f64() * 1000.0
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
