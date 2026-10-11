# DitherCraft (Rust) — Parity & Academic Literature Matrix (`docs/parity.md`)

This document details the functional, mathematical, and algorithmic parity of **DitherCraft** against vintage DSP literature, reference software, and academic standards.

DitherCraft is built as a 100% clean-room, Pure Rust desktop workstation and DSP engine with zero foreign binary dependencies or proprietary shaders.

---

## 1. Academic DSP Grounding & Citations

All algorithms implemented in `crates/dither-core` adhere to public scientific publications:

| Author & Year | Publication / Origin | Algorithm Implementation |
|---|---|---|
| **Bayer (1973)** | *An Optimum Method for Two-Level Rendition of Continuous-Tone Pictures* | Dynamic recursive Bayer matrices ($2\times 2$ to $16\times 16$) |
| **Floyd & Steinberg (1976)** | *An Adaptive Algorithm for Spatial Gray Scale* | Classic error diffusion ($7/16, 3/16, 5/16, 1/16$) with subpixel residual accumulation |
| **Heckbert (1982)** | *Color Image Quantization for Frame Buffer Display* | Median-Cut native palette extraction |
| **Atkinson (1984)** | Apple Computer Bill Atkinson Dither Kernel | Apple Macintosh 1-bit diffusion retaining $6/8$ error ($1/8$ per neighbor) |
| **Stevenson & Arce (1985)** | *Binary Display of Hexagonal Continuous-Tone Halftones* | 12-neighbor hexagonal diffusion kernel |
| **Niblack (1986)** | *An Introduction to Digital Image Processing* | Local adaptive binarization ($m + k \cdot s$) with integral-image acceleration |
| **Knuth (1987)** | *Digital Halftones by Dot Diffusion* | Dot-diffusion priority grid matrix |
| **Ulichney (1987, 1993)** | *Digital Halftoning* & *Void-and-Cluster Method* | Blue noise void-and-cluster screening matrix |
| **Otsu (1979)** | *A Threshold Selection Method from Gray-Level Histograms* | Global inter-class variance maximization threshold |
| **Fan (1993)** | *A Simple Modification to Error Diffusion Weights* | Error diffusion edge enhancement |
| **Shiau & Fan (1996)** | *A Set of Easily Implementable Coefficients in Error Diffusion* | Set 1 & Set 2 rational power-of-two diffusion kernels |
| **Riemersma (1998)** | *A Balanced Dithering Technique with a Space-Filling Curve* | Hilbert fractal curve traversal with exponential decay queue |
| **Sauvola & Pietikäinen (2000)**| *Adaptive Document Image Binarization* | Contrast-normalized local adaptive binarization |
| **Ostromoukhov (2001)** | *A Simple and Efficient Error-Diffusion Algorithm* | Dynamic tonal-dependent blue-noise coefficient interpolation |
| **Roberts (2014)** | *The Unreasonable Effectiveness of Quasirandom Sequences* | Low-discrepancy quasirandom sequence modulation ($R_2$) |

---

## 2. Dithering Engine Parity (63 Algorithms)

DitherCraft achieves parity across **6 mathematical categories**:

### 2.1 Error-Diffusion (17 Algorithms)
- **Classic**: Floyd-Steinberg, Atkinson, Sierra-3, Sierra-2, Sierra-Lite.
- **Extended Stencils**: Jarvis-Judice-Ninke (JJN), Stucki, Burkes, Stevenson-Arce (1985).
- **Academic Variable & Blue Noise**: Ostromoukhov (2001, 256-level dynamic tables), Minimized Average Error (1992), Fan (1993), Shiau-Fan 1 & 2 (1996), Variable-Error (1991).
- **Space-Filling & Grid Curves**: Dot-Diffusion (Knuth 1987), Riemersma (Hilbert fractal curve with 16-element error queue).

### 2.2 Ordered Dithering (13 Algorithms)
- Dynamic Bayer matrices: $2\times 2$, $4\times 4$, $8\times 8$, $16\times 16$.
- Clustered Dot ($4\times 4$, $8\times 8$), Central White Point, Dispersed Dot, Spiral.
- Structural Line & Hatch: Horizontal Line, Vertical Line, Diagonal Line, Checkerboard 45°.
- Modern Screen: Interleaved Gradient Noise (IGN).

### 2.3 Bitmap Conversion (7 Algorithms)
- Fixed Threshold, Adaptive Otsu Threshold, Mean Threshold.
- Integral-Image Accelerated ($O(1)$ per pixel): Gaussian Local Threshold, Sauvola Binarization, Niblack Binarization.
- Multi-Level Step Quantization.

