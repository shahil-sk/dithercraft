# DitherCraft (Rust) — Engineering Roadmap (ROADMAP.md)

This roadmap details the engineering phases for the clean-room implementation of DitherCraft in Pure Rust, adhering to strict clean-room engineering, no-panic reliability, command-driven architecture, and non-blocking performance.

---

## Phase 1: Cargo Workspace & Processing Core (`crates/dither-core`)
- [x] Configure Cargo workspace (`crates/dither-core`, `apps/dithercraft-cli`, `apps/dithercraft`, `xtask`).
- [x] Core image abstractions: zero-copy `ImageBuffer`, `RgbColor`, `Palette`, and `DitherAlgorithm` trait.
- [x] **Implement 63 Algorithms across 6 Mathematical Categories**:
  - [x] **Error-Diffusion (17 algorithms)**:
    - Floyd-Steinberg, Atkinson, Sierra-3, Sierra-2, Sierra-Lite, Jarvis-Judice-Ninke (JJN), Stucki, Burkes, Stevenson-Arce, Ostromoukhov, Variable-Error, Dot-Diffusion, Riemersma (Hilbert space-filling curve), Minimized Average Error, Fan, Shiau-Fan (1 & 2), and serpentine row traversal.
  - [x] **Ordered Dithering (13 algorithms)**:
    - Dynamic Bayer matrix generator ($2\times 2$, $4\times 4$, $8\times 8$, $16\times 16$), Clustered Dot ($4\times 4$, $8\times 8$), Central White Point, Dispersed Dot, Spiral, Horizontal Line, Vertical Line, Diagonal Line, Checkerboard 45°, Interleaved Gradient Noise.
  - [x] **Bitmap Conversion (7 algorithms)**:
    - Fixed Threshold, Adaptive Otsu Threshold, Mean Threshold, Gaussian Local Threshold, Sauvola Binarization, Niblack Binarization, Multi-level Step Quantization.
  - [x] **Halftone Arrays (7 algorithms)**:
    - Rotated Screen Halftones ($0^\circ, 15^\circ, 45^\circ, 75^\circ$), Circular Screen, Diamond Dot, Elliptical Dot, Line Screen, Crosshatch Halftone.
  - [x] **Modulation Systems (6 algorithms)**:
    - Frequency Modulation (FM / Blue Noise), Amplitude Modulation (AM / Screen Arrays), Stochastic Screening, Void-and-Cluster Masking, Uniform Random Noise Dithering, Triangular Noise Dithering.
  - [x] **Custom Artistic Tones (6 algorithms)**:
    - Mezzotint, Stippling, Woodcut Simulation, Newsprint Grain, Etching Crosshatch, Matrix Glitch Dispersion.
- [x] **Specialized Renderers**:
  - [x] `cmyk_halftone`: Four-color process separations (CMYK) with independent dot screen angles ($15^\circ, 75^\circ, 0^\circ, 45^\circ$) and adjustable screen frequencies (LPI).
  - [x] `pop_tone`: High-contrast stylized graphic rendering remapping midtones to sharp dual/tri-tone boundaries.
  - [x] `floyd_steinberg`: Classic academic error-diffusion with subpixel residual accumulation.
- [x] **Color Depth Engine**:
  - [x] Luminosity Mapping: Direct projection of pixel brightness to palette colors.
  - [x] Mismatch Mode: Intentional misalignment between color depth grids and luminosity scales for retro hardware glitch artifacts.
- [x] Multi-threaded row/tile processing with Rayon.
- [x] Criterion micro-benchmarks comparing scalar vs SIMD/Rayon performance.

---

## Phase 2: Color Science, Palette Management & Vector Isolation
- [x] Color distance metrics: Euclidean RGB, Perceptual Redmean, CIELAB $\Delta E^*$.
- [x] Fast spatial k-d tree acceleration for nearest-palette-color lookup.
- [x] **Color & Palette Management**:
  - [x] Native Color Extraction Engine: Median Cut and K-Means clustering from source images.
  - [x] Swatch Controls: Slot locking to preserve anchor tones and selective randomization for unlocked slots.
  - [x] Tonal Sorting & Cycling: Luminance-sorted ($Y = 0.299R + 0.587G + 0.114B$) ordered cycling for dynamic animated sweeps.
- [x] Multi-format palette parser: `.json` (presets), `.hex`, `.gpl` (GIMP palette), `.pal`.
- [x] Bundled retro palettes: Game Boy, CGA Mode 1, Apple II, Commodore 64, NES, Cyberpunk, Mac 1-bit, PICO-8.
- [x] **Black Vector Isolation Engine (`crates/dither-core/src/vector`)**:
  - [x] High-precision contour tracing extracting darkest pixel data into optimized SVG vector paths.
  - [x] Path optimization for vinyl cutters, CNC plotters, laser engravers, and embroidery machines.

---

