# DitherCraft (Rust) — Architecture & Capability Gap Analysis (`docs/gaps.md`)

This document assesses the architectural boundaries, hardware constraints, and future exploration vectors for **DitherCraft**.

---

## 1. Current State Assessment

DitherCraft has achieved complete clean-room parity with its reference design across:
- **63 Dithering Algorithms**: Error diffusion, ordered matrices, adaptive thresholds, halftones, modulation, and artistic filters.
- **Color Science**: CIELAB $\Delta E^*$, Redmean, Euclidean RGB, 3D KD-Tree spatial nearest neighbor acceleration, Median Cut & K-Means clustering.
- **Effects & CRT Simulation**: Post-dither bloom glow, scanlines, barrel distortion, vignette, chromatic aberration, authentic 5-mode JPEG glitch core.
- **Animation & Timeline**: 9 temporal noise patterns, timeline scrub toolbar, parameter keyframing, and multi-frame animated GIF pipeline.
- **Vector Isolation**: Native black pixel contour tracing to SVG paths.
- **CLI & MCP Automation**: High-throughput multi-threaded batch engine and stdio Model Context Protocol server.

---

## 2. Identified Gaps & Future Opportunity Vectors

### 2.1 GPU Compute Acceleration (WGSL)
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented standalone WebGPU Shading Language (WGSL) compute kernels in `dither_core::shaders`:
    - `BAYER_ORDERED_COMPUTE_WGSL`: 16×16 workgroup compute kernel executing 8×8 Bayer ordered matrix thresholding against storage buffer palettes.
    - `HALFTONE_SCREEN_COMPUTE_WGSL`: Compute kernel calculating rotated screen coordinate grids and frequency-modulated dot matrices.
    - `CRT_POSTPROCESS_WGSL`: Unified compute pass performing barrel curvature lens distortion, scanline sinusoidal modulation, chromatic aberration pixel displacement, and vignette shading.
  - Zero external GUI/graphics crate coupling maintained within `crates/dither-core`.

### 2.2 Direct Video Codec Container Demuxing (Pure-Rust ISO-BMFF MP4 & APNG)
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented pure-Rust `Mp4Demuxer` in `dither_core::video` parsing ISO-BMFF boxes (`ftyp`, `moov`, `mvhd`, `trak`, `tkhd`, `mdia`, `stbl`, `stsd`, `stts`, `stsc`, `stsz`, `stco`, `co64`, `stss`) to extract video track metadata, nominal FPS, and sample offsets without system FFmpeg dependencies.
  - Integrated direct MP4 video loading in the desktop UI and `mp4-info` inspection in the CLI.
  - Implemented APNG (Animated Portable Network Graphics) full 24-bit color depth exporter across desktop GUI and CLI without external C libraries.

### 2.3 Advanced Perceptual Color Spaces (CIEDE2000 & OKLab)
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Integrated Björn Ottosson's **OKLab** perceptual color space (`OklabColor`), eliminating the blue-to-purple hue distortion of CIELAB 1976.
  - Implemented the complete ISO/CIE standard **CIEDE2000** ($\Delta E_{00}$) equation with hue rotation terms ($R_T$) and parametric compensation factors ($k_L, k_C, k_H$).
  - Added `ColorMetric::Oklab` and `ColorMetric::Ciede2000` to `dither-core` and the desktop UI picker.

### 2.4 Vector Shaper Curve Smoothing & Douglas-Peucker Simplification
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented `rdp_simplify` (**Ramer-Douglas-Peucker**) polyline simplification on `Point2D` geometries, eliminating collinear line points while preserving sharp contour features.
  - Implemented `MultiColorSvgSeparation`: Multi-layer SVG separation exporting dedicated `<g id="color-..." fill="...">` layers per palette swatch for pen plotters (AxiDraw) and screen printing.
  - Implemented `GCodeExporter`: Direct CNC / plotter toolpath generator featuring Traveling Salesperson (TSP) nearest-neighbor path sorting to minimize rapid pen-up travel time.