### 2.4 Halftone Arrays (7 Algorithms)
- Rotated Screens: $0^\circ, 15^\circ, 45^\circ, 75^\circ$.
- Dot Morphologies: Circular Screen, Diamond Dot, Elliptical Dot, Line Screen, Crosshatch Halftone.

### 2.5 Modulation Systems (6 Algorithms)
- Frequency Modulation (FM / Blue Noise Roberts $R_2$).
- Amplitude Modulation (AM / Screen Arrays).
- Stochastic Screening, Void-and-Cluster Masking (Ulichney 1993).
- Uniform Random Noise, Triangular HP-TPDF Noise (1989).

### 2.6 Artistic Dispersion & Stylization (6 Algorithms)
- Mezzotint, Stippling, Woodcut Simulation, Newsprint Grain, Etching Crosshatch, Matrix Glitch Dispersion.
- Pop-Tone midtone remapping, CMYK four-color process angled separations ($15^\circ, 75^\circ, 0^\circ, 45^\circ$).

---

## 3. Color Science & Quantization Parity

- **Color Distance Metrics**:
  - Euclidean RGB: Standard fast distance.
  - Perceptual Redmean: Perceptually weighted Euclidean RGB distance.
  - CIELAB $\Delta E^*_{ab}$: Perceptual color difference via CIE D65 sRGB $\to$ Linear $\to$ XYZ $\to$ CIE $L^*a^*b^*$ conversion.
- **Fast Nearest Neighbor Lookup**: Balanced 3D spatial KD-Tree (`PaletteKdTree`) providing $O(\log K)$ zero-allocation lookups.
- **Dynamic Palette Extraction**: Native Heckbert Median-Cut (1982) and Lloyd K-Means (1982) clustering.
- **Swatch Controls**: Slot locking, selective unlocked randomization, tonal sorting ($Y = 0.299R + 0.587G + 0.114B$), and luminance cycling sweeps.
- **Multi-Format Interop**: Seamless parsing and serialization for `.json` presets, `.hex`, `.gpl` (GIMP palette), and `.pal` (JASC-PAL).

---

## 4. Stackable Post-Effects Parity

- **Vintage CRT Simulation**: Subpixel scanline raster lines, phosphor bloom glow, spherical barrel curvature lens distortion, radial corner vignette.
- **Authentic JPEG Glitch Core**: 5 distinct glitch regimes:
  1. *Block Shift*: Horizontal dislocation of $8\times 8$ and $16\times 16$ macroblocks.
  2. *Channel Swap*: Transposition of chrominance and luminance planes.
  3. *Scan Line Offset*: Phase tearing across raster lines.
  4. *Block Scramble*: Pseudo-random tile rearrangement.
  5. *Interlace Corruption*: Alternating field dropped frames and comb-filter decay.
- **Spatial Filters**: Separable 1D Gaussian Blur, high-pass Sharpen, directional & radial Chromatic Aberration, Epsilon Bloom Glow.

---

## 5. Animation, Multi-Frame & Video Parity

- **9 CRT & LCD Temporal Noise Patterns**:
  1. CRT Scan Beam Jitter
  2. Phosphor Grain Hum
  3. Interlaced Field Shimmer
  4. LCD Subpixel Grid Drift
  5. VHS Tape Phase Noise
  6. Thermal Analog Drift
  7. Static Hash White Noise
  8. Bayer Phase Stepping
  9. Blue Noise Temporal Jitter
- **Timeline Engine**: Scrub bar, transport controls (`Play`/`Pause`/`Step`), 1..60 FPS target, looping, keyframing (`TimelineTrack`, `Keyframe`, `Interpolation`), and automated LFO modulation.
- **Multi-Frame Animated GIF**: Lossless preservation of frame delay and duration, parallel per-frame rendering across Rayon workers, multi-frame GIF import and export.

---

## 6. Vector Isolation & Plotter Output

- **Contour Tracing**: High-precision boundary tracing extracting darkest pixel contours into optimized SVG `<path>` vectors.
- **Hardware Compatibility**: Ready for laser engravers, vinyl cutters, Cricut, pen plotters, CNC embroidery, and screen printing.

---

## 7. Headless CLI & Model Context Protocol (MCP)

- **Clap CLI**: `single`, `gif`, `svg`, `batch`, `bench`, and `mcp`.
- **Stdio MCP Server**: JSON-RPC 2.0 interface exposing `list_algorithms`, `list_palettes`, `dither_image`, `isolate_vector`, and `extract_palette` for autonomous coding agent workflows.
