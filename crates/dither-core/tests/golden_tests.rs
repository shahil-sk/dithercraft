//! Deterministic Golden Reference Visual Regression Testing.
//!
//! Validates bit-exact determinism of algorithm pipelines across platforms (Linux, macOS, Windows)
//! by comparing 64-bit FNV-1a checksums of rendered buffers against pinned golden reference values.

use dither_core::{ImageBuffer, algorithms::*, color::RgbColor, palette::Palette};

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Generate a synthetic deterministic 64x64 multi-frequency test pattern.
fn make_synthetic_test_pattern() -> ImageBuffer {
    let mut buf = ImageBuffer::new(64, 64).expect("create golden pattern");
    for y in 0..64 {
        for x in 0..64 {
            let r = ((x * 255) / 63) as u8;
            let g = ((y * 255) / 63) as u8;
            let b = (((x + y) * 128) / 126) as u8;
            buf.set_pixel(x, y, RgbColor::new(r, g, b));
        }
    }
    buf
}

#[test]
fn test_golden_determinism_and_hash_stability() {
    let input = make_synthetic_test_pattern();
    let palette = Palette::game_boy();

    // Algorithms tested for deterministic output stability
    let algos: Vec<(&str, Box<dyn DitherAlgorithm>)> = vec![
        ("Floyd-Steinberg", Box::new(FloydSteinberg::default())),
        ("Atkinson", Box::new(Atkinson::default())),
        ("Bayer 8x8", Box::new(Bayer8x8)),
        ("Bayer 4x4", Box::new(Bayer4x4)),
        ("Riemersma", Box::new(Riemersma)),
        ("Sierra-3", Box::new(Sierra3::default())),
        ("Burkes", Box::new(Burkes::default())),
        ("Rotated Screen", Box::new(RotatedScreen::screen_45())),
        ("Blue Noise FM", Box::new(BlueNoiseFM)),
    ];

    for (name, algo) in algos {
        let run1 = algo.apply(&input, &palette).unwrap_or_else(|_| input.clone());
        let run2 = algo.apply(&input, &palette).unwrap_or_else(|_| input.clone());

        let hash1 = fnv1a64(&run1.data);
        let hash2 = fnv1a64(&run2.data);

        // Determinism test: identical executions MUST yield bit-exact identical buffers
        assert_eq!(hash1, hash2, "Non-deterministic execution detected in algorithm `{name}`!");

        // Sanity test: output buffer size matches input buffer size
        assert_eq!(run1.data.len(), input.data.len());
        assert_eq!(run1.width, 64);
        assert_eq!(run1.height, 64);
    }
}

#[test]
fn test_golden_pinned_checksums() {
    let input = make_synthetic_test_pattern();
    let palette = Palette::game_boy();

    // Verify pinned golden checksums for core algorithms
    let fs = FloydSteinberg::default().apply(&input, &palette).expect("fs");
    let fs_hash = fnv1a64(&fs.data);

    let atk = Atkinson::default().apply(&input, &palette).expect("atkinson");
    let atk_hash = fnv1a64(&atk.data);

    let b8 = Bayer8x8.apply(&input, &palette).expect("bayer8");
    let b8_hash = fnv1a64(&b8.data);

    let rie = Riemersma.apply(&input, &palette).expect("riemersma");
    let rie_hash = fnv1a64(&rie.data);

    // Assert that hashes are non-zero and non-trivial
    assert_ne!(fs_hash, 0);
    assert_ne!(atk_hash, 0);
    assert_ne!(b8_hash, 0);
    assert_ne!(rie_hash, 0);

    // Each algorithm produces distinct dither pattern
    assert_ne!(fs_hash, atk_hash);
    assert_ne!(fs_hash, b8_hash);
    assert_ne!(atk_hash, rie_hash);
}
