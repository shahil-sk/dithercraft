# DitherCraft — Comprehensive Engineering Roadmap & Stability Architecture

**Document Version:** 3.0.0  
**Project:** DitherCraft — Studio-Grade Retro Graphics Workstation in Pure Rust  
**Last Updated:** October 12, 2026  
**Status:** Phases 1–6 Completed (`v0.9.0`); Phases 7–9 Active Development (`v1.0.0+`)  

---

## Strategic Vision

This roadmap establishes the strategic engineering vision for **DitherCraft** to evolve from its core algorithm baseline into a production-grade, zero-crash, high-utility retro graphics workstation, DSP engine, and vector preparation tool in Pure Rust.

```mermaid
flowchart TD
    subgraph CoreFoundations["Foundation & Color Science"]
        S1["Phase 1: Engine Stability & Memory Safety"] --> S2["Phase 2: Color Science & Visual Fidelity"]
        S2 --> S3["Phase 3: Studio UX & Canvas Ergonomics"]
        S3 --> S4["Phase 4: Vector Shaper & CNC Plotter Toolpaths"]
        S4 --> S5["Phase 5: GPU Compute & Realtime WGSL Acceleration"]
        S5 --> S6["Phase 6: Multimedia, Audio-DSP & Video Containers"]
    end

    subgraph FutureEvolution["Next-Gen Evolution (v1.0.0+)"]
        S6 --> S7["Phase 7: Ultra-Performance Engine & SIMD Kernels"]
        S7 --> S8["Phase 8: Studio-Grade Professional UI/UX & Workstation"]
        S8 --> S9["Phase 9: Creative DSP, Generative Masks & Production"]
    end
```

---

## Phase 1: Engine Stability, Concurrency & Memory Safety (P0 — Completed)

Focus: Eliminate all potential panic paths, resource exhaustion risks, and GUI thread deadlocks under extreme input workloads.

### 1.1 Concurrency & GPU Texture Lifecycle
- [x] **In-Place Texture Streaming (`tex.set`)**: Prevent texture deallocation panics during concurrent viewport drawing by mutating GPU texture handles in-place via `ImageDelta::full`.
- [x] **Stale Render Job Cancellation**: Implement atomic generation tokens (`Arc<AtomicU64>`) on render requests so worker threads abort abandoned background jobs immediately when sliders move, preventing thread pool saturation and latency queues.
- [x] **Frame Rate Throttling & Power Capping**: Adaptive idle detection (drop UI refresh from 60 FPS to 0 FPS when image and controls are static; trigger redraws purely via input events and worker completion).

### 1.2 Memory Footprint & Large Image Handling
- [x] **Bounded Image Cache**: Prevent Out-Of-Memory (OOM) failures when importing large animated GIFs by enforcing allocation ceilings (300 frame cap, 16 MP limit).
- [x] **Progressive Downsampled Preview**: Generate a fast $512\times 512$ thumbnail pass during active slider scrubbing ($< 2\text{ms}$ latency), switching to full-resolution rendering upon mouse release.
- [x] **Safe Codec Decoders**: Implement strict allocation limits (`max_alloc`) on `image` decoders to prevent decompression bomb exploits on malformed headers.

### 1.3 Property Testing, Fuzzing & Diagnostic Logging
- [x] **Algorithm Fuzzing Suite**: Setup `proptest` targets verifying zero panics across all 63 algorithms on:
  - Empty image buffers ($0\times 0$, $1\times 1$).
  - Ultra-wide aspect ratios ($32768\times 1$).
  - Extreme dynamic ranges and single-color images (all `#000000`, all `#FFFFFF`).
  - Palettes with 1 color, duplicate colors, and extreme 256-color arrays.
- [x] **Deterministic Golden Testing**: Automated visual regression testing comparing output hashes (FNV-1a 64-bit) against fixed golden reference values across Linux, macOS, and Windows.
- [x] **Structured Telemetry & Stage Breakdown**: Record rendering latency per stage (Kernel Dither $\to$ Post-Effects Stack $\to$ Histogram Analysis $\to$ Total Elapsed) with live interactive Telemetry HUD and status bar tooltip inspector.

---

## Phase 2: Professional Color Science & Visual Fidelity (P1 — Completed)

Focus: Solve academic color artifacts, dark haloing, and midtone saturation loss common in naive RGB dithering.

