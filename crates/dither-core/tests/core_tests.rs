use dither_core::{ImageBuffer, algorithms::*, color::RgbColor, effects::*, palette::Palette, vector::BlackVectorIsolation};

#[test]
fn test_image_buffer_creation_and_pixel_access() {
    let mut buf = ImageBuffer::new(10, 10).expect("create buffer");
    assert_eq!(buf.width, 10);
    assert_eq!(buf.height, 10);
    assert_eq!(buf.data.len(), 300);

    assert!(buf.set_pixel(2, 3, RgbColor::new(120, 200, 255)));
    let px = buf.get_pixel(2, 3).expect("get pixel");
    assert_eq!(px, RgbColor::new(120, 200, 255));

    // Bounds checking
    assert_eq!(buf.get_pixel(10, 10), None);
    assert!(!buf.set_pixel(10, 10, RgbColor::WHITE));

    // Clamped fetch
    let clamped = buf.get_pixel_clamped(-5, -5);
    assert_eq!(clamped, RgbColor::BLACK);
}

#[test]
fn test_rgb_color_luminance_and_distance() {
    let black = RgbColor::BLACK;
    let white = RgbColor::WHITE;
    assert_eq!(black.luminance(), 0);
    assert_eq!(white.luminance(), 255);

    let red = RgbColor::new(255, 0, 0);
    let green = RgbColor::new(0, 255, 0);
    assert!(green.luminance() > red.luminance()); // Green has higher perceived luminosity

    let hex = RgbColor::from_hex("#FF8000").expect("hex parse");
    assert_eq!(hex, RgbColor::new(255, 128, 0));
    assert_eq!(hex.to_hex(), "#FF8000");
}

#[test]
fn test_palette_swatch_locking_and_tonal_sorting() {
    let mut pal = Palette::game_boy();
    assert_eq!(pal.len(), 4);

    // Lock swatch 0
    assert!(pal.set_locked(0, true));
    let original_swatch0 = pal.swatches[0].color;

    // Randomize unlocked
    pal.randomize_unlocked(123456);
    assert_eq!(pal.swatches[0].color, original_swatch0); // Locked swatch preserved

    // Tonal sorting
    pal.sort_by_luminance(true);
    for i in 0..pal.len() - 1 {
        assert!(pal.swatches[i].color.luminance() <= pal.swatches[i + 1].color.luminance());
    }

    // Tonal cycling
    let cycled = pal.cycle_tonal(1);
    assert_eq!(cycled.len(), pal.len());
}

#[test]
fn test_ordered_bayer_and_error_diffusion() {
    let mut input = ImageBuffer::new(16, 16).expect("buf");
    for y in 0..16 {
        for x in 0..16 {
            let lum = ((x + y) * 8) as u8;
            input.set_pixel(x, y, RgbColor::new(lum, lum, lum));
        }
    }

    let pal = Palette::monochrome();

    // Bayer 8x8
    let bayer = Bayer8x8;
    let out_bayer = bayer.apply(&input, &pal).expect("apply bayer");
    assert_eq!(out_bayer.width, 16);
    assert_eq!(out_bayer.height, 16);

    // Floyd-Steinberg
    let fs = FloydSteinberg::default();
    let out_fs = fs.apply(&input, &pal).expect("apply floyd-steinberg");
    assert_eq!(out_fs.width, 16);
    assert_eq!(out_fs.height, 16);

    // Verify all output colors belong to the target palette
    let colors = pal.colors();
    for y in 0..16 {
        for x in 0..16 {
            let px = out_fs.get_pixel(x, y).expect("px");
            assert!(colors.contains(&px));
        }
    }
}

#[test]
fn test_black_vector_isolation_svg() {
    let mut input = ImageBuffer::new(8, 8).expect("buf");
    // Place a 4x4 black square in the middle
    for y in 2..6 {
        for x in 2..6 {
            input.set_pixel(x, y, RgbColor::BLACK);
        }
    }

    let isolator = BlackVectorIsolation { max_luminance_threshold: 128, pixel_size: 2.0 };
    let svg = isolator.trace_to_svg(&input).expect("trace svg");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("fill=\"#000000\""));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn test_stackable_effects() {
    let mut input = ImageBuffer::new(16, 16).expect("buf");
    input.set_pixel(8, 8, RgbColor::WHITE);

    let glow = EpsilonGlow { threshold: 200, radius: 2, intensity: 1.0 };
    let out_glow = glow.apply(&input).expect("apply glow");
    assert_eq!(out_glow.width, 16);

    let chromatic = ChromaticAberration { shift_x: 2, shift_y: 0, radial: false };
    let out_chromatic = chromatic.apply(&input).expect("apply chromatic");
    assert_eq!(out_chromatic.width, 16);

    let glitch = JpegGlitchCore { mode: JpegSubMode::BlockShift, intensity: 0.5, block_size: 8, seed: 42 };
    let out_glitch = glitch.apply(&input).expect("apply glitch");
    assert_eq!(out_glitch.width, 16);
}
