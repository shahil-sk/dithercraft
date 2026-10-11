# DitherCraft — Comprehensive Engineering Roadmap & Stability Architecture

This roadmap establishes the strategic engineering vision for **DitherCraft** to evolve from its core algorithm baseline into a production-grade, zero-crash, high-utility retro graphics workstation and DSP engine in Pure Rust.

---

```mermaid
flowchart TD
    subgraph S1["Phase 1: Engine Stability & Memory Safety"]
        A1["In-Place Texture Streaming"] --> A2["Render Task Cancellation"]
        A2 --> A3["Bounded Memory Pool"]
        A3 --> A4["Proptest & Fuzz Suite"]
    end

    subgraph S2["Phase 2: Color Science & Visual Fidelity"]
        B1["Gamma-Correct Linear Dithering"] --> B2["OKLab / Oklch Perceptual Space"]
        B2 --> B3["CIEDE2000 ΔE00 Metric"]
        B3 --> B4["Palette Histogram & Analyzer"]
    end

    subgraph S3["Phase 3: Studio UX & Canvas Ergonomics"]
        C1["Infinite Pan & Zoom (Pixel Grid)"] --> C2["Undo / Redo Command History"]
        C2 --> C3["Drag & Drop Batch Studio"]
        C3 --> C4["Live Swatch Drag Reordering"]
    end

    subgraph S4["Phase 4: Vector Shaper & CNC / Plotter Toolpaths"]
        D1["RDP Polygon Simplification"] --> D2["Cubic Bézier Curve Fitting"]
        D2 --> D3["Multi-Color SVG Separations"]
        D3 --> D4["G-Code Toolpath Optimization"]
    end

    subgraph S5["Phase 5: GPU Compute & Realtime Acceleration"]
        E1["WGSL Ordered & Halftone Shaders"] --> E2["Zero-Copy WGPU Texture Pipeline"]
        E2 --> E3["Realtime 4K/60FPS CRT Shader"]
    end

    subgraph S6["Phase 6: Multimedia, Audio-DSP & Video"]
        F1["Pure-Rust Video Demuxer (MP4/WebM)"] --> F2["Audio-Reactive FFT Dither Modulation"]
        F2 --> F3["APNG / Animated WebP Exporters"]
    end

    S1 --> S2 --> S3 --> S4 --> S5 --> S6
```

---

## Phase 1: Engine Stability, Concurrency & Memory Safety (P0 — Immediate)

Focus: Eliminate all potential panic paths, resource exhaustion risks, and GUI thread deadlocks under extreme input workloads.

### 1.1 Concurrency & GPU Texture Lifecycle
- [x] **In-Place Texture Streaming (`tex.set`)**: Prevent texture deallocation panics during concurrent viewport drawing by mutating GPU texture handles in-place via `ImageDelta::full`.
- [ ] **Stale Render Job Cancellation**: Implement atomic generation tokens (`Arc<AtomicU64>`) on render requests so worker threads abort abandoned background jobs immediately when sliders move, preventing thread pool saturation and latency queues.
- [ ] **Frame Rate Throttling & Power Capping**: Introduce adaptive idle detection (drop UI refresh from 60 FPS to 0 FPS when image and controls are static; trigger redraws purely via input events and worker completion).

### 1.2 Memory Footprint & Large Image Handling
- [ ] **Bounded Image Cache**: Prevent Out-Of-Memory (OOM) failures when importing large animated GIFs (100+ frames at 4K) by enforcing a 512 MB LRU disk-backed frame swap or downsampled working proxy buffer.
- [ ] **Progressive Downsampled Preview**: Generate a fast $512\times 512$ thumbnail pass during active slider scrubbing ($< 2\text{ms}$ latency), switching to full-resolution rendering upon mouse release.
- [ ] **Safe Codec Decoders**: Implement strict allocation limits (`max_alloc`) on `image` decoders to prevent zip-bomb or decompression bomb exploits on malformed headers.

### 1.3 Property Testing, Fuzzing & Diagnostic Logging
- [ ] **Algorithm Fuzzing Suite**: Setup `cargo-fuzz` and `proptest` targets verifying zero panics across all 63 algorithms on:
  - Empty image buffers ($0\times 0$, $1\times 1$).
  - Ultra-wide aspect ratios ($32768\times 1$).
  - Extreme dynamic ranges and single-color images (all `#000000`, all `#FFFFFF`).
  - Palettes with 1 color, duplicate colors, and extreme 256-color arrays.