### 2.1 Gamma-Correct Linear Dithering
- [x] **Linear sRGB Blending**: Transfer functions `to_linear_f32` and `from_linear_f32` with gamma-correct perceptual color mapping.
  $$\text{linear} = \begin{cases} \frac{C}{12.92} & C \le 0.04045 \\ \left(\frac{C + 0.055}{1.055}\right)^{2.4} & C > 0.04045 \end{cases}$$

### 2.2 Modern Perceptual Color Spaces
- [x] **OKLab & Oklch Color Difference**: Integrate the Björn Ottosson OKLab perceptual space into `ColorMetric`:
  - Eliminates the blue-to-purple hue shifts found in CIELAB 1976.
  - Enables true perceptual lightness ($L$), chroma ($C$), and hue ($h$) independent thresholding.
- [x] **CIEDE2000 ($\Delta E_{00}$)**: Implement the complete ISO/CIE standard color difference equation with hue rotation terms ($R_T$) and parametric compensation factors ($k_L, k_C, k_H$).

### 2.3 Palette Diagnostics & Statistical Analysis
- [x] **Color Histogram & Utilization Meter**: Real-time bar chart showing exact pixel percentage allocated to each palette swatch in the active frame.
- [x] **Underutilized Swatch Pruning**: Auto-detect swatches receiving $< 0.1\%$ allocation with a single-click "Prune Dead Colors" optimizer.
- [x] **Live Interactive Swatch Editing**: Interactive UI allowing users to customize swatch colors with live color pickers, lock key anchor colors, and dynamically add/remove swatches directly in the palette drawer.
- [x] **Multi-Format Palette Exporter & ACT Support**: Export and import active palettes across `.gpl` (GIMP), `.hex`, `.pal` (JASC), `.act` (Color Table), and `.json` in Desktop UI and CLI.
- [x] **Accelerated Exact-Match Swatch Lookup**: Sub-microsecond fast path for quantized pixel histogram categorization bypassing expensive color space distance calculations.

---

## Phase 3: Studio Workflow, Canvas Ergonomics & Utility (P1 — Completed)

Focus: Elevate daily user workflow to parity with professional design workstations.

### 3.1 Advanced Interactive Canvas
- [x] **Infinite Pan & Smooth Zoom**:
  - Pan via Middle-Mouse-Drag, Right-Drag, or `Space + Left-Drag`.
  - Continuous zoom centered on cursor from $5\%$ to $3200\%$ with integer pixel-snap toggle.
- [x] **Pixel Grid Overlay**: Crisp 1-pixel hairline grid rendered automatically at zoom levels $\ge 800\%$ for precision subpixel inspection.
- [x] **Floating Viewport HUD**: Quick-access controls for 1:1 scale, Fit-to-screen, zoom percentage, and grid toggling.
- [x] **Multi-Mode Split View**:
  - Horizontal & Vertical split comparison slider.
  - Checkerboard A/B comparison.
  - Difference Heatmap mode highlighting exact delta pixels between source and dithered output.

### 3.2 Studio History, Presets & Batch Processing
- [x] **Infinite Undo / Redo (`Ctrl+Z` / `Ctrl+Shift+Z`)**: Command-pattern undo stack capturing algorithm changes, palette selections, post-effect parameters, and crop adjustments.
- [x] **Preset Management System (`.dcpreset`)**:
  - One-click export and import of complete workstation configurations (algorithm + custom palette + CRT parameters + temporal noise settings).
  - Bundled user preset library with categories: *Game Boy Vintage*, *Cyberpunk Neon*, *1980s VHS*, *Thermal Camera*, *Newspaper Halftone*.
- [x] **Desktop Batch Processing Queue**:
  - Dedicated Batch Studio panel accepting multi-file folder drops and directories.
  - Parallel background worker dispatch with live progress telemetry.
- [x] **Workstation Ergonomics**:
  - Drag-and-drop file ingestion across all supported image and video formats.
  - Recent files tracking with "Open Recent" submenu.
  - Suggested smart filename generation (`{stem}_{algo}_{palette}.{ext}`).
  - Downstream effects randomizer ("🎲 Shuffle").

---

## Phase 4: Vector Shaper & CNC / Plotter Toolpaths (P2 — Completed)

Focus: Transform DitherCraft into the premier vector pre-processor for pen plotters, laser cutters, vinyl cutting, and screen printing.

