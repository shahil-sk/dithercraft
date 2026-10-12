use dither_core::{
    ImageBuffer,
    algorithms::*,
    color::RgbColor,
    effects::*,
    palette::{ColorMetric, Palette},
    vector::BlackVectorIsolation,
};

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
fn test_image_buffer_downsampling() {
    let mut buf = ImageBuffer::new(100, 100).expect("create buffer");
    for y in 0..100 {
        for x in 0..100 {
            buf.set_pixel(x, y, RgbColor::new(200, 100, 50));
        }
    }

    // Downsample to 20x20
    let down = buf.downsample(20, 20).expect("downsample");
    assert_eq!(down.width, 20);
    assert_eq!(down.height, 20);
    let px = down.get_pixel(10, 10).expect("pixel");
    assert_eq!(px.r, 200);
    assert_eq!(px.g, 100);
    assert_eq!(px.b, 50);

    // Aspect ratio preservation: 100x50 -> max(20, 20) => 20x10
    let mut buf_rect = ImageBuffer::new(100, 50).expect("rect buffer");
    buf_rect.set_pixel(0, 0, RgbColor::WHITE);
    let down_rect = buf_rect.downsample(20, 20).expect("downsample rect");
    assert_eq!(down_rect.width, 20);
    assert_eq!(down_rect.height, 10);

    // If already smaller, preserves exact buffer
    let smaller = down_rect.downsample(50, 50).expect("noop");
    assert_eq!(smaller.width, 20);
    assert_eq!(smaller.height, 10);
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
fn test_all_64_algorithms_instantiation_and_execution() {
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

    assert_eq!(ALL_ALGORITHMS.len(), 64, "Expected exactly 64 algorithms, got {}", ALL_ALGORITHMS.len());

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

    // 5. ACT (Adobe Color Table) export and parse roundtrip
    let gb = Palette::game_boy();
    let act_bytes = gb.export_act();
    assert_eq!(act_bytes.len(), 772);
    let pal_act = Palette::parse_act("GameBoy ACT", &act_bytes).expect("parse act");
    assert_eq!(pal_act.len(), 4);
    assert_eq!(pal_act.swatches[0].color, gb.swatches[0].color);

    // 6. GPL export and parse roundtrip
    let gpl_text = gb.export_gpl();
    let pal_gpl_rt = Palette::parse_gpl(&gpl_text).expect("gpl roundtrip");
    assert_eq!(pal_gpl_rt.len(), gb.len());

    // 7. HEX export and parse roundtrip
    let hex_text = gb.export_hex();
    let pal_hex_rt = Palette::parse_hex("Hex RT", &hex_text).expect("hex roundtrip");
    assert_eq!(pal_hex_rt.len(), gb.len());

    // 8. PAL export and parse roundtrip
    let pal_text = gb.export_pal();
    let pal_jasc_rt = Palette::parse_pal(&pal_text).expect("pal roundtrip");
    assert_eq!(pal_jasc_rt.len(), gb.len());
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

#[test]
fn test_temporal_tracks_lfo_and_animation_sequence() {
    use dither_core::temporal::*;

    // 1. TimelineTrack Keyframing
    let mut track = TimelineTrack::new("brightness", 0.0);
    track.interpolation = Interpolation::Linear;
    track.add_keyframe(0, 0.0);
    track.add_keyframe(10, 100.0);
    track.add_keyframe(20, 50.0);

    assert_eq!(track.eval(0), 0.0);
    assert_eq!(track.eval(5), 50.0);
    assert_eq!(track.eval(10), 100.0);
    assert_eq!(track.eval(15), 75.0);
    assert_eq!(track.eval(20), 50.0);
    assert_eq!(track.eval(30), 50.0); // Clamped at end

    // SmoothStep interpolation
    track.interpolation = Interpolation::SmoothStep;
    assert_eq!(track.eval(0), 0.0);
    assert_eq!(track.eval(5), 50.0); // Symmetric midpoint
    assert_eq!(track.eval(10), 100.0);

    // 2. LFO Modulator
    let lfo_sine = LfoModulator::new(LfoWaveform::Sine, 0.1, 10.0, 50.0);
    let val_0 = lfo_sine.eval(0);
    assert!((val_0 - 50.0).abs() < 1e-4);

    let lfo_sq = LfoModulator::new(LfoWaveform::Square, 0.1, 5.0, 10.0);
    assert_eq!(lfo_sq.eval(0), 15.0);
    assert_eq!(lfo_sq.eval(6), 5.0);

    // 3. AnimationSequence
    let mut seq = AnimationSequence::new();
    let frame1 = ImageBuffer::new(4, 4).expect("buf");
    let frame2 = ImageBuffer::new(4, 4).expect("buf");
    seq.add_frame(frame1, 100);
    seq.add_frame(frame2, 200);

    assert_eq!(seq.len(), 2);
    assert_eq!(seq.total_duration_ms(), 300);
    assert_eq!(seq.frame_index_at_time(50, true), 0);
    assert_eq!(seq.frame_index_at_time(150, true), 1);
    assert_eq!(seq.frame_index_at_time(350, true), 0); // Looping wraps around

    // Parallel processing across frames
    let processed = seq
        .process_frames(|buf, idx| {
            let mut out = buf.clone();
            out.set_pixel(0, 0, RgbColor::new((idx * 50) as u8, 0, 0));
            Ok(out)
        })
        .expect("process frames");

    assert_eq!(processed.len(), 2);
    assert_eq!(processed.frames[0].buffer.get_pixel(0, 0), Some(RgbColor::new(0, 0, 0)));
    assert_eq!(processed.frames[1].buffer.get_pixel(0, 0), Some(RgbColor::new(50, 0, 0)));
}

#[test]
fn test_all_9_temporal_noise_patterns() {
    let mut buf = ImageBuffer::new(16, 16).expect("buf");
    for y in 0..16 {
        for x in 0..16 {
            buf.set_pixel(x, y, RgbColor::new(128, 128, 128));
        }
    }

    for pat in dither_core::temporal::TemporalNoisePattern::ALL {
        let applied = pat.apply(&buf, 5, 0.5).expect("apply pattern");
        assert_eq!(applied.width, 16);
        assert_eq!(applied.height, 16);
        assert_eq!(applied.data.len(), 16 * 16 * 3);
        assert!(!pat.name().is_empty());
    }
}

#[test]
fn test_oklab_color_distance_and_perceptual_metric() {
    let red = RgbColor::new(255, 0, 0);
    let dark_red = RgbColor::new(180, 0, 0);
    let blue = RgbColor::new(0, 0, 255);

    let ok_red = red.to_oklab();
    let ok_blue = blue.to_oklab();
    assert!(ok_red.l > 0.0 && ok_red.l <= 1.0);
    assert!(ok_blue.l > 0.0 && ok_blue.l <= 1.0);

    let d_red_darkred = red.distance_oklab(&dark_red);
    let d_red_blue = red.distance_oklab(&blue);
    assert!(d_red_darkred < d_red_blue);

    // Test Palette nearest color lookup with OKLab
    let pal = Palette::new("Test", vec![RgbColor::new(0, 0, 0), RgbColor::new(255, 0, 0), RgbColor::new(0, 0, 255)]).expect("pal");
    let nearest = pal.find_nearest_with_metric(&RgbColor::new(200, 10, 10), ColorMetric::Oklab);
    assert_eq!(nearest, RgbColor::new(255, 0, 0));
}

#[test]
fn test_ciede2000_color_distance_and_perceptual_metric() {
    let red = RgbColor::new(255, 0, 0);
    let dark_red = RgbColor::new(180, 0, 0);
    let blue = RgbColor::new(0, 0, 255);

    let d_same = red.distance_ciede2000(&red);
    assert!(d_same < 1e-4);

    let d_red_darkred = red.distance_ciede2000(&dark_red);
    let d_red_blue = red.distance_ciede2000(&blue);
    assert!(d_red_darkred > 0.0);
    assert!(d_red_darkred < d_red_blue);

    // Test Palette nearest color lookup with CIEDE2000
    let pal = Palette::new("Test", vec![RgbColor::new(0, 0, 0), RgbColor::new(255, 0, 0), RgbColor::new(0, 0, 255)]).expect("pal");
    let nearest = pal.find_nearest_with_metric(&RgbColor::new(200, 10, 10), ColorMetric::Ciede2000);
    assert_eq!(nearest, RgbColor::new(255, 0, 0));
}

#[test]
fn test_rdp_polyline_simplification() {
    use dither_core::vector::{Point2D, rdp_simplify};

    // Collinear points along a straight line: should reduce to endpoints
    let straight_line = vec![Point2D::new(0.0, 0.0), Point2D::new(1.0, 1.0), Point2D::new(2.0, 2.0), Point2D::new(3.0, 3.0), Point2D::new(4.0, 4.0)];
    let simplified = rdp_simplify(&straight_line, 0.1);
    assert_eq!(simplified.len(), 2);
    assert_eq!(simplified[0], Point2D::new(0.0, 0.0));
    assert_eq!(simplified[1], Point2D::new(4.0, 4.0));

    // Sharp corner in middle: must preserve corner
    let corner = vec![Point2D::new(0.0, 0.0), Point2D::new(2.0, 2.0), Point2D::new(4.0, 0.0)];
    let preserved = rdp_simplify(&corner, 0.5);
    assert_eq!(preserved.len(), 3);
}

#[test]
fn test_multicolor_svg_layer_separation() {
    use dither_core::vector::MultiColorSvgSeparation;

    let mut buf = ImageBuffer::new(4, 4).expect("buf");
    // Draw top half red, bottom half blue
    for x in 0..4 {
        buf.set_pixel(x, 0, RgbColor::new(255, 0, 0));
        buf.set_pixel(x, 1, RgbColor::new(255, 0, 0));
        buf.set_pixel(x, 2, RgbColor::new(0, 0, 255));
        buf.set_pixel(x, 3, RgbColor::new(0, 0, 255));
    }

    let pal = Palette::new("RedBlue", vec![RgbColor::new(255, 0, 0), RgbColor::new(0, 0, 255)]).expect("pal");
    let exporter = MultiColorSvgSeparation::default();
    let svg = exporter.trace_palette_layers(&buf, &pal).expect("svg");

    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
    assert!(svg.contains(r##"fill="#FF0000""##));
    assert!(svg.contains(r##"fill="#0000FF""##));
}

#[test]
fn test_gcode_toolpath_generation_and_optimization() {
    use dither_core::vector::{GCodeConfig, GCodeExporter};

    let mut buf = ImageBuffer::new(8, 8).expect("buf");
    // Draw 2 horizontal black bars
    for x in 1..7 {
        buf.set_pixel(x, 2, RgbColor::new(0, 0, 0));
        buf.set_pixel(x, 5, RgbColor::new(0, 0, 0));
    }

    let config = GCodeConfig::default();
    let gcode = GCodeExporter::generate_gcode(&buf, &config).expect("gcode");

    assert!(gcode.contains("G21 ; Units: mm"));
    assert!(gcode.contains("G90 ; Absolute positioning"));
    assert!(gcode.contains("G00 Z"));
    assert!(gcode.contains("G01 Z"));
    assert!(gcode.contains("M02 ; End of program"));
}

#[test]
fn test_spot_color_plate_separation() {
    use dither_core::vector::SpotColorPlateSeparation;

    let mut buf = ImageBuffer::new(6, 6).expect("buf");
    for x in 0..6 {
        buf.set_pixel(x, 0, RgbColor::new(255, 0, 0));
        buf.set_pixel(x, 1, RgbColor::new(255, 0, 0));
        buf.set_pixel(x, 2, RgbColor::new(0, 255, 0));
        buf.set_pixel(x, 3, RgbColor::new(0, 255, 0));
        buf.set_pixel(x, 4, RgbColor::new(0, 0, 255));
        buf.set_pixel(x, 5, RgbColor::new(0, 0, 255));
    }

    let pal = Palette::new("RGB_Trio", vec![RgbColor::new(255, 0, 0), RgbColor::new(0, 255, 0), RgbColor::new(0, 0, 255)]).expect("pal");

    let sep = SpotColorPlateSeparation::default();
    let plates = sep.generate_all_plates(&buf, &pal).expect("plates");

    assert_eq!(plates.len(), 3);
    assert_eq!(plates[0].0, "plate_01_FF0000.svg");
    assert_eq!(plates[1].0, "plate_02_00FF00.svg");
    assert_eq!(plates[2].0, "plate_03_0000FF.svg");

    let plate1_svg = &plates[0].1;
    assert!(plate1_svg.contains("<svg"));
    assert!(plate1_svg.contains("</svg>"));
    assert!(plate1_svg.contains("Registration Targets"));
    assert!(plate1_svg.contains("Print Slug Header"));
    assert!(plate1_svg.contains("Density Calibration Step Wedge"));
    assert!(plate1_svg.contains(r##"fill="#000000""##));
}

#[test]
fn test_cmyk_halftone_separations_and_dot_shapes() {
    use dither_core::renderers::{CmykHalftone, HalftoneDotShape};

    let mut buf = ImageBuffer::new(16, 16).expect("buf");
    for y in 0..16 {
        for x in 0..16 {
            let r = (x * 16) as u8;
            let g = (y * 16) as u8;
            let b = 128u8;
            buf.set_pixel(x, y, RgbColor::new(r, g, b));
        }
    }

    let cmyk = CmykHalftone { frequency: 0.20, dot_shape: HalftoneDotShape::Elliptical, dot_gain: 0.15, ..Default::default() };

    let pal = Palette::cmyk_process();
    let dithered = cmyk.apply(&buf, &pal).expect("cmyk apply");
    assert_eq!(dithered.width, 16);
    assert_eq!(dithered.height, 16);

    let seps = cmyk.generate_separations(&buf).expect("seps");
    assert_eq!(seps.cyan.width, 16);
    assert_eq!(seps.magenta.width, 16);
    assert_eq!(seps.yellow.width, 16);
    assert_eq!(seps.black.width, 16);

    let svg_plates = seps.export_all_svg_plates(&cmyk, 25.0, 1.0).expect("svg plates");
    assert_eq!(svg_plates.len(), 4);
    assert_eq!(svg_plates[0].0, "plate_cmyk_01_cyan.svg");
    assert_eq!(svg_plates[1].0, "plate_cmyk_02_magenta.svg");
    assert_eq!(svg_plates[2].0, "plate_cmyk_03_yellow.svg");
    assert_eq!(svg_plates[3].0, "plate_cmyk_04_black.svg");

    let cyan_svg = &svg_plates[0].1;
    assert!(cyan_svg.contains("PLATE 1/4: CYAN"));
    assert!(cyan_svg.contains("15.0° ROSETTE SCREEN"));
    assert!(cyan_svg.contains("Registration Targets"));
    assert!(cyan_svg.contains("Density Calibration Step Wedge"));
}

#[test]
fn test_spatial_tonal_band_dithering() {
    use dither_core::algorithms::{AlphaMaskDither, Atkinson, Bayer8x8, FloydSteinberg, TonalBandDither};

    let mut buf = ImageBuffer::new(32, 32).expect("buf");
    for y in 0..32 {
        for x in 0..32 {
            let gray = (x * 8) as u8;
            buf.set_pixel(x, y, RgbColor::new(gray, gray, gray));
        }
    }

    let pal = Palette::monochrome();

    let tonal = TonalBandDither::new(85, 170, 16, Box::new(Bayer8x8), Box::new(FloydSteinberg::default()), Box::new(Atkinson::default()));

    let out = tonal.apply(&buf, &pal).expect("tonal dither");
    assert_eq!(out.width, 32);
    assert_eq!(out.height, 32);

    let (ws, _wm, wh) = tonal.compute_weights(10);
    assert!(ws > 0.9);
    assert!(wh < 0.1);

    let (ws_hi, _wm_hi, wh_hi) = tonal.compute_weights(240);
    assert!(wh_hi > 0.9);
    assert!(ws_hi < 0.1);

    // Test Alpha Mask Dither
    let mut mask = ImageBuffer::new(32, 32).expect("mask");
    for y in 0..32 {
        for x in 0..32 {
            let val = if x < 16 { 255 } else { 0 };
            mask.set_pixel(x, y, RgbColor::new(val, val, val));
        }
    }

    let alpha_dither = AlphaMaskDither::new(Box::new(FloydSteinberg::default()), Box::new(Bayer8x8), mask);
    let alpha_out = alpha_dither.apply(&buf, &pal).expect("alpha dither");
    assert_eq!(alpha_out.width, 32);
    assert_eq!(alpha_out.height, 32);
}

#[test]
fn test_gcode_arc_fitting_and_hatch_styles() {
    use dither_core::vector::{GCodeConfig, GCodeExporter, HatchStyle};

    let mut buf = ImageBuffer::new(16, 16).expect("buf");
    for y in 2..14 {
        for x in 2..14 {
            buf.set_pixel(x, y, RgbColor::new(0, 0, 0));
        }
    }

    // 1. Vertical hatch style
    let config_vert = GCodeConfig { hatch_style: HatchStyle::Vertical, ..Default::default() };
    let gcode_vert = GCodeExporter::generate_gcode(&buf, &config_vert).expect("vert gcode");
    assert!(gcode_vert.contains("Hatch style: Vertical"));

    // 2. 45-degree crosshatch style
    let config_diag = GCodeConfig { hatch_style: HatchStyle::CrossHatch45, hatch_spacing_mm: 1.0, ..Default::default() };
    let gcode_diag = GCodeExporter::generate_gcode(&buf, &config_diag).expect("diag gcode");
    assert!(gcode_diag.contains("Hatch style: CrossHatch45"));

    // 3. Stipple Poisson style
    let config_stipple = GCodeConfig { hatch_style: HatchStyle::StipplePoisson, hatch_spacing_mm: 1.5, ..Default::default() };
    let gcode_stipple = GCodeExporter::generate_gcode(&buf, &config_stipple).expect("stipple gcode");
    assert!(gcode_stipple.contains("Hatch style: StipplePoisson"));

    // 4. Arc fitting on circle points
    let arc_config = GCodeConfig { arc_fitting: true, arc_tolerance: 0.5, tool_change_pause: true, tool_number: Some(1), ..Default::default() };

    let gcode_arc = GCodeExporter::generate_gcode(&buf, &arc_config).expect("arc gcode");
    assert!(gcode_arc.contains("T01 M06 ; Tool change"));
    assert!(gcode_arc.contains("M00 ; Pause for pen inspection"));
    assert!(gcode_arc.contains("Arc fitting: true"));
}

#[test]
fn test_custom_matrix_workshop() {
    use dither_core::algorithms::CustomMatrixDither;

    let matrix = CustomMatrixDither::new("Checker 2x2", 2, 2, vec![64, 192, 192, 64]).expect("custom matrix");

    let mut buf = ImageBuffer::new(4, 4).expect("buf");
    for y in 0..4 {
        for x in 0..4 {
            buf.set_pixel(x, y, RgbColor::new(128, 128, 128));
        }
    }
    let pal = Palette::monochrome();
    let out = matrix.apply(&buf, &pal).expect("custom matrix apply");
    assert_eq!(out.width, 4);
    assert_eq!(out.height, 4);

    let json = matrix.to_json().expect("matrix json");
    let loaded = CustomMatrixDither::from_json(&json).expect("from json");
    assert_eq!(loaded.name, "Checker 2x2");
    assert_eq!(loaded.width, 2);
    assert_eq!(loaded.height, 2);
}
