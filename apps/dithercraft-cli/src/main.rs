use clap::{Parser, Subcommand};
use dither_core::{ImageBuffer, algorithms::*, color::RgbColor, palette::Palette, renderers::*, vector::BlackVectorIsolation};
use image::ImageReader;
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
    /// Export darkest pixels as an isolated vector path SVG
    Svg {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value_t = 128)]
        threshold: u8,
    },
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
    match name.to_lowercase().as_str() {
        "mono" | "monochrome" | "1bit" => Palette::monochrome(),
        "cga" => Palette::cga_mode_1(),
        "cyberpunk" => Palette::cyberpunk(),
        _ => Palette::game_boy(),
    }
}

fn select_algorithm(name: &str) -> Box<dyn DitherAlgorithm> {
    match name.to_lowercase().as_str() {
        "bayer" | "bayer8" | "bayer8x8" => Box::new(Bayer8x8),
        "bayer4" | "bayer4x4" => Box::new(Bayer4x4),
        "bayer16" | "bayer16x16" => Box::new(Bayer16x16),
        "atkinson" => Box::new(Atkinson::default()),
        "burkes" => Box::new(Burkes::default()),
        "sierra" | "sierra3" => Box::new(Sierra3::default()),
        "sierra-lite" => Box::new(SierraLite::default()),
        "stucki" => Box::new(Stucki::default()),
        "cmyk" | "cmyk-halftone" => Box::new(CmykHalftone::default()),
        "pop-tone" => Box::new(PopTone::default()),
        "ign" => Box::new(InterleavedGradientNoise),
        "otsu" => Box::new(OtsuThreshold),
        _ => Box::new(FloydSteinberg::default()),
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

        Commands::Svg { input, output, threshold } => {
            let start = Instant::now();
            let img = load_image(&input)?;
            let isolator = BlackVectorIsolation { max_luminance_threshold: threshold, pixel_size: 1.0 };
            let svg = isolator.trace_to_svg(&img)?;
            std::fs::write(&output, svg)?;
            let elapsed = start.elapsed();
            println!("Isolated black vectors from {} into {} in {:.2}ms", input.display(), output.display(), elapsed.as_secs_f64() * 1000.0);
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