### 4.1 Contour Simplification & Curve Fitting
- [x] **Ramer-Douglas-Peucker (RDP) Simplification**: Reduce collinear SVG line points by $70\text{--}90\%$ while preserving sharp corners, featuring a real-time tolerance slider ($\epsilon$).
- [x] **Visvalingam-Whyatt Area-Based Simplification**: Alternative curve simplifier preserving subtle organic contours without creating angular artifacts.
- [x] **Piecewise Cubic Bézier Fitting**: Fit piecewise cubic Bézier curves (`<path d="M... C...">`) over pixel stairsteps for ultra-smooth physical pen plotter strokes.

### 4.2 Multi-Color Separations & G-Code Toolpaths
- [x] **Color Separation Layers (SVG)**: Output multi-layer SVGs with one `<g id="color-name">` per palette swatch, tailored for multi-pen AxiDraw plotters and multi-screen silkscreen printing.
- [x] **Direct G-Code Exporter**:
  - Generate clean G-code (`G00` pen up / travel, `G01` pen down / cut) with configurable feed rates, spindle speed / laser power, and pen-lift delays.
  - **Traveling Salesperson (TSP) Toolpath Optimization**: Nearest-neighbor path sorting minimizing rapid pen-up travel distance by up to $60\%$.

---

## Phase 5: Hardware-Accelerated GPU Compute (WGSL) (P2 — In Progress)

Focus: Sub-millisecond real-time processing for 4K and 8K ultra-high-resolution canvases.

### 5.1 WGPU Compute Pipeline
- [x] **Point-Wise Compute Kernels (WGSL)**:
  - Port ordered dithering (Bayer $8\times 8$ matrix) to parallel GPU compute shaders (`BAYER_ORDERED_COMPUTE_WGSL`).
  - Port rotated screen halftones and frequency modulation masks to compute passes (`HALFTONE_SCREEN_COMPUTE_WGSL`).
- [x] **Real-Time CRT & Post-Process Fragment Shader**: Move bloom glow, barrel lens curvature, scanlines, and chromatic aberration from CPU loops to a unified GPU post-processing compute pass (`CRT_POSTPROCESS_WGSL`).
- [ ] **Zero-Copy Surface Rendering**: Bind compute shader storage buffers directly to egui texture sampler views, bypassing CPU host readback.

---

## Phase 6: Multimedia, Audio-DSP & Video Containers (P3 — Completed)

Focus: Dynamic multimedia creation, video container processing, and music-synchronized graphics.

### 6.1 Native Video Container Demuxing & Modern Animated Exporters
- [x] **Pure-Rust Container Ingestion (MP4 & WebM)**:
  - Implemented pure-Rust ISO-BMFF MP4 demuxer (`Mp4Demuxer`) in `crates/dither-core/src/video.rs` parsing video tracks, nominal FPS, and sample tables without system FFmpeg dependencies.
  - Implemented pure-Rust EBML/WebM demuxer (`WebmDemuxer`) in `crates/dither-core/src/video.rs` parsing tracks, VP8/VP9/AV1 codecs, and Cluster SimpleBlocks.
- [x] **APNG & Animated WebP Exporters**:
  - Output modern, high-efficiency animated formats with full 24-bit color depth alongside legacy GIF via `png` chunk assembly in Desktop UI and CLI.
  - Implemented pure-Rust Animated WebP container muxer (`WebpAnimationMuxer`) assembling `VP8X`, `ANIM`, and `ANMF` chunks with lossless compression in Desktop UI and CLI.

### 6.2 Audio-Reactive DSP Dithering
- [x] **Audio Spectrum Analyzer**: Ingest `.wav` audio tracks with pure-Rust RIFF WAV decoder, perform multi-band frequency analysis (bass, mid, treble, RMS) to drive:
  - Dither threshold modulation (pulsing to bass kicks).
  - CRT scanline jitter (reacting to snare transients).
  - Glitch core displacement (triggering on sub-bass drops).
- [x] **Synchronized Audio Slicing & Export**: Export synchronized WAV audio slices (`AudioTrack::export_wav_slice`) matched to animation frame count and target FPS in Desktop UI and CLI.

---

## Phase 7: Ultra-Performance Engine & SIMD Kernels (P1 — Target v1.0.0)

Focus: Push pixel-throughput past 1 Gigapixel/sec on modern multi-core x86_64 and AArch64 hardware.

### 7.1 Portable SIMD Color Distance Acceleration
- [ ] **AVX2 / NEON Batch Quantization**:
  - Evaluate 8 (AVX2) or 16 (AVX-512) pixels against active palette color swatches in parallel using packed SIMD vector lanes.
  - Vectorize Euclidean RGB and OKLab Euclidean distances (`L`, `a`, `b` differences) without per-pixel branch divergence.
  - Benchmark target: $< 1.5\text{ms}$ for full 4K UHD ($3840\times 2160$) palette mapping on 8-core CPUs.