- [ ] **Deterministic Golden Testing**: Automated visual regression testing comparing output hashes against fixed golden reference images across Linux, macOS, and Windows.
- [ ] **Structured Telemetry**: Replace raw console logging with structured `tracing` spans recording rendering latency per stage (color conversion $\to$ spatial indexing $\to$ kernel diffusion $\to$ post-effects).

---

## Phase 2: Professional Color Science & Visual Fidelity (P1)

Focus: Solve academic color artifacts, dark haloing, and midtone saturation loss common in naive RGB dithering.

### 2.1 Gamma-Correct Linear Dithering
- [ ] **Linear sRGB Blending**: Naive dithering in non-linear sRGB (gamma $\approx 2.2$) darkens midtones and causes muddy edges. Implement fast SIMD transfer functions:
  $$\text{linear} = \begin{cases} \frac{C}{12.92} & C \le 0.04045 \\ \left(\frac{C + 0.055}{1.055}\right)^{2.4} & C > 0.04045 \end{cases}$$
  Execute all error distribution and matrix additions in linear luminance space, converting back to sRGB upon palette quantization.

### 2.2 Modern Perceptual Color Spaces
- [ ] **OKLab & Oklch Color Difference**: Integrate the Björn Ottosson OKLab perceptual space into `ColorMetric`:
  - Eliminates the notorious blue-to-purple hue shifts found in CIELAB 1976.
  - Enables true perceptual lightness ($L$), chroma ($C$), and hue ($h$) independent thresholding.
- [ ] **CIEDE2000 ($\Delta E_{00}$)**: Implement the complete ISO/CIE standard color difference equation with hue rotation terms ($R_T$) and parametric compensation factors ($k_L, k_C, k_H$).

### 2.3 Palette Diagnostics & Statistical Analysis
- [ ] **Color Histogram & Utilization Meter**: Real-time bar chart showing exact pixel percentage allocated to each palette swatch in the active frame.
- [ ] **Underutilized Swatch Pruning**: Auto-detect swatches receiving $< 0.1\%$ allocation with a single-click "Prune Dead Colors" optimizer.
- [ ] **Live Swatch Drag-and-Drop**: Interactive UI allowing users to drag swatch positions, lock key anchor colors, and assign custom RGB hex codes directly in the palette drawer.

---

## Phase 3: Studio Workflow, Canvas Ergonomics & Utility (P1)

Focus: Elevate daily user workflow to parity with professional design workstations.

### 3.1 Advanced Interactive Canvas
- [ ] **Infinite Pan & Smooth Zoom**:
  - Pan via Middle-Mouse-Drag or `Space + Left-Drag`.
  - Continuous zoom centered on cursor from $10\%$ to $3200\%$ with integer pixel-snap toggle.
- [ ] **Pixel Grid Overlay**: Crisp 1-pixel hairline grid rendered automatically at zoom levels $\ge 800\%$ for precision subpixel inspection.
- [ ] **Multi-Mode Split View**:
  - Horizontal & Vertical split comparison slider.
  - Checkerboard A/B comparison.
  - Difference Heatmap mode highlighting exact delta pixels between source and dithered output.

### 3.2 Studio History & Presets
- [ ] **Infinite Undo / Redo (`Ctrl+Z` / `Ctrl+Shift+Z`)**: Command-pattern undo stack capturing algorithm changes, palette selections, post-effect parameters, and crop adjustments.
- [ ] **Preset Management System (`.dcpreset`)**:
  - One-click export and import of complete workstation configurations (algorithm + custom palette + CRT parameters + temporal noise settings).
  - Bundled user preset library with categories: *Game Boy Vintage*, *Cyberpunk Neon*, *1980s VHS*, *Thermal Camera*, *Newspaper Halftone*.
- [ ] **Drag-and-Drop Batch Processing Queue**:
  - Dedicated Batch Studio panel accepting multi-file folder drops.
  - Parallel background worker dispatch with live progress bars and thumbnail before/after previews.

---

## Phase 4: Vector Shaper & CNC / Plotter Toolpaths (P2)

Focus: Transform DitherCraft into the premier vector pre-processor for pen plotters, laser cutters, vinyl cutting, and screen printing.

### 4.1 Contour Simplification & Curve Fitting
- [ ] **Ramer-Douglas-Peucker (RDP) Simplification**: Reduce collinear SVG line points by $70\text{--}90\%$ while preserving sharp corners, featuring a real-time tolerance slider ($\epsilon$).
- [ ] **Visvalingam-Whyatt Area-Based Simplification**: Alternative curve simplifier preserving subtle organic contours without creating angular artifacts.
- [ ] **Schneider's Cubic Bézier Fitting**: Fit piecewise cubic Bézier curves (`<path d="M... C...">`) over pixel stairsteps for ultra-smooth physical pen plotter strokes.

