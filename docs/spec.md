# DitherCraft (Rust) — Clean-Room System Specification (SPEC.md)

**Document Version:** 3.1.0  
**Project:** DitherCraft — Pure Rust Clean-Room Implementation  
**Target Reference:** Clean-Room Implementation of Dither Boy Architecture  
**Language & Edition:** Pure Rust (2024 Edition)  
**Last Updated:** October 12, 2026  
**Classification:** Clean-Room Architecture & Functional Specification  

---

## 1. Executive Summary & Clean-Room Directive

**DitherCraft** is an open-source, studio-grade desktop workstation and DSP engine for retro graphical dithering, vintage CRT simulations, creative glitch effects, and vector toolpath preparation, built in **Pure Rust**.

Inspired by [VectorCraft](https://github.com/storytold/vectorcraft) and [Dither Boy](https://studioaaa.com/product/dither-boy/), the project adheres to strict clean-room engineering, no-panic reliability, command-driven architecture, and automated multi-platform delivery.

### Clean-Room Principle
All mathematical formulations, threshold matrices, and quantization procedures are implemented strictly from public scientific literature and open digital signal processing (DSP) papers (Bayer 1973, Floyd-Steinberg 1976, Atkinson 1984, Ulichney 1987, Riemersma 1998, Ostromoukhov 2001, Ottosson 2020). No code or proprietary assets from Dither Boy or third-party proprietary software are referenced or copied.

---

## 2. Cargo Workspace Architecture

The system is architected as a modular Cargo workspace:

```text
dithercraft/
├── Cargo.toml                  # Workspace root
├── apps/
│   ├── dithercraft/            # Hardware-accelerated desktop workstation (egui/eframe/wgpu)
│   │   └── src/main.rs         # 60+ FPS preview, 5 studio themes, Telemetry HUD, non-blocking workers
│   └── dithercraft-cli/        # Headless batch & streaming CLI + benchmarks + MCP server
│       └── src/main.rs         # Subcommands: single, gif, svg, bench, mcp, mp4-info, webm-info, trim-audio
├── crates/
│   └── dither-core/            # Pure DSP & dithering math (no GUI, no windowing)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── algorithms/     # 63 algorithms across 6 categories
│       │   ├── audio.rs        # Pure-Rust RIFF WAV decoder, spectrum analyzer, WAV slice exporter
│       │   ├── buffer.rs       # ImageBuffer creation, progressive downsampling, pixel access
│       │   ├── color.rs        # Linear sRGB, OKLab, CIELAB, CIEDE2000, Euclidean RGB
│       │   ├── color_depth/    # Luminosity mapping & Mismatch mode
│       │   ├── effects/        # Stackable post-effects & authentic JPEG Glitch Core
│       │   ├── palette.rs      # Extraction (Median Cut, K-Means), 3D KD-Tree, ACT/GPL/HEX/PAL/JSON
│       │   ├── renderers/      # CMYK Halftone, Pop Tone, Floyd-Steinberg
│       │   ├── shaders/        # Pure WGSL compute kernels (Bayer, Halftone, Unified CRT)
│       │   ├── temporal.rs     # 9 CRT/LCD temporal noise patterns, LFO tracks, keyframes
│       │   ├── vector/         # Black vector isolation, RDP, Visvalingam, Bézier splines, G-code TSP
│       │   └── video.rs        # ISO-BMFF MP4 demuxer, EBML WebM demuxer, Animated WebP muxer
│       └── tests/              # Core verification suite (golden hashes, proptests)
├── packaging/                  # Multi-platform installers (MSI, DMG, Deb, RPM, Flatpak)
├── xtask/                      # Cargo automation: layering, stats, bundle, brand checks, CI gates
├── assets/                     # App icons, vector assets, fonts
└── presets/                    # Bundled retro JSON palettes, studio profiles & settings
```

---

## 3. Processing Core (`dither-core`)

### 3.1 Algorithm Library (63 Total Algorithms)
Organized into 6 major mathematical categories:
1. **Error-Diffusion (17)**: Floyd-Steinberg, Atkinson, Sierra-3, Sierra-2, Sierra-Lite, Jarvis-Judice-Ninke (JJN), Stucki, Burkes, Stevenson-Arce, Ostromoukhov (2001 dynamic 256-level tables), Variable-Error, Dot-Diffusion, Riemersma (Hilbert curve), Minimized Average Error, Fan, Shiau-Fan (1 & 2), and bidirectional Serpentine passes.
2. **Ordered Dithering (13)**: Bayer $2\times 2$, $4\times 4$, $8\times 8$, $16\times 16$, Clustered Dot ($4\times 4$, $8\times 8$), Central White Point, Dispersed Dot, Spiral, Horizontal Line, Vertical Line, Diagonal Line, Checkerboard 45°, Interleaved Gradient Noise (IGN).
3. **Bitmap Conversion (7)**: Fixed Threshold, Adaptive Otsu Threshold, Mean Threshold, Gaussian Local Threshold, Sauvola Binarization, Niblack Binarization, Multi-level Step Quantization.
4. **Halftone Arrays (7)**: Rotated Screen Halftones ($0^\circ, 15^\circ, 45^\circ, 75^\circ$), Circular Screen, Diamond Dot, Elliptical Dot, Line Screen, Crosshatch Halftone.
5. **Modulation Systems (6)**: Frequency Modulation (FM / Blue Noise Quasirandom $R_2$), Amplitude Modulation (AM / Screen Arrays), Stochastic Screening, Void-and-Cluster Masking, Uniform Random Noise Dithering, Triangular Noise Dithering.
6. **Custom Artistic Tones (6)**: Mezzotint, Stippling, Woodcut Simulation, Newsprint Grain, Etching Crosshatch, Matrix Glitch Dispersion.

### 3.2 Specialized Renderers
- **`cmyk_halftone`**: Simulates four-color process printing separations (Cyan, Magenta, Yellow, blacK) with independent, adjustable dot screen angles ($15^\circ, 75^\circ, 0^\circ, 45^\circ$) and configurable screen frequencies (LPI).
- **`pop_tone`**: High-contrast, stylized graphic rendering that remaps midtones aggressively to punchy discrete dual/tri-tone boundaries.
- **`floyd_steinberg`**: Classic academic error-diffusion with subpixel residual accumulation and optional serpentine row scanning.

### 3.3 Color Science & Color Depth Engine
- **Gamma-Correct Linear sRGB Blending**: Perceptually accurate linear light transfer functions preventing dark haloing around dithered boundaries.
- **Modern Perceptual Spaces**:
  - **OKLab & Oklch**: True perceptual lightness ($L$), chroma ($C$), and hue ($h$) color distance eliminating blue-to-purple shifts.
  - **CIEDE2000 ($\Delta E_{00}$)**: Complete ISO/CIE standard color difference with rotation terms ($R_T$) and parametric factors ($k_L, k_C, k_H$).
- **Luminosity Mapping**: Directly projects input pixel luminosity onto ordered color values of the selected palette, bypassing standard Euclidean distance matching.
- **Mismatch Mode**: Intentionally breaks alignment between color depth quantization grids and luminosity scaling factors, injecting retro bit-depth quantization artifacts, color banding, and vintage hardware glitches.

---

## 4. Color and Palette Management

### 4.1 Native Color Extraction Engine
- Automatically samples and clusters native colors from source imagery using Median Cut and K-Means clustering to generate custom color profiles on image import.

### 4.2 Swatch Controls & Real-Time Histogram
- **Slot Locking**: Individual color swatches in active palettes can be toggled to a "Locked" state, protecting them from global modifications.
- **Selective Randomization**: Shuffles and mutates only unlocked color slots, enabling rapid exploration of color harmonies while preserving base anchor tones.
- **Interactive Swatch Editor**: Dynamic swatch additions, deletions, and live color pickers with instantaneous non-blocking re-rendering.
- **Pixel Allocation Histogram**: Real-time bar chart showing exact pixel percentage allocated to each palette swatch in the active frame with a single-click "Prune Dead Colors" optimizer.

### 4.3 Tonal Sorting & Cycling
- **Ordered Cycling**: Automatically re-indexes active palette entries in strictly ascending or descending luminance ($Y = 0.299R + 0.587G + 0.114B$), enabling real-time palette rotation and cyclic animated color shifting across isolated tonal registers.

---

## 5. Animation and Video Pipeline

### 5.1 Native Multi-Frame Engine
- High-performance streaming pipeline supporting single still images, multi-frame video files (`MP4`, `WebM`), and animated formats (`GIF`, `APNG`, `WebP`).
- **Pure-Rust ISO-BMFF MP4 Demuxer**: Safe parsing of video tracks, timestamps, and sample tables without system FFmpeg dependencies.
- **Pure-Rust EBML / WebM Demuxer**: Safe parsing of WebM/MKV tracks, VP8/VP9/AV1 codecs, and Cluster SimpleBlocks.

### 5.2 Interface Components
- **Timeline Editor**: In-app multi-frame sequence interface with frame trimming, timeline scrub bar, play/pause controls, loop mode, and frame-rate targeting (1 to 60 FPS).
- **Live Playback**: Real-time viewport playback rendering video frames on-the-fly while processing parameters and dither modes are adjusted.

### 5.3 Audio-Reactive DSP Modulation
- **Pure-Rust RIFF WAV Decoder**: Ingests 8-bit, 16-bit, and 32-bit PCM audio streams.
- **Multi-Band Spectrum Analyzer**: Computes sub-bass, mid, treble, and RMS loudness to modulate dither thresholds, CRT scanline jitter, and glitch core displacements.
- **Synchronized Audio Slicing**: Exports frame-synchronized WAV audio files matching animation loop length and FPS.

### 5.4 Temporal Variation & Keyframing
- **9 CRT & LCD Noise Patterns**:
  1. *CRT Scan Beam Jitter*
  2. *Phosphor Grain Hum*
  3. *Interlaced Field Shimmer*
  4. *LCD Subpixel Grid Drift*
  5. *VHS Tape Phase Noise*
  6. *Thermal Analog Drift*
  7. *Static Hash White Noise*
  8. *Bayer Phase Stepping*
  9. *Blue Noise Temporal Jitter*
- **Timeline Keyframing**: Automated property interpolation (brightness, contrast, dither scale, threshold bias, chromatic shift) across frame ranges.

---

## 6. Stackable Downstream Effects & Glitch Processors

Downstream, layer-based effect stack executing after core dithering/quantization:

```text
[Input Image] ──► [Core Dither & Palette] ──► [Downstream Effects Stack] ──► [Output]
```

### Effects Library
1. **Epsilon Glow**: Diffusion bloom algorithm that extracts highlights, applies separable Gaussian convolution, and bleeds luminous halos across high-contrast dithered edges.
2. **Chromatic Aberration**: Lens distortion filter that radially and directionally offsets Red, Green, and Blue channels with subpixel spatial interpolation.
3. **JPEG Glitch Core**: Authentic discrete cosine transform (DCT) corruption with 5 sub-modes:
   - *Block Shift*: Horizontal dislocation of $8\times 8$ and $16\times 16$ macroblocks.
   - *Channel Swap*: Transposes YCbCr / RGB chrominance and luminance planes.
   - *Scan Line Offset*: Staggered horizontal phase tearing across raster rows.
   - *Block Scramble*: Pseudo-random spatial shuffling of image compression tiles.
   - *Interlace Corruption*: Alternating field dropped frames and comb-filter decay.
4. **Vintage CRT Simulation**: Subpixel scanline raster modulation, spherical barrel curvature lens distortion, and radial corner vignette.
5. **Spatial Transforms**: Standard high-pass Sharpen and separable Gaussian Blur.

---

## 7. User Interface, Ergonomics & Viewport

### 7.1 Navigation & Controls
- **Searchable Dropdowns**: Instant fuzzy text filtering for all 63 algorithms and styles without traversing nested category submenus.
- **Adjustment Shuffling**: Dedicated randomizer ("🎲 Shuffle") that rolls new random values for sliders while respecting locked attributes.
- **Scroll Wheel Hardware Toggles**: User preference to enable/disable fine-grained mouse wheel increments when hovering over sliders.
- **Drag-and-Drop Ingestion**: Native file drop support for images, GIFs, MP4, and WebM files.
- **Recent Files Menu**: "Open Recent" submenu tracking historical loaded media.

### 7.2 Rendering Modes & Viewport
- **Infinite Pan & Smooth Zoom**: Cursor-centered zoom from $5\%$ to $3200\%$ with pixel grid overlay automatically active at $\ge 800\%$.
- **Floating Viewport HUD**: Quick-access controls for 1:1 scale, Fit-to-screen, zoom percentage, and grid toggling.
- **Multi-Mode Split View**: Horizontal, Vertical, Checkerboard A/B, and Difference Heatmap comparisons.
- **Debounce Mode**: Power-saving mode pausing live-view rendering while sliders are actively dragged, rendering on release.
- **Pipeline Stage Telemetry HUD**: Live window and status bar tooltip displaying Kernel Dither, Downstream Effects, Swatch Histogram, and Total Latency timings.

### 7.3 Multi-Theme Support (5 Global Themes)
1. *Studio Dark* (Professional dark neutral)
2. *Amber CRT* (Vintage 1980s monochrome terminal)
3. *Cyber Matrix* (High-contrast neon green on pitch black)
4. *Vapor Slate* (Cool retro synthwave tones)
5. *Industrial High-Contrast* (Clean studio monochrome)

---

## 8. Export and Production Pipeline

### 8.1 Export Formats
- **Raster Still Formats**: PNG (lossless 8-bit & 24-bit), JPEG (quality adjustable), WebP (lossless & lossy).
- **Animation Formats**: Animated GIF, Animated APNG (full 24-bit color depth), Animated WebP (lossless `VP8L` container with `ANMF` chunks).
- **Audio Formats**: Synchronized WAV PCM audio slices.
- **Vector & CNC Formats**:
  - SVG (single-layer black vector contour).
  - Multi-Color Layered SVG (dedicated `<g id="color-name">` per palette swatch for multi-pen AxiDraw plotters).
  - Smooth Bézier SVG ($C^1$-continuous cubic spline curves).
  - G-Code Toolpaths (`G00` / `G01` CNC toolpaths with Traveling Salesperson path optimization).

### 8.2 Production Automation
- **Black Vector Isolation**: Precision contour tracing algorithm isolating dark pixel data into optimized vector paths.
- **Smart Suggested Filenames**: Formatted as `{stem}_{algo}_{palette}.{ext}` across all export dialogs.
- **Desktop Batch Processing Studio**: Parallel multi-image directory processing queue with live progress telemetry.

---

## 9. Performance & Quality Invariants

| Invariant | Standard |
| :--- | :--- |
| **GUI Responsiveness** | Main thread $\ge 60$ FPS. Dithering runs on background Rayon worker pools. |
| **Allocations** | Zero dynamic heap allocations in per-pixel processing hot loops. |
| **Reliability** | No `unwrap()` or `panic!` in production paths. Errors handled as `Result<T, DitherError>`. |
| **Decoupling** | `crates/dither-core` has zero GUI/windowing dependencies. |
| **Binary Footprint** | Standalone production GUI binary $< 20\text{ MB}$; CLI $< 5\text{ MB}$. |

---

## 10. Next-Gen Roadmap Evolution Specifications (Phases 7–13)

### 10.1 Phase 7: Ultra-Performance Engine & SIMD Kernels
- **AVX2 / NEON Batch Quantization [Delivered in v0.9.8]**: SIMD batch distance calculation vectorizing Euclidean RGB and OKLab Euclidean distances (`L`, `a`, `b`), evaluating 8–16 pixels per register concurrently with zero branch divergence (`quantize_batch_8`, `quantize_buffer_simd`).
- **Tile-Based Parallel Wavefront Error Diffusion [Delivered in v0.9.8]**: Partition images into diagonal wavefront bands ($x + y = k$) or stepped tiles, breaking serial horizontal dependencies and unlocking multi-core Rayon acceleration for error diffusion (`diffuse_wavefront_parallel`, `ParallelWavefrontFloydSteinberg`).
- **Fractal Space-Filling Curves [Delivered in v0.9.5]**: Recursive Hilbert space-filling curve generator (`HilbertCurve`) in `crates/dither-core/src/buffer.rs` with exact bidirectional bijective mapping (`d2xy` and `xy2d`) and ordered path visitation.
- **Zero-Allocation Ring Buffers [Delivered in v0.9.5]**: Pre-allocated memory arena (`ImageBufferPool`) with `acquire` and `release` recycling buffers during animation scrubbing, alongside `ImageBuffer::adjust_levels(black_point, white_point, gamma)` 256-entry lookup table.

### 10.2 Phase 8: Studio-Grade Professional UI/UX & Creative Workstation
- **Interactive Tone Curves & Levels Editor**: Cubic spline curves for Master Luminance, Red, Green, and Blue channels with real-time pre-dither histogram display and draggable Black/Mid/White points.
- **Draggable & Reorderable Downstream Effects Stack [Delivered in v0.9.8]**: Visual effect pipeline allowing layer reordering (⬆/⬇), duplicate/delete, per-effect blend modes (Normal, Multiply, Screen, Overlay, Color Dodge, Difference), and opacity sliders (`EffectStack`, `EffectNode`).
- **Floating Precision Magnifier Loupe [Delivered in v0.9.5]**: Real-time cursor-tethered $9 \times 9$ subpixel inspection loupe displaying coordinates `(X, Y)`, `RGB(r, g, b)`, and `#RRGGBB` hex readout (hotkey `L`).
- **Instant A/B Preset Snapshot Comparator [Delivered in v0.9.5]**: 4 quick snapshot memory slots (`F5`–`F8` recall, `Shift+F5`–`F8` store) for instantaneous comparison between different algorithm configurations.
- **Studio Layout & Keyboard HUD [Delivered in v0.9.5]**: In-app keyboard shortcut cheatsheet modal dialog (`?` / `F1`) summarizing navigation controls, snapshot slots, and loupe toggles.
- **Visual Preset Browser & Thumbnail Matrix**: Real-time multi-preset gallery rendering active canvas across all presets simultaneously in a responsive thumbnail matrix.
- **Canvas Viewport Rulers & Alignment Guides**: Top and left edge pixel rulers, draggable guide lines with magnetic snapping, and retro framing guides (Game Boy 10:9, CGA 4:3, NES 8:7, Macintosh 4:3, Arcade 3:4).
- **Canvas Geometry Workshop [Delivered in v0.9.8]**: Lossless 90°/180°/270° rotation, horizontal/vertical flips, integer pixel scaling ($1\times, 2\times, 4\times, 8\times$), and interactive crop dialog with HUD buttons.
- **Color Vision Deficiency (CVD) Simulation [Delivered in v0.9.8]**: Perceptual simulation filters based on Machado et al. (2009) and Brettel et al. (1997) for Protanopia, Deuteranopia, Tritanopia, and Achromatopsia accessibility auditing (`CvdMode`).

### 10.3 Phase 9: Creative DSP, Generative Masks & Production
- **Selective Dithering & Spatial Alpha Masking**: Paint-brush masks, gradient alpha masks, and luminance range thresholding (e.g., dither only dark shadows while keeping highlights smooth).
- **Custom Threshold Matrix Designer [Delivered in v0.9.8]**: Interactive in-app matrix editor to test custom ordered dithering patterns and typography stamps.
- **Pure-Rust 3D LUT Vintage Film Emulation [Delivered in v0.9.8]**: Zero-dependency `.cube` 3D Look-Up Table parser and trilinear interpolator simulating 1970s instant film, vintage Technicolor, and arcade CRT phosphors.
- **Production Batch Automation**: Multi-target simultaneous export (PNG, WebP, SVG, and WAV in one pass) with smart naming templates.

### 10.4 Phase 10: Advanced Printmaking & Physical Fabrication Studio
- **CMYK Rosette Dot-Screen Angle Separations [Delivered in v0.9.8]**: Traditional offset printing rosette screens with standard screen angles: Cyan (15°), Magenta (75°), Yellow (0°), and Black (45°) to prevent moiré interference. Configurable screen frequency (15–150 LPI), Euclidean round/elliptical/line dot geometries, and Yule-Nielsen dot gain compensation.
- **Physical Spot Color Separations & Registration Target Generator [Delivered in v0.9.8]**: Channel separation exporting individual positive films per palette swatch with automated corner crosshairs (`+`), center bullseyes, and density step-wedge calibration strips.
- **Pen Plotter Arc Fitting (G02/G03) & Nib Offsetting [Delivered in v0.9.8]**: Bi-directional stroke offsetting simulating actual pen nib widths ($0.1\text{mm}$ to $1.2\text{mm}$) and circular arc fitting converting dense polylines to native `G02`/`G03` commands.
- **Geometric & Stippled Hatch Infill Patterns [Delivered in v0.9.8]**: Multi-angle parallel line hatching, cross-hatching, concentric spiral infills, and Poisson-disk stippling for closed vector outlines.

### 10.5 Phase 11: Non-Linear Video & Motion Graphics Suite
- **Keyframed Parameter Automation Timeline [Delivered in v0.9.8]**: Multi-track timeline supporting keyframes for algorithm morphing, CRT scanline jitter, barrel curvature, and glitch intensity with cubic Bézier easing curves.
- **Inter-Frame Temporal Coherence & Flickering Suppression [Delivered in v0.9.8]**: Motion-compensated inter-frame error propagation ($t \to t+1$) suppressing high-frequency noise and temporal flicker (`TemporalCoherenceFilter`).
- **Onion Skinning & Pixel Animation Auditing [Delivered in v0.9.8]**: Multi-frame onion skinning displaying preceding (red tint) and succeeding (cyan tint) frames with adjustable opacity.
- **Temporal Frame Decimation & Vintage Refresh Emulation [Delivered in v0.9.8]**: Variable cadence decimation simulating 60Hz NTSC, 50Hz PAL, 24fps cinema, 12fps anime, and 8fps stop-motion.

### 10.6 Phase 12: Generative DSP & Procedural Texture Synthesis
- **Procedural Blue Noise Void & Cluster Synthesizer [Delivered in v0.9.8]**: Pure-Rust Void and Cluster algorithm (Ulichney 1993) synthesizing customized blue noise masks with optimal isotropic high-frequency energy.
- **Reaction-Diffusion & Morphogenetic Dither Kernels [Delivered in v0.9.8]**: Real-time Gray-Scott reaction-diffusion equations generating biological labyrinthine, coral, and leopard spot dither textures.
- **Vector Flow Field Steered Dithering [Delivered in v0.9.8]**: Directional error diffusion paths steered along continuous 2D vector flow fields generated via pseudo-random vortex noise.

### 10.7 Phase 13: WebAssembly Studio & Zero-Dependency Plugin API
- **Serverless Browser-Native WebAssembly Workstation [Delivered in v0.9.8]**: Native compilation to `wasm32-unknown-unknown` under `cargo xtask ci` with zero host OS dependencies.
- **Stable C/Rust DSP Plugin ABI (`dither_core_ffi`) [Delivered in v0.9.8]**: Stable C ABI definition and C header (`include/dither_core_ffi.h`) allowing community developers to author custom dithering kernels and post-effects as native dynamic libraries or Wasm modules.

---

## 11. Next-Gen Studio Workstation Specifications (Phases 14–18)

### 11.1 Phase 14: Studio-Grade UI/UX Architecture & Ergonomics
- **Modular Docking System (`egui_dock`)**: Dockable, tear-off tabbed split panels for Viewport, Algorithm Parameters, Effect Stack, Curves/Levels, Palette Studio, Timeline, Mask Inspector, and Telemetry.
- **Universal Command Palette (`Ctrl+K` / `Cmd+K`)**: Fuzzy-matched search launcher for instant algorithm activation, preset loading, transforms, and export operations.
- **Studio Theme Customization**: Pre-calibrated palettes for *Cyberpunk Neon*, *Amber Phosphor*, *Green VT220*, *Macintosh Platinum 1984*, and *Slate Studio*, with high-DPI font scaling ($75\%\dots 200\%$).
- **Telemetry HUD & Memory Arena Gauge**: Live microsecond per-stage profiling ($T_{\text{dither}}, T_{\text{effects}}, T_{\text{gpu}}$), memory pool allocation counter, and AVX2/NEON SIMD active badges.
- **Custom Keymap Rebinding**: Configurable hotkey profiles with Photoshop, Aseprite, and Blender compatibility layouts.

### 11.2 Phase 15: Spatial Masking & Dual-Algorithm Blending
- **Paintbrush & Eraser Mask Tool**: Interactive raster paintbrush modifying an 8-bit grayscale `MaskBuffer` with configurable radius ($1\dots 512\text{px}$), Hermite smoothstep edge feathering ($0\%\dots 100\%$), opacity, and stylus pressure sensitivity.
- **Procedural Gradient Masks**: Interactive linear, radial/vignette, and conical gradient generation with boolean invert/clear operators.
- **Tonal & Color Proximity Range Masking**: Parametric luminance isolation (Shadows, Midtones, Highlights) and OKLab $\Delta E$ color range extraction.
- **Dual-Algorithm Parallel Scanline Blending**: Independent algorithm dispatch (Algorithm A inside mask, Algorithm B outside mask) blended in linear sRGB color space via zero-allocation Rayon chunking.
- **Quick Mask Ruby Overlay (`Q`)**: Translucent `#FF005580` viewport mask feedback.

### 11.3 Phase 16: Interactive Selection Tools & Vector Marquees
- **Geometric Marquee Selection**: Rectangular (`M`) and Elliptical (`Shift+M`) marquees with 1:1 aspect constraint and pixel dimension locking.
- **Freeform & Semantic Selection**: Freehand Lasso (`L`), Polygonal Lasso, Magic Wand (`W`) flood selection with OKLab distance tolerance, and global color range selection.
- **Animated Marching Ants Overlay**: Real-time 2D Moore-neighbor contour stroke shader with $2\text{ Hz}$ phase rotation.
- **Selection Arithmetic & Boundary Modifiers**: Add (`Shift`), Subtract (`Alt`), Intersect (`Shift+Alt`), feathering ($1\dots 64\text{px}$), expand/contract dilation/erosion, and spline smoothing.
- **Selection-to-Mask Interoperability**: Instant bidirectional conversion (`Ctrl+Shift+M` / `Ctrl+Click`) and selection-bounded rendering acceleration.

### 11.4 Phase 17: Enterprise Media Ingestion & Robust File Pipeline
- **Asynchronous Multi-Format Decoders**: Non-blocking Rayon loading pipeline for PNG, JPEG, WebP, GIF, BMP, TIFF, TGA, ICO, QOI, AVIF, HDR, SVG (dynamic DPI rasterization), and animated APNG/WebP/GIF.
- **Gigapixel Memory-Mapped Streaming (`memmap2`)**: Tiled memory-mapped access for images $> 100\text{ MB}$ or $> 16\text{ MP}$ with decompression bomb dimension safeguards.
- **Desktop Drag-and-Drop Router**: Native OS drag-and-drop routing for images, animation sequences, `.dcpreset` files, `.cube` LUTs, and `.gpl` palettes.
- **File System Watcher & Auto-Reload**: Background disk watcher (`notify` crate) automatically refreshing the canvas when external creative apps save source edits.
- **Persistent Recent Files Registry**: SQLite/JSON history tracking up to 30 items with $96\times 96$ cached thumbnails and missing file validation.
- **Image Sequence Importer**: Automated numbered sequence ingestion (`img_001.png`... `img_999.png`) directly onto the animation timeline.

### 11.5 Phase 18: Project Bundles, Batch Automation Studio & Ecosystem
- **Portable Project Bundle Format (`.dithercraft` / `.dcp`)**: Compressed ZIP archive preserving source frames, `project.json`, embedded masks, custom matrices, and 3D LUTs.
- **Multi-Target Batch Automation Studio**: Multi-threaded folder batch export with customizable file templates (`{stem}_{algo}_{palette}_{dims}_{timestamp}`).
- **OS Clipboard Interoperability**: Direct pixel buffer copy (`Ctrl+C`) and paste (`Ctrl+V`) to/from OS clipboard.
- **Headless JSON-RPC / MCP Streaming Daemon**: Remote control IPC protocol for programmatic automation and AI agent workstation control.