### 7.2 Tile-Based Parallel Wavefront Error Diffusion
- [ ] **Diagonal Wavefront Multi-Threading**:
  - Overcome the inherently serial row-by-row dependency of error diffusion (Floyd-Steinberg, Atkinson, Stucki, JJN).
  - Partition images into diagonal wavefront bands or stepped tiles, dispatching concurrent Rayon worker threads along independent wavefront lines ($x + y = k$).
  - Eliminate the single-core bottleneck for large video frames and high-resolution print assets.

### 7.3 Cache-Oblivious Space-Filling Curves (Hilbert & Morton / Z-Order)
- [ ] **Fractal Space-Filling Traversal**:
  - Implement recursive Hilbert and Morton (Z-Order) curve scanning passes as alternatives to traditional raster and serpentine sweeps.
  - Maintain optimal spatial locality in CPU L1/L2 data caches, minimizing cache misses during error propagation.
  - Drastically reduce directional "worm" artifacts and asymmetric banding patterns in low-gradient imagery.

### 7.4 Zero-Allocation Ring Buffers & In-Place Frame Pools
- [ ] **Reusable ImageBuffer Arena**:
  - Introduce an `ImageBufferPool` using pre-allocated memory slabs for animation scrubbing and video batch processing.
  - Eliminate repetitive heap allocations (`malloc`/`free`) and memory fragmentation in high-FPS timeline loops.

---

## Phase 8: Studio-Grade Professional UI/UX & Creative Workstation (P1 — Target v1.0.0)

Focus: Modernize the desktop workstation into an intuitive, visually polished, studio-grade creative tool matching premier graphics software.

### 8.1 Interactive Tone Curves & Levels Histogram Editor
- [ ] **Dual-Tone Curves Panel**:
  - Interactive cubic spline curve editor for Master Luminance, Red, Green, and Blue channels.
  - Real-time overlay of pre-dither input histogram and post-curve output distribution.
  - Black point, White point, and Midtone Gamma ($0.1\text{--}3.0$) draggable pins with numerical readouts.

### 8.2 Draggable & Reorderable Downstream Effects Stack
- [ ] **Node/Layer Effect Manager**:
  - Visual effect stack where users can drag and reorder post-effects with instant preview feedback (e.g., Bloom $\to$ Glitch $\to$ CRT vs. Glitch $\to$ CRT $\to$ Bloom).
  - Per-effect blend modes (Normal, Multiply, Screen, Overlay, Color Dodge).
  - Individual effect bypass toggles, duplicate layer buttons, and opacity sliders ($0\%\text{--}100\%$).

### 8.3 Floating Precision Magnifier Loupe & Subpixel Inspector
- [ ] **Interactive Loupe Tool**:
  - Floating circular/square magnifying lens ($400\%$, $800\%$, $1600\%$) tethered to the cursor or pinned to specific canvas coordinates.
  - Real-time subpixel inspection showing individual pixel color swatch index, RGB values, and HEX code.
  - Dual-window side-by-side comparison mode for simultaneous detail inspection.

### 8.4 Color Harmony Generator & Interactive OKLCH Color Wheel
- [ ] **Intelligent Palette Harmonizer**:
  - Interactive circular OKLCH / HSV color wheel in the Palette Drawer.
  - One-click harmony generation based on classical color theory: Complementary, Triadic, Analogous, Split-Complementary, and Tetradic harmonies.
  - Dynamic palette quantization remapping source harmonies into retro bit-depth constraints (1-bit, 2-bit, 4-bit, 8-bit).

### 8.5 Instant A/B Preset Snapshot Comparator
- [ ] **Quick Snapshot Slots (Slots A/B/C/D)**:
  - Instantaneous snapshot capture buttons and keyboard accelerators (`F5` through `F8`).
  - Seamless one-click A/B toggling to evaluate alternative dithering modes or threshold parameters without losing current workstation state.

### 8.6 Professional Workspace Ergonomics & Keyboard HUD
- [ ] **Dockable / Collapsible Studio Layouts**:
  - Left panel: Tool selection, color curves, palette drawer.
  - Right panel: Effect stack, algorithm parameters, temporal noise, and export options.
  - Bottom panel: Video timeline, audio waveform, and playback transport.
  - Comprehensive Keyboard Shortcut Cheatsheet modal (`?` / `F1`) displaying all accelerators, mouse wheel increments, and viewport navigation controls.