### 2.5 Studio Presets & Command History
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Added `StudioPreset` serialization supporting one-click import and export of workstation configurations (`.dcpreset` / `.json`).
  - Bundled 5 curated workstation profiles: *Game Boy Nostalgia*, *Cyberpunk Neon*, *1980s VHS Broadcast*, *Newspaper Halftone*, *Macintosh 1984 Classic*.
  - Added command history undo/redo stack with keyboard accelerators (`Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+Y`).

### 2.6 Visvalingam-Whyatt Polygon Simplification & Bézier Splines
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented `visvalingam_simplify`: Area-elimination polyline simplification removing vertices by minimal triangular area, preserving natural organic outlines without angular spikes.
  - Implemented `fit_cubic_spline` & `export_smooth_bezier_svg`: $C^1$-continuous piecewise cubic Bézier curve fitting over contours, generating smooth `<path d="M... C...">` vector strokes for AxiDraw plotters.

### 2.7 Desktop Batch Processing Studio & Parallel Queue
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Built a dedicated Batch Processing Studio window inside the desktop application (`apps/dithercraft`).
  - Enables folder-level multi-image batch conversion with non-blocking Rayon worker threads, asynchronous image decoding/encoding, and progress telemetry.

### 2.8 Audio-Reactive DSP Modulation (Pure-Rust RIFF WAV)
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented zero-dependency pure-Rust `AudioTrack::parse_wav` decoding 8-bit, 16-bit, and 32-bit PCM audio streams.
  - Implemented windowed multi-band spectrum analyzer `get_frame_spectrum` computing sub-bass, mid, treble, and RMS loudness to drive dither threshold pulsation, CRT jitter, and glitch bursts.

### 2.9 Live Interactive Swatch Editing in Palette Drawer
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Integrated interactive `color_edit_button_srgb` color pickers for every swatch in the palette drawer.
  - Added live dynamic swatch addition (`+ Add Swatch`), single-click swatch removal (`✕`), and instantaneous non-blocking canvas re-dithering.

### 2.10 Progressive Downsampled Preview & Deterministic Golden Testing
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented `ImageBuffer::downsample(max_w, max_h)` providing $< 2\text{ms}$ low-latency rendering during active slider interaction, automatically triggering high-resolution passes on release.
  - Implemented FNV-1a 64-bit golden checksum regression test suite verifying bit-exact cross-platform determinism across algorithm pipelines.

### 2.11 Multi-Format Palette Exporters & Binary ACT Color Table Support
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented bidirectional palette exporters for GIMP (`.gpl`), plain hex (`.hex`), JASC-PAL (`.pal`), Photoshop Color Table (`.act`), and JSON.
  - Added CLI `extract-palette` command for automated palette generation from images via Median-Cut and K-Means.
  - Added exact-match fast path in nearest-neighbor palette lookup, accelerating real-time histogram calculation by up to $10\times$.

### 2.12 Pure-Rust EBML / WebM Container Demuxing
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented pure-Rust `WebmDemuxer` in `dither_core::video` decoding EBML variable-size integer IDs (VINT), Element Sizes, and Master Elements without external C/FFmpeg libraries.
  - Demuxes `Segment`, `Info` (`TimecodeScale`), `Tracks` (`TrackEntry`, `TrackNumber`, `TrackType`, `CodecID` e.g. VP8/VP9/AV1, `PixelWidth`, `PixelHeight`), and `Cluster` (`SimpleBlock` / `Block`).
  - Added direct `.webm` / `.mkv` media loading in the desktop workstation and `webm-info` inspection in the CLI.

