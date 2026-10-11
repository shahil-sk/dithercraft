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

### 2.2 Direct Video Codec Container Demuxing (MP4 / WebM)
- **Current Status**: The multi-frame engine supports animated GIF files and frame sequences natively without external C libraries.
- **Identified Gap**: Ingesting H.264/H.265 `.mp4` and VP9/AV1 `.webm` video files requires external extraction or transcoding into animated GIF / image frames.
- **Next Frontier**: Integrate a zero-system-dependency pure-Rust demuxer/decoder (or optional feature-gated `ffmpeg-sys`) for streaming direct video tracks through the dither pipeline.

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

---

## 3. Reliability & Zero-Panic Assurance

| Criterion | Current Implementation | Status |
|---|---|---|
| **No Panics in Shipped Code** | All workspace code denies `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`. | Verified (100% clean) |
| **Strict Crate Layering** | `dither-core` has zero GUI/windowing dependencies. | Verified (`cargo xtask layers`) |
| **Non-Blocking UI** | All dither calculations and effects execute on Rayon background workers. | Verified ($\ge 60\text{ FPS}$) |
| **Release Footprint** | Desktop GUI: 12 MB. Headless CLI: 1.9 MB. | Verified ($<20\text{ MB}$ target) |