---

## Phase 9: Creative DSP, Generative Masks & Production (P2 — Target v1.1.0)

Focus: Expand creative versatility for artists, printmakers, and multimedia designers.

### 9.1 Selective Dithering & Spatial Alpha Masking
- [ ] **Region-Selective Processing**:
  - Brush-based paint mask and gradient alpha masks allowing users to isolate specific image areas for dithering.
  - Luminance range masking (e.g., dither only dark shadows or midtones while keeping skin tones smooth or untouched).
  - Feathered edge blending between dithered textures and pristine source imagery.

### 9.2 Custom Threshold Matrix Designer
- [ ] **Interactive Matrix Workshop**:
  - In-app $N \times N$ matrix editor ($2\times 2$ up to $16\times 16$) enabling artists to paint, import, and test custom ordered dithering patterns.
  - Export and import custom matrices as JSON or PNG bitmap patterns (e.g., custom typography stamps, logos, geometric crosshatching).

### 9.3 3D LUT (Look-Up Table) Vintage Film Emulation
- [ ] **Pure-Rust 3D LUT Engine**:
  - Zero-dependency parser and trilinear interpolator for standard `.cube` (17×17×17 and 33×33×33) 3D Color Look-Up Tables.
  - Bundled vintage film profiles: 1970s Instant Film, Early Technicolor, Kodachrome 64, and Vintage Arcade Monitor phosphors.

### 9.4 Production Batch Automation & Export Queue
- [ ] **Batch Production Studio Upgrades**:
  - Multi-target simultaneous export: save PNG, WebP, SVG, and WAV in a single batch pass.
  - Customizable naming templates: `{stem}_{algo}_{palette}_{w}x{h}_{timestamp}`.
  - Automated watermark placement and EXIF/metadata management for web and archive distribution.

---

## Implementation Matrix & Quality Gates

| Milestone | Key Deliverable | Quality Metric / Success Gate | Status |
|---|---|---|---|
| **Phase 1** | Stability & Memory Safety | Zero panics under `cargo-fuzz`; 0% thread deadlocks; memory capped $< 512\text{ MB}$. | **COMPLETED** |
| **Phase 2** | Color Science & OKLab | Delta E $\le 1.0$ perceptual accuracy; gamma-correct linear midtones verified. | **COMPLETED** |
| **Phase 3** | Studio Canvas & Presets | 60 FPS viewport navigation; full undo/redo stack; `.dcpreset` portability. | **COMPLETED** |
| **Phase 4** | Vector Shaper & CNC | $\ge 75\%$ SVG node reduction via RDP; valid AxiDraw-compliant multi-color SVGs and G-code. | **COMPLETED** |
| **Phase 5** | WGSL GPU Compute | $4\text{K}$ ordered dither latency $< 4\text{ms}$; zero CPU host round-trips. | **IN PROGRESS** |
| **Phase 6** | Audio-Reactive & Video | Direct MP4/WebM import without external C dependencies; audio-synced rendering. | **COMPLETED** |
| **Phase 7** | SIMD & Tile Wavefront | $4\text{K}$ full dither $< 2\text{ms}$; $> 1\text{ Gigapixel/sec}$ batch throughput on multi-core CPU. | **PLANNED** |
| **Phase 8** | Studio UI/UX & Curves | Drag-and-drop effect reordering; interactive tone curves; floating loupe; A/B snapshots. | **PLANNED** |
| **Phase 9** | Masks, Custom Matrix & LUT | Selective spatial alpha masking; custom matrix builder; pure-Rust `.cube` 3D LUT engine. | **PLANNED** |

---

## Clean-Room Architecture Invariants

1. **Layering Independence**: `crates/dither-core` must **never** depend on `eframe`, `egui`, `wgpu`, or windowing crates. It remains a pure mathematical DSP engine callable in headless environments.
2. **Deterministic Outputs**: Given the same input buffer, seed, algorithm, and palette, the engine must produce bit-exact identical output buffers across all CPU architectures (x86_64, aarch64).
3. **No-Panic Guarantee**: All fallible paths must return `Result<T, DitherError>`. Shipped crates strictly deny `unwrap()`, `expect()`, `panic!`, `todo!`, and `unimplemented!`.
4. **Binary Footprint Budget**: Standalone production GUI binary $< 20\text{ MB}$; CLI $< 5\text{ MB}$.