### 4.2 Multi-Color Separations & G-Code Toolpaths
- [ ] **Color Separation Layers (SVG)**: Output multi-layer SVGs with one `<g id="color-name">` per palette swatch, tailored for multi-pen AxiDraw plotters and multi-screen silkscreen printing.
- [ ] **Direct G-Code Exporter**:
  - Generate clean G-code (`G00` pen up / travel, `G01` pen down / cut) with configurable feed rates, spindle speed / laser power, and pen-lift delays.
  - **Traveling Salesperson (TSP) Toolpath Optimization**: Nearest-neighbor path sorting minimizing rapid pen-up travel distance by up to $60\%$.

---

## Phase 5: Hardware-Accelerated GPU Compute (WGSL) (P2)

Focus: Sub-millisecond real-time processing for 4K and 8K ultra-high-resolution canvases.

### 5.1 WGPU Compute Pipeline
- [ ] **Point-Wise Compute Kernels (WGSL)**:
  - Port ordered dithering (Bayer $2\times 2$ to $16\times 16$, Clustered Dot, Spiral, Interleaved Gradient Noise) to parallel GPU compute shaders.
  - Port rotated screen halftones and blue noise modulation masks to compute passes.
  - Achieve $> 240\text{ FPS}$ throughput on $3840\times 2160$ displays.
- [ ] **Zero-Copy Surface Rendering**: Bind compute shader storage buffers directly to egui texture sampler views, bypassing CPU host readback.
- [ ] **Real-Time CRT & Post-Process Fragment Shader**: Move bloom glow, barrel lens curvature, scanlines, and chromatic aberration from CPU loops to a unified GPU post-processing fragment pass.

---

## Phase 6: Multimedia, Audio-DSP & Video Container Pipeline (P3)

Focus: Dynamic multimedia creation, video container processing, and music-synchronized graphics.

### 6.1 Native Video Container Demuxing
- [ ] **Pure-Rust Container Ingestion**: Integrate a pure-Rust MP4 and WebM demuxer (e.g. `mp4` / `webm` crates) to extract video frame sequences without requiring system FFmpeg installations.
- [ ] **APNG & Animated WebP Exporters**: Output modern, high-efficiency animated formats with full 24-bit color depth alongside legacy GIF.

### 6.2 Audio-Reactive DSP Dithering
- [ ] **Audio Spectrum Analyzer**: Ingest `.wav` / `.mp3` audio tracks, perform real-time Fast Fourier Transform (FFT) analysis, and map low/mid/high frequency bands to:
  - Dither threshold modulation (pulsing to bass kicks).
  - CRT scanline jitter (reacting to snare transients).
  - Glitch core displacement (triggering on sub-bass drops).
- [ ] **Audio-Synchronized Video Export**: Export final animated video renders muxed with the original soundtrack.

---

## Implementation Matrix & Quality Gates

| Milestone | Key Deliverable | Quality Metric / Success Gate |
|---|---|---|
| **Phase 1** | Stability & Memory Safety | Zero panics under `cargo-fuzz`; 0% thread deadlocks; memory capped $< 512\text{ MB}$. |
| **Phase 2** | Color Science & OKLab | Delta E $\le 1.0$ perceptual accuracy; gamma-correct linear midtones verified. |
| **Phase 3** | Studio Canvas & Presets | 60 FPS viewport navigation; full undo/redo stack; `.dcpreset` portability. |
| **Phase 4** | Vector Shaper & CNC | $\ge 75\%$ SVG node reduction via RDP; valid AxiDraw-compliant multi-color SVGs and G-code. |
| **Phase 5** | WGSL GPU Compute | $4\text{K}$ ordered dither latency $< 4\text{ms}$; zero CPU host round-trips. |
| **Phase 6** | Audio-Reactive & Video | Direct MP4/WebM import without external C dependencies; audio-synced rendering. |

---

## Clean-Room Architecture Invariants

1. **Layering Independence**: `crates/dither-core` must **never** depend on `eframe`, `egui`, `wgpu`, or windowing crates. It remains a pure mathematical DSP engine callable in headless environments.
2. **Deterministic Outputs**: Given the same input buffer, seed, algorithm, and palette, the engine must produce bit-exact identical output buffers across all CPU architectures (x86_64, aarch64).
3. **No-Panic Guarantee**: All fallible paths must return `Result<T, DitherError>`. Shipped crates strictly deny `unwrap()`, `expect()`, `panic!`, `todo!`, and `unimplemented!`.
4. **Binary Footprint Budget**: Standalone production GUI binary $< 20\text{ MB}$; CLI $< 5\text{ MB}$.