### 2.13 Pure-Rust Animated WebP Container Multiplexer & Modern Animation Pipeline
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented zero-dependency `WebpAnimationMuxer` and `WebpAnimFrame` in `dither_core::video` assembling valid RIFF WebP animation streams containing `VP8X` extended flags, `ANIM` background/loop parameters, and per-frame `ANMF` subchunks.
  - Integrated "Export Animated WebP..." in the desktop workstation with parallel Rayon frame rendering and lossless `VP8L` bitstream compression.
  - Verified bit-exact format compliance and decoding roundtrip with `image_webp::WebPDecoder`.

### 2.14 Synchronized Audio WAV Slicing & Audio Reactive Timeline Loops
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented `AudioTrack::export_wav` and `AudioTrack::export_wav_slice` in `dither_core::audio` exporting 16-bit PCM standard RIFF WAV byte streams precisely matched to frame count and target FPS.
  - Added "Export Synchronized Audio (.wav)..." to the desktop workstation File menu and `trim-audio` subcommand to the CLI.

### 2.15 Structured Pipeline Telemetry & Interactive Stage Breakdown HUD
- **Current Status**: **RESOLVED** (`v0.9.0`).
  - Implemented multi-stage latency telemetry tracking: Kernel Dither time (`dither_ms`), Downstream Effects stack time (`effects_ms`), Swatch Histogram quantization (`histogram_ms`), and total elapsed time.
  - Added dedicated "View -> Pipeline Telemetry HUD" window displaying real-time stage performance alongside generation ID and engine state.
  - Integrated status bar hover tooltip showing instantaneous per-stage telemetry.

### 2.16 Hilbert Fractal Space-Filling Scan Curves
- **Current Status**: **RESOLVED** (`v0.9.5`).
  - Implemented pure-Rust bidirectional `HilbertCurve` in `crates/dither-core/src/buffer.rs` with exact bijective mapping functions (`d2xy` and `xy2d`).
  - Added `HilbertCurve::generate_order(width, height)` enabling fractal scanline traversal that maintains spatial locality, maximizes CPU L1/L2 cache efficiency, and completely eliminates horizontal sweep asymmetry.

### 2.17 Reusable ImageBuffer Arena Pool & Pre-Dither LUT Tone Levels
- **Current Status**: **RESOLVED** (`v0.9.5`).
  - Implemented `ImageBufferPool` bounded memory arena in `crates/dither-core/src/buffer.rs` with `acquire` and `release` methods recycling image buffers during high-FPS timeline playback.
  - Implemented `ImageBuffer::adjust_levels(black_point, white_point, gamma)` with a 256-element precomputed lookup table for zero-allocation pre-dither tone transfer.

### 2.18 Instant A/B Preset Snapshot Comparator
- **Current Status**: **RESOLVED** (`v0.9.5`).
  - Implemented 4 instant preset snapshot slots (`[A]`, `[B]`, `[C]`, `[D]`) in `apps/dithercraft/src/main.rs`.
  - Hotkeys: `F5`–`F8` recall snapshots with background Rayon rendering; `Shift + F5`–`F8` (or right-click) store the active workstation state.
  - Accessible via top Presets menu, bottom status bar, and timeline panel.

### 2.19 Floating Precision Magnifier Loupe & Subpixel Inspector
- **Current Status**: **RESOLVED** (`v0.9.5`).
  - Implemented interactive $9 \times 9$ subpixel magnifying HUD tethered to cursor in `apps/dithercraft/src/main.rs`.
  - Real-time readout of pixel coordinates `(X, Y)`, `RGB(r, g, b)`, and `#RRGGBB` hex code.
  - Toggled via `L` key, the floating Viewport HUD, and the View menu.

### 2.20 Studio Keyboard Shortcuts Cheatsheet Modal
- **Current Status**: **RESOLVED** (`v0.9.5`).
  - Implemented structured keyboard shortcut cheatsheet modal dialog (`?` / `F1`) documenting canvas navigation, zoom shortcuts, A/B snapshots, and loupe hotkeys.

---

