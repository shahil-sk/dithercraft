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

    let chromatic_radial = ChromaticAberration { shift_x: 2, shift_y: 2, radial: true };
    let out_chromatic_r = chromatic_radial.apply(&input).expect("apply radial chromatic");
    assert_eq!(out_chromatic_r.width, 16);

    for mode in
        [JpegSubMode::BlockShift, JpegSubMode::ChannelSwap, JpegSubMode::ScanLineOffset, JpegSubMode::BlockScramble, JpegSubMode::InterlaceCorruption]
    {
        let glitch = JpegGlitchCore { mode, intensity: 0.5, block_size: 8, seed: 42 };
        let out_glitch = glitch.apply(&input).expect("apply glitch");
        assert_eq!(out_glitch.width, 16);
    }

    let crt = VintageCrtSimulation::default();
    let out_crt = crt.apply(&input).expect("apply crt");
    assert_eq!(out_crt.width, 16);
    assert_eq!(out_crt.height, 16);

    let blur = GaussianBlur::new(1.2);
    let out_blur = blur.apply(&input).expect("apply blur");
    assert_eq!(out_blur.width, 16);
    assert_eq!(out_blur.height, 16);

    let sharpen = Sharpen::new(1.0, 1.0);
    let out_sharpen = sharpen.apply(&input).expect("apply sharpen");
    assert_eq!(out_sharpen.width, 16);
    assert_eq!(out_sharpen.height, 16);
}

#[test]
fn test_all_63_algorithms_instantiation_and_execution() {
    let mut input = ImageBuffer::new(32, 32).expect("create test buffer");
    for y in 0..32 {
        for x in 0..32 {
            let r = (x * 8) as u8;
            let g = (y * 8) as u8;
            let b = ((x + y) * 4) as u8;
            input.set_pixel(x, y, RgbColor::new(r, g, b));
        }
    }

    let pal = Palette::game_boy();
    let empty_pal = Palette { name: "empty".into(), swatches: vec![], metric: dither_core::ColorMetric::default() };

    assert!(ALL_ALGORITHMS.len() >= 63, "Expected at least 63 algorithms, got {}", ALL_ALGORITHMS.len());

    for info in ALL_ALGORITHMS {
        let algo_opt = create_algorithm(info.name);
        assert!(algo_opt.is_some(), "Failed to instantiate algorithm: {}", info.name);
        let algo = algo_opt.unwrap();

        // 1. Successful execution
        let res = algo.apply(&input, &pal);
        assert!(res.is_ok(), "Algorithm {} failed: {:?}", info.name, res.err());
        let out = res.unwrap();
        assert_eq!(out.width, 32, "Width mismatch for {}", info.name);
        assert_eq!(out.height, 32, "Height mismatch for {}", info.name);
        assert_eq!(out.data.len(), 32 * 32 * 3, "Buffer len mismatch for {}", info.name);

        // 2. Empty palette error handling (never panic)
        let empty_res = algo.apply(&input, &empty_pal);
        assert!(empty_res.is_err(), "Algorithm {} should reject empty palette", info.name);
    }
}

#[test]
fn test_cielab_color_distance() {
    let black = RgbColor::BLACK;
    let white = RgbColor::WHITE;
    let lab_b = black.to_lab();
    let lab_w = white.to_lab();

    assert!((lab_b.l - 0.0).abs() < 1e-3);
    assert!((lab_w.l - 100.0).abs() < 1e-1);

    let de = black.distance_cielab_de(&white);
    assert!(de > 99.0); // Extreme distance in Lab space
}

#[test]
fn test_bundled_retro_palettes() {
    let mono = Palette::monochrome();
    assert_eq!(mono.len(), 2);

    let mac = Palette::mac_1bit();
    assert_eq!(mac.len(), 2);

    let gb = Palette::game_boy();
    assert_eq!(gb.len(), 4);

    let cga = Palette::cga_mode_1();
    assert_eq!(cga.len(), 4);

    let cyber = Palette::cyberpunk();
    assert_eq!(cyber.len(), 4);

    let apple = Palette::apple_ii();
    assert_eq!(apple.len(), 16);

    let c64 = Palette::commodore_64();
    assert_eq!(c64.len(), 16);

    let nes = Palette::nes();
    assert_eq!(nes.len(), 16);

    let pico = Palette::pico_8();
    assert_eq!(pico.len(), 16);
}

#[test]
fn test_palette_parsers_json_hex_gpl_pal() {
    // 1. JSON parse
    let json_str = r##"["#000000", "#FF0000", "#00FF00", "#0000FF"]"##;
    let pal_json = Palette::parse_json(json_str).expect("parse json");
    assert_eq!(pal_json.len(), 4);

    // 2. HEX parse
    let hex_str = "#112233\n#445566\n#AABBCC\n; comment line\n";
    let pal_hex = Palette::parse_hex("Custom Hex", hex_str).expect("parse hex");
    assert_eq!(pal_hex.len(), 3);
    assert_eq!(pal_hex.name, "Custom Hex");

    // 3. GPL parse
    let gpl_str = "GIMP Palette\nName: TestGPL\nColumns: 4\n#\n  0   0   0 Black\n255 255 255 White\n";
    let pal_gpl = Palette::parse_gpl(gpl_str).expect("parse gpl");
    assert_eq!(pal_gpl.len(), 2);
    assert_eq!(pal_gpl.name, "TestGPL");

    // 4. JASC-PAL parse
    let pal_str = "JASC-PAL\n0100\n2\n0 0 0\n255 0 255\n";
    let pal_jasc = Palette::parse_pal(pal_str).expect("parse pal");
    assert_eq!(pal_jasc.len(), 2);
}

#[test]
fn test_color_extraction_median_cut_and_kmeans() {
    let mut img = ImageBuffer::new(64, 64).expect("buf");
    // Create an image with 4 distinct colored quadrants
    for y in 0..64 {
        for x in 0..64 {
            let color = match (x < 32, y < 32) {
                (true, true) => RgbColor::new(255, 0, 0),     // Red
                (false, true) => RgbColor::new(0, 255, 0),    // Green
                (true, false) => RgbColor::new(0, 0, 255),    // Blue
                (false, false) => RgbColor::new(255, 255, 0), // Yellow
            };
            img.set_pixel(x, y, color);
        }
    }

    // 1. Median Cut extraction
    let med_pal = Palette::from_image_median_cut("MedianExtracted", &img, 4).expect("median cut");
    assert_eq!(med_pal.len(), 4);

    // 2. K-Means extraction
    let kmeans_pal = Palette::from_image_kmeans("KMeansExtracted", &img, 4, 10).expect("kmeans");
    assert_eq!(kmeans_pal.len(), 4);
}

#[test]
fn test_palette_kd_tree_acceleration() {
    let pal = Palette::cga_mode_1();
    let tree = pal.to_kd_tree();

    let target = RgbColor::new(50, 240, 230); // Close to cyan (#55FFFF)
    let nearest_linear = pal.find_nearest_with_metric(&target, dither_core::ColorMetric::EuclideanRgb);
    let nearest_tree = tree.find_nearest(&target).expect("kd-tree nearest");

    assert_eq!(nearest_linear, nearest_tree);
}
