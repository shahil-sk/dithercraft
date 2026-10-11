# DitherCraft (Rust) — Clean-Room System Specification (SPEC.md)

**Document Version:** 2.1.0  
**Project:** DitherCraft — Pure Rust Clean-Room Implementation  
**Target Reference:** Clean-Room Implementation of Dither Boy Architecture  
**Language & Edition:** Pure Rust (2024 Edition)  
**Classification:** Clean-Room Architecture & Functional Specification  

---

## 1. Executive Summary & Clean-Room Directive

**DitherCraft** is an open-source, studio-grade desktop workstation and DSP engine for retro graphical dithering, vintage CRT simulations, and creative glitch effects, built in **Pure Rust**.

Inspired by [VectorCraft](https://github.com/storytold/vectorcraft) and [Dither Boy](https://studioaaa.com/product/dither-boy/), the project adheres to strict clean-room engineering, no-panic reliability, command-driven architecture, and automated multi-platform delivery.

### Clean-Room Principle
All mathematical formulations, threshold matrices, and quantization procedures are implemented strictly from public scientific literature and open digital signal processing (DSP) papers (Bayer 1973, Floyd-Steinberg 1976, Atkinson 1984, Ulichney 1987, Riemersma 1998, Ostromoukhov 2001). No code or proprietary assets from Dither Boy or third-party proprietary software are referenced or copied.

---

## 2. Cargo Workspace Architecture

The system is architected as a modular Cargo workspace:

```text
dithercraft/
├── Cargo.toml                  # Workspace root
├── apps/
│   ├── dithercraft/            # Hardware-accelerated desktop application (egui/eframe/wgpu)
│   │   └── src/main.rs         # 60+ FPS preview, 5 studio themes, non-blocking workers
│   └── dithercraft-cli/        # Headless batch & streaming CLI + benchmarks
│       └── src/main.rs         # Subcommands: single, svg, bench
├── crates/
│   └── dither-core/            # Pure DSP & dithering math (no GUI, no windowing)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── algorithms/     # 63 algorithms across 6 categories
│       │   ├── renderers/      # CMYK Halftone, Pop Tone, Floyd-Steinberg
│       │   ├── color_depth/    # Luminosity mapping & Mismatch mode
│       │   ├── palette.rs      # Extraction, Swatch locking/random, Tonal cycling
│       │   ├── effects/        # Stackable post-effects & JPEG Glitch Core
│       │   ├── vector/         # Black vector isolation & SVG path tracing
│       │   └── temporal.rs     # 9 CRT/LCD temporal noise patterns
│       └── tests/              # Core verification suite
├── packaging/                  # Multi-platform installers (MSI, DMG, Deb, RPM, Flatpak)
├── xtask/                      # Cargo automation: layering, stats, bundle, brand checks
├── assets/                     # App icons, vector assets, fonts
└── presets/                    # Bundled retro JSON palettes & settings
```

---

## 3. Processing Core (`dither-core`)

### 3.1 Algorithm Library (63 Total Algorithms)
Organized into 6 major mathematical categories:
1. **Error-Diffusion**: Floyd-Steinberg, Atkinson, Sierra-3, Sierra-2, Sierra-Lite, Jarvis-Judice-Ninke (JJN), Stucki, Burkes, Stevenson-Arce, Ostromoukhov, Variable-Error, Dot-Diffusion, Riemersma, Minimized Average Error (Ostromoukhov-style), Fan, Shiau-Fan (1 & 2), and bidirectional Serpentine passes.
2. **Ordered Dithering**: Bayer $2\times 2$, $4\times 4$, $8\times 8$, $16\times 16$, Clustered Dot ($4\times 4$, $8\times 8$), Central White Point, Dispersed Dot, Spiral, Horizontal Line, Vertical Line, Diagonal Line, Checkerboard 45°, Interleaved Gradient Noise.
3. **Bitmap Conversion**: Fixed Threshold, Adaptive Otsu Threshold, Mean Threshold, Gaussian Local Threshold, Sauvola Binarization, Niblack Binarization, Multi-level Step Quantization.
4. **Halftone Arrays**: Rotated Screen Halftones ($0^\circ, 15^\circ, 45^\circ, 75^\circ$), Circular Screen, Diamond Dot, Elliptical Dot, Line Screen, Crosshatch Halftone.
5. **Modulation Systems**: Frequency Modulation (FM / Blue Noise), Amplitude Modulation (AM / Screen Arrays), Stochastic Screening, Void-and-Cluster Masking, Uniform Random Noise Dithering, Triangular Noise Dithering.
6. **Custom Artistic Tones**: Mezzotint, Stippling, Woodcut Simulation, Newsprint Grain, Etching Crosshatch, Matrix Glitch Dispersion.

### 3.2 Specialized Renderers
- **`cmyk_halftone`**: Simulates four-color process printing separations (Cyan, Magenta, Yellow, blacK) with independent, adjustable dot screen angles ($15^\circ, 75^\circ, 0^\circ, 45^\circ$) and configurable screen frequencies (LPI).
- **`pop_tone`**: High-contrast, stylized graphic rendering that remaps midtones aggressively to punchy discrete dual/tri-tone boundaries.
- **`floyd_steinberg`**: Classic academic error-diffusion with subpixel residual accumulation and optional serpentine row scanning.

### 3.3 Color Depth Engine
- **Luminosity Mapping**: Directly projects input pixel luminosity onto ordered color values of the selected palette, bypassing standard Euclidean distance matching when stylistic gradients are desired.
- **Mismatch Mode**: Intentionally breaks alignment between color depth quantization grids and luminosity scaling factors, injecting retro bit-depth quantization artifacts, color banding, and vintage hardware glitches.

---

## 4. Color and Palette Management

### 4.1 Native Color Extraction Engine
- Automatically samples and clusters native colors from source imagery using Median Cut and K-Means clustering to generate custom color profiles on image import.

### 4.2 Swatch Controls
- **Slot Locking**: Individual color swatches in active palettes can be toggled to a "Locked" state, protecting them from global modifications.
- **Selective Randomization**: Shuffles and mutates only unlocked color slots, enabling rapid exploration of color harmonies while preserving base anchor tones.

### 4.3 Tonal Sorting & Cycling
- **Ordered Cycling**: Automatically re-indexes active palette entries in strictly ascending or descending luminance ($Y = 0.299R + 0.587G + 0.114B$), enabling real-time palette rotation and cyclic animated color shifting across isolated tonal registers.

---

## 5. Animation and Video Pipeline

### 5.1 Native Multi-Frame Engine
- High-performance, streaming pipeline supporting both single still images and multi-frame video files (`MP4`, animated `GIF`, `WebM`).

### 5.2 Interface Components
- **Timeline Editor**: In-app multi-frame sequence interface with frame trimming, timeline scrub bar, play/pause controls, loop mode, and frame-rate targeting (1 to 60 FPS).
- **Live Playback**: Real-time viewport playback rendering video frames on-the-fly while processing parameters and dither modes are adjusted.

### 5.3 Temporal Variation
- **9 CRT & LCD Noise Patterns**: Animated procedural noise overlays simulating physical cathode-ray tube phosphor decay, shadow mask hum, and vintage liquid-crystal subpixel response:
  1. *CRT Scan Beam Jitter*
  2. *Phosphor Grain Hum*
  3. *Interlaced Field Shimmer*
  4. *LCD Subpixel Grid Drift*
  5. *VHS Tape Phase Noise*
  6. *Thermal Analog Drift*
  7. *Static Hash White Noise*
  8. *Bayer Phase Stepping*
  9. *Blue Noise Temporal Jitter*

### 5.4 Keyframes & Parameter Modulation
- Timeline keyframing system that automates property interpolation (brightness, contrast, dither scale, threshold bias, chromatic shift) across frame ranges.

---

## 6. Stackable Downstream Effects & Glitch Processors

Downstream, layer-based, reorderable effect stack executing after core dithering/quantization:

```text
[Input Image] ──► [Core Dither & Palette] ──► [Stackable Effect Layer 1] ──► [Layer 2] ──► [Output]
```

### Effects Library
1. **Epsilon Glow**: Diffusion bloom algorithm that extracts highlights, applies separable Gaussian convolution, and bleeds luminous halos across high-contrast dithered edges.
2. **Chromatic Aberration**: Lens distortion filter that radially and directionally offsets the Red, Green, and Blue channels with subpixel spatial interpolation.
3. **JPEG Glitch Core**: Authentic discrete cosine transform (DCT) and byte-stream compression corruption with 5 dedicated sub-modes:
   - *Block Shift*: Horizontal dislocation of $8\times 8$ and $16\times 16$ macroblocks.
   - *Channel Swap*: Transposes YCbCr / RGB chrominance and luminance planes.
   - *Scan Line Offset*: Staggered horizontal phase tearing across raster rows.
   - *Block Scramble*: Pseudo-random spatial shuffling of image compression tiles.
   - *Interlace Corruption*: Alternating field dropped frames and comb-filter decay.
4. **Spatial Transforms**: Standard high-pass Sharpen, Gaussian Blur, and directional Chromatic Channel Distortions.

---

## 7. User Interface & Performance Optimizations

### 7.1 Navigation & Controls
- **Searchable Dropdowns**: Instant fuzzy text filtering for all 63 algorithms and styles without traversing nested category submenus.
- **Adjustment Shuffling**: Dedicated randomizer that rolls new random values for sliders (brightness, contrast, scale, glitch intensity) while respecting locked attributes.
- **Scroll Wheel Hardware Toggles**: User preference to enable/disable fine-grained mouse wheel increments when hovering over sliders.

### 7.2 Rendering Modes & Viewport
- **Debounce Mode**: Optional power-saving mode that pauses live-view rendering while sliders are actively dragged, updating on mouse release.
- **Optimized Viewport**: Hardware-accelerated (WGPU) canvas with nearest-neighbor crisp pixel scaling, smooth panning, and $<16\text{ms}$ redraws during zoom/pan navigation.

### 7.3 Multi-Theme Support (5 Global Themes)
Five fully customized interface themes modifying typography, widget geometry, slider graphics, and contextual accent colors:
1. *Studio Dark* (Professional dark neutral)
2. *Amber CRT* (Vintage 1980s monochrome terminal)
3. *Cyber Matrix* (High-contrast neon green on pitch black)
4. *Vapor Slate* (Cool retro synthwave tones)
5. *Industrial High-Contrast* (Clean studio monochrome)

---

## 8. Export and Production Pipeline

### 8.1 Export Formats
- **Raster Formats**: PNG (lossless, 8-bit & 24-bit indexed), JPEG (quality adjustable).
- **Video Formats**: MP4 (H.264 / AAC) with audio preservation.
- **Vector Formats**: SVG.

### 8.2 Production Features
- **Black Vector Isolation**: High-precision vector contour extraction algorithm that traces only the darkest pixel data into optimized vector paths (`SVG`), tailored specifically for vinyl cutters, CNC plotters, laser engravers, and embroidery machines.
- **Production Automation**:
  - Customizable export filename patterns (`{name}_{algo}_{palette}_{timestamp}`).
  - Default output destination folder targeting.
  - Historical recent-file and recent-export tracking.

---

## 9. Performance & Quality Invariants

| Invariant | Standard |
| :--- | :--- |
| **GUI Responsiveness** | Main thread $\ge 60$ FPS. Dithering runs on background Rayon workers. |
| **Allocations** | Zero dynamic heap allocations in per-pixel processing hot loops. |
| **Reliability** | No `unwrap()` or `panic!` in production paths. Errors handled as `Result<T, DitherError>`. |
| **Decoupling** | `dither-core` has zero GUI/windowing dependencies. |