## 3. Active Next-Gen Capability & Performance Vectors (Phases 7–13)

### 3.1 SIMD Vectorized Color Distance Acceleration (Phase 7)
- **Target Milestone**: `v1.0.0`
- **Objective**: Accelerate the per-pixel palette quantization inner loop using packed AVX2 (x86_64) and NEON (AArch64) vector instructions.
- **Specification**: Vectorize batch Euclidean and OKLab distance equations to evaluate 8 or 16 pixels against active palette color swatches in parallel vector lanes with zero branch divergence.
- **Success Metric**: Latency $< 1.5\text{ms}$ for full 4K UHD ($3840\times 2160$) image quantization.

### 3.2 Tile-Based Parallel Wavefront Error Diffusion (Phase 7)
- **Target Milestone**: `v1.0.0`
- **Objective**: Overcome the serial scanline bottleneck of classical error diffusion (Floyd-Steinberg, Atkinson, Stucki, JJN).
- **Specification**: Partition large frames into diagonal wavefront tiles ($x + y = k$), scheduling non-overlapping tiles across Rayon thread pools concurrently.
- **Success Metric**: $> 1\text{ Gigapixel/sec}$ aggregate batch throughput across multi-core processors.

### 3.3 Interactive Dual-Tone Curves & Levels Histogram Tool (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Studio-grade tonal manipulation prior to discrete quantization.
- **Specification**: Cubic spline curve editor for Master Luminance, R, G, B with real-time pre-dither histogram distribution and draggable Black/Mid/White points.
- **Success Metric**: Real-time $< 1\text{ms}$ curve remapping with interactive visual feedback.

### 3.4 Reorderable Downstream Effects Stack (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Flexible layer-based post-processing architecture.
- **Specification**: Drag-and-drop layer reordering for Bloom, Chromatic Aberration, Glitch Core, CRT, Blur, and Sharpen with per-layer blend modes (Multiply, Screen, Overlay) and bypass toggles.
- **Success Metric**: Interactive drag-and-drop reordering with instantaneous canvas updates.

### 3.5 OKLCH Color Wheel & Retro Harmony Generator (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Academic color harmony exploration mapped to retro color depth constraints.
- **Specification**: Circular OKLCH / HSV color wheel in the Palette Drawer generating Complementary, Triadic, Analogous, and Tetradic harmonies remapped to discrete hardware bit depths (1-bit to 8-bit).
- **Success Metric**: Instant harmony swatch generation with one-click canvas preview.

### 3.6 Visual Preset Browser & Live Thumbnail Matrix (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Rapid visual discovery across diverse aesthetic profiles.
- **Specification**: Responsive multi-thumbnail gallery rendering active image across all presets simultaneously side-by-side with 1-click apply.
- **Success Metric**: Multi-thumbnail grid rendered in $< 20\text{ms}$ total elapsed background time.

### 3.7 Canvas Viewport Rulers, Magnetic Guides & Retro Aspect Grids (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Pixel-perfect layout alignment and retro display framing.
- **Specification**: Border pixel rulers, draggable magnetic guidelines, and framing aspect overlays (Game Boy 10:9, CGA 4:3, NES 8:7, Arcade 3:4).
- **Success Metric**: Crisp hairline rendering with zero overhead on canvas panning/zooming.

### 3.8 Canvas Geometry Workshop: Crop, Rotate, Flip & Integer Scaling (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Non-destructive geometric image preparation.
- **Specification**: Aspect-constrained crop box, 90° rotations, axis flips, and integer pixel scaling ($1\times, 2\times, 4\times, 8\times$).
- **Success Metric**: Lossless integer pixel upscaling and responsive crop bounding box.

