use dither_core::{
    ImageBuffer,
    algorithms::{ALL_ALGORITHMS, create_algorithm},
    color::RgbColor,
    palette::Palette,
    vector::{Point2D, rdp_simplify},
};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_buffer_extremes_never_panic(
        w in 1usize..=32,
        h in 1usize..=32,
        r_fill in 0u8..=255,
        g_fill in 0u8..=255,
        b_fill in 0u8..=255,
        algo_idx in 0usize..ALL_ALGORITHMS.len(),
    ) {
        let mut buf = ImageBuffer::new(w as u32, h as u32).expect("valid buffer");
        for y in 0..(h as u32) {
            for x in 0..(w as u32) {
                buf.set_pixel(x, y, RgbColor::new(r_fill, g_fill, b_fill));
            }
        }

        let pal = Palette::game_boy();
        let info = &ALL_ALGORITHMS[algo_idx];
        let algo = create_algorithm(info.name).expect("instantiate algorithm");

        let res = algo.apply(&buf, &pal);
        prop_assert!(res.is_ok(), "Algorithm {} panicked or errored: {:?}", info.name, res.err());

        let out = res.expect("output");
        prop_assert_eq!(out.width, w as u32);
        prop_assert_eq!(out.height, h as u32);
        prop_assert_eq!(out.data.len(), w * h * 3);

        // Every pixel must belong to palette
        for y in 0..(h as u32) {
            for x in 0..(w as u32) {
                let px = out.get_pixel(x, y).expect("pixel access");
                prop_assert!(pal.swatches.iter().any(|s| s.color == px));
            }
        }
    }

    #[test]
    fn prop_color_distance_symmetry_and_identity(
        r1 in 0u8..=255,
        g1 in 0u8..=255,
        b1 in 0u8..=255,
        r2 in 0u8..=255,
        g2 in 0u8..=255,
        b2 in 0u8..=255,
    ) {
        let c1 = RgbColor::new(r1, g1, b1);
        let c2 = RgbColor::new(r2, g2, b2);

        // Identity
        prop_assert_eq!(c1.distance_sq(&c1), 0);
        prop_assert_eq!(c1.distance_redmean(&c1), 0);
        prop_assert!(c1.distance_oklab(&c1) < 1e-5);
        prop_assert!(c1.distance_ciede2000(&c1) < 1e-4);
        prop_assert!(c1.distance_cielab_de(&c1) < 1e-4);

        // Symmetry
        let d_sq_12 = c1.distance_sq(&c2);
        let d_sq_21 = c2.distance_sq(&c1);
        prop_assert_eq!(d_sq_12, d_sq_21);

        let d_red_12 = c1.distance_redmean(&c2);
        let d_red_21 = c2.distance_redmean(&c1);
        prop_assert_eq!(d_red_12, d_red_21);

        let d_ok_12 = c1.distance_oklab(&c2);
        let d_ok_21 = c2.distance_oklab(&c1);
        prop_assert!((d_ok_12 - d_ok_21).abs() < 1e-4);

        let d_cie_12 = c1.distance_ciede2000(&c2);
        let d_cie_21 = c2.distance_ciede2000(&c1);
        prop_assert!((d_cie_12 - d_cie_21).abs() < 1e-4);

        // Non-negativity
        prop_assert!(d_ok_12 >= 0.0);
        prop_assert!(d_cie_12 >= 0.0);
    }

    #[test]
    fn prop_gamma_srgb_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
        let orig = RgbColor::new(r, g, b);
        let (lin_r, lin_g, lin_b) = orig.to_linear_f32();
        let roundtrip = RgbColor::from_linear_f32(lin_r, lin_g, lin_b);
        prop_assert_eq!(roundtrip, orig, "Mismatch in sRGB gamma roundtrip for {:?}", orig);
    }

    #[test]
    fn prop_rdp_simplification_invariants(
        n in 2usize..=40,
        epsilon in 0.0f32..=100.0f32,
    ) {
        let pts: Vec<Point2D> = (0..n).map(|i| Point2D::new(i as f32, ((i * 7) % 11) as f32)).collect();
        let simplified = rdp_simplify(&pts, epsilon);

        prop_assert!(simplified.len() >= 2);
        prop_assert!(simplified.len() <= pts.len());
        prop_assert_eq!(simplified.first(), pts.first());
        prop_assert_eq!(simplified.last(), pts.last());
    }
}