## Phase 3: Stackable Effects & Glitch Processors (`crates/dither-core/src/effects`)
- [x] Downstream, layer-based, reorderable effect stack executing post-dither.
- [x] **Effects Library**:
  - [x] **Epsilon Glow**: Highlight extraction, separable Gaussian bloom convolution, edge bleeding halos.
  - [x] **Chromatic Aberration**: Radial and directional RGB subpixel channel displacement.
  - [x] **JPEG Glitch Core**: Authentic discrete cosine transform (DCT) & byte-stream corruption with 5 sub-modes:
    - *Block Shift*: Horizontal dislocation of $8\times 8$ and $16\times 16$ macroblocks.
    - *Channel Swap*: Transposing YCbCr / RGB chrominance and luminance planes.
    - *Scan Line Offset*: Staggered horizontal phase tearing across raster rows.
    - *Block Scramble*: Pseudo-random spatial shuffling of image compression tiles.
    - *Interlace Corruption*: Alternating field dropped frames and comb-filter decay.
  - [x] **Spatial Transforms**: Standard Gaussian Blur, high-pass Sharpen, directional chromatic channel distortions.
  - [x] Vintage CRT Simulation: Scanlines, phosphor glow, barrel lens curvature, vignette.

---

## Phase 4: Animation & Video Pipeline (`crates/dither-core/src/temporal`)
- [ ] Native multi-frame processing engine supporting still images and video files (`MP4`, animated `GIF`, `WebM`).
- [ ] **In-App Timeline Editor**:
  - [ ] Multi-frame interface with frame trimming, scrub bar, transport controls, loop mode, 1–60 FPS targeting.
  - [ ] Real-time live playback viewport with on-the-fly parameter tweaking.
- [x] **9 CRT & LCD Temporal Noise Patterns**:
  1. CRT Scan Beam Jitter
  2. Phosphor Grain Hum
  3. Interlaced Field Shimmer
  4. LCD Subpixel Grid Drift
  5. VHS Tape Phase Noise
  6. Thermal Analog Drift
  7. Static Hash White Noise
  8. Bayer Phase Stepping
  9. Blue Noise Temporal Jitter
- [ ] Timeline keyframing & automated parameter modulation over time (brightness, contrast, scale, glitch intensity).

---

## Phase 5: Hardware-Accelerated Native GUI (`apps/dithercraft`)
- [x] Native cross-platform desktop UI using `egui` and `eframe` (WGPU hardware-accelerated backend).
- [ ] **Interactive Viewport**:
  - [x] Hardware-accelerated canvas with nearest-neighbor crisp pixel scaling, smooth panning, $<16\text{ms}$ redraws.
  - [ ] Split-view comparison slider (Before vs. After).
- [x] **Navigation & Controls**:
  - [x] Searchable Dropdowns: Instant fuzzy text filtering across all 63 algorithms and styles.
  - [x] Adjustment Shuffling: Dedicated randomizer rolling new slider values while preserving locked attributes.
  - [ ] Scroll Wheel Hardware Toggles: User preference to toggle fine-grained wheel increments on sliders.
  - [x] Debounce Mode: Power-saving mode pausing live-view updates during slider drag, updating on mouse release.
- [x] **Multi-Theme Support (5 Global Themes)**:
  1. *Studio Dark* (Professional dark neutral)
  2. *Amber CRT* (Vintage 1980s monochrome terminal)
  3. *Cyber Matrix* (High-contrast neon green on pitch black)
  4. *Vapor Slate* (Cool retro synthwave tones)
  5. *Industrial High-Contrast* (Clean studio monochrome)
- [x] **Non-Blocking Threading**:
  - [x] Background Rayon worker thread pool with instant cancellation on slider input to eliminate queuing lag.

---

## Phase 6: Headless CLI, Batch Processing & Production Automation (`apps/dithercraft-cli`)
- [x] High-throughput CLI interface powered by `clap` with subcommands:
  - `dither single <input> <output> --algo <algo> --palette <palette>`
  - `dither batch <input_dir> <output_dir> --threads <n>`
  - `dither video <input_video> <output_video> --algo <algo> --fps <fps>`
  - `dither bench` (internal latency and throughput report)
- [ ] Model Context Protocol (MCP) server & JSON-RPC interface for headless automation and testing.
- [ ] **Production Automation**:
  - [ ] Customizable export filename patterns (`{name}_{algo}_{palette}_{timestamp}`).
  - [ ] Default destination folder targeting.
  - [ ] Historical recent-file and recent-export tracking.
- [ ] Multi-format exporter: PNG (8-bit indexed / 24-bit), JPEG, MP4, SVG (black vector isolated).

---

## Phase 7: Deployment & Cross-Platform Distribution
- [ ] Production release profiles with fat Link-Time Optimization (LTO), binary stripping, and single codegen unit.
- [ ] Multi-platform automated CI/CD matrix via GitHub Actions:
  - Windows: Standalone portable `.exe` ($< 20\text{ MB}$).
  - Linux: Standalone binary and AppImage ($< 20\text{ MB}$).
  - macOS: Universal binary (Apple Silicon + Intel x86_64) packaged as `.dmg` ($< 25\text{ MB}$).
- [ ] Zero runtime external dependencies: native self-contained binaries.