### 3.9 Color Vision Deficiency (CVD) Real-Time Accessibility Filters (Phase 8)
- **Target Milestone**: `v1.0.0`
- **Objective**: Accessibility and high-contrast print legibility auditing.
- **Specification**: Real-time viewport simulation modes based on Machado et al. (2009) and Brettel et al. (1997) for Protanopia, Deuteranopia, Tritanopia, and Achromatopsia.
- **Success Metric**: 60 FPS real-time color re-mapping via direct 3x3 matrix multiplication.

### 3.10 Selective Spatial Dithering & Alpha Masking (Phase 9)
- **Target Milestone**: `v1.1.0`
- **Objective**: Non-destructive region-selective and luminance-band dithering.
- **Specification**: Paint-brush masks, gradient alpha masks, and luminance range thresholding (e.g., dither only dark shadows while keeping highlights smooth).
- **Success Metric**: Seamless feathering and alpha composite blending between dithered textures and pristine source layers.

### 3.11 Custom Threshold Matrix Designer (Phase 9)
- **Target Milestone**: `v1.1.0`
- **Objective**: Creative custom pattern generation for ordered dithering and halftones.
- **Specification**: In-app $N \times N$ matrix workshop ($2\times 2$ to $16\times 16$) to draw, import, and test custom ordered dithering patterns, typography stamps, and logo halftones.
- **Success Metric**: Dynamic compiler turning user matrices into zero-allocation threshold lookups.

### 3.12 Pure-Rust 3D LUT Vintage Film Emulation (Phase 9)
- **Target Milestone**: `v1.1.0`
- **Objective**: Authentic vintage film and monitor phosphor color transformation.
- **Specification**: Zero-dependency parser and trilinear interpolator for `.cube` (17×17×17, 33×33×33) 3D Look-Up Tables.
- **Success Metric**: Bit-accurate trilinear interpolation running in $< 3\text{ms}$ on 1080p frames.

### 3.13 CMYK Rosette Dot-Screen Angle Separations (Phase 10)
- **Target Milestone**: `v1.1.0`
- **Objective**: Professional offset printing screen separations with zero moiré.
- **Specification**: Angular rosette halftoning with standard screen angles: Cyan (15°), Magenta (75°), Yellow (0°), and Black (45°) with configurable LPI and Yule-Nielsen dot gain compensation.
- **Success Metric**: Moiré-free rosette patterns verified across synthetic gradients.

### 3.14 Physical Spot Color Separations & Registration Target Generator (Phase 10)
- **Target Milestone**: `v1.1.0`
- **Objective**: Automated screen printing and risograph plate preparation.
- **Specification**: Export individual printable positive films per swatch with corner registration marks (`+`), center bullseyes, and density calibration strips.
- **Success Metric**: Valid multi-page or separated SVG/PDF files conforming to silkscreen standards.

### 3.15 Pen Plotter Arc Fitting (G02/G03) & Bi-Directional Nib Offsetting (Phase 10)
- **Target Milestone**: `v1.1.0`
- **Objective**: Smooth mechanical plotter toolpaths and paper tear prevention.
- **Specification**: Bi-directional stroke offsetting simulating nib widths ($0.1\text{mm}$ to $1.2\text{mm}$) and circular arc fitting converting polylines to native `G02`/`G03` commands.
- **Success Metric**: $\ge 60\%$ reduction in G-code command count and zero plotter pen chatter.

### 3.16 Geometric & Stippled Hatch Infill Patterns (Phase 10)
- **Target Milestone**: `v1.1.0`
- **Objective**: Solid and tonal hatching inside vector boundary paths.
- **Specification**: Parallel line hatching, dual cross-hatching, concentric spiral infills, and Poisson-disk stippling for closed vector outlines.
- **Success Metric**: Continuous non-lifting infill paths maximizing plotting speed.

### 3.17 Keyframed Parameter Automation Timeline (Phase 11)
- **Target Milestone**: `v1.2.0`
- **Objective**: Dynamic parameter animation for music videos and cutscenes.
- **Specification**: Multi-track timeline supporting keyframes for algorithm morphing, CRT scanlines, barrel curvature, and glitch intensity with cubic Bézier easing.
- **Success Metric**: Smooth 60 FPS playback with real-time parameter interpolation.

### 3.18 Inter-Frame Temporal Coherence & Flickering Suppression (Phase 11)
- **Target Milestone**: `v1.2.0`
- **Objective**: Elimination of temporal swimming noise in video sequences.
- **Specification**: Propagate residual quantization errors across consecutive frames ($t \to t+1$) with optical-flow motion compensation.
- **Success Metric**: $\ge 50\%$ reduction in temporal inter-frame pixel variance on static backgrounds.

### 3.19 Traditional Animation Lightbox & Multi-Frame Onion Skinning (Phase 11)
- **Target Milestone**: `v1.2.0`
- **Objective**: Precise pixel rotoscoping and frame alignment.
- **Specification**: Multi-frame onion skinning ghosting preceding (red) and succeeding (green) frames with configurable opacity.
- **Success Metric**: Interactive frame stepping with instantaneous composite overlay rendering.

### 3.20 Procedural Blue Noise Void & Cluster Synthesizer (Phase 12)
- **Target Milestone**: `v1.3.0`
- **Objective**: Custom isotropic high-frequency dither masks without external textures.
- **Specification**: Pure-Rust Void and Cluster algorithm (Ulichney 1993) generating blue noise masks of any dimension ($32\times 32$ to $512\times 512$).
- **Success Metric**: Flat blue noise radially averaged power spectrum with zero DC leakage.

### 3.21 Reaction-Diffusion & Morphogenetic Dither Kernels (Phase 12)
- **Target Milestone**: `v1.3.0`
- **Objective**: Biological and organic generative dither aesthetics.
- **Specification**: Real-time Gray-Scott reaction-diffusion equations generating labyrinthine, leopard spot, and coral stippling.
- **Success Metric**: Stable PDE simulation at $> 60\text{ FPS}$ on $512\times 512$ grids.

### 3.22 Vector Flow Field Steered Dithering (Phase 12)
- **Target Milestone**: `v1.3.0`
- **Objective**: Directional organic texturing and flow contour dithering.
- **Specification**: Error diffusion steered along 2D vector flow fields generated from Perlin/Simplex gradient noise.
- **Success Metric**: Continuous organic grain alignment following user-tuned flow angles.

### 3.23 Serverless Browser-Native WebAssembly Workstation (Phase 13)
- **Target Milestone**: `v1.4.0`
- **Objective**: Zero-install instant browser accessibility with 100% client-side privacy.
- **Specification**: Native compilation to `wasm32-unknown-unknown` utilizing WebGPU/WebGL2 shaders and WebAssembly SIMD (`wasm-simd128`).
- **Success Metric**: Clean compilation and full GUI functionality in modern browsers.

### 3.24 Stable C/Rust DSP Plugin ABI (`dither_core_ffi`) (Phase 13)
- **Target Milestone**: `v1.4.0`
- **Objective**: Extensible community DSP plugin ecosystem.
- **Specification**: Stable C ABI definition allowing third-party developers to author custom dither kernels and post-effects as native dynamic libraries or Wasm modules.
- **Success Metric**: Zero-copy host-plugin memory sharing and hot-reloadable plugin drawer.

---

## 4. Reliability & Zero-Panic Assurance

| Criterion | Current Implementation | Status |
|---|---|---|
| **No Panics in Shipped Code** | All workspace code denies `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`. | Verified (100% clean) |
| **Strict Crate Layering** | `dither-core` has zero GUI/windowing dependencies. | Verified (`cargo xtask layers`) |
| **Non-Blocking UI** | All dither calculations and effects execute on Rayon background workers. | Verified ($\ge 60\text{ FPS}$) |
| **Release Footprint** | Desktop GUI: 12 MB. Headless CLI: 1.9 MB. | Verified ($<20\text{ MB}$ target) |
