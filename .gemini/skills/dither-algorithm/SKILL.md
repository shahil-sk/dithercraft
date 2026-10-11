---
name: dither-algorithm
description: >-
  Guide for implementing, optimizing, and debugging Pure Rust dithering algorithms, kernels, and palettes.
  Use when adding new dithering methods, modifying error diffusion matrices, optimizing SIMD kernels, or tuning Rayon parallelism.
---

# Dither Guy (Rust) — Dithering Algorithms Guide

Guidelines and mathematical conventions for algorithms implemented in `crates/dither-core`.

## Core Dithering Categories

### 1. Ordered Dithering
- **Mechanism**: Adds a threshold matrix (e.g. Bayer matrix $M_N$) to pixel brightness values before quantizing.
- **Complexity**: $O(W \times H)$, embarrassingly parallel across CPU cores using `rayon::iter::ParallelIterator`.
- **Dynamic Bayer Matrix Generation**:
  $$M_{2N} = \begin{bmatrix} 4M_N & 4M_N + 2 \\ 4M_N + 3 & 4M_N + 1 \end{bmatrix}$$
- **Zero-Allocation**: Map over pre-allocated image buffer rows without intermediate allocations.

### 2. Error Diffusion
- **Mechanism**: Quantizes the current pixel, computes residual error $e = \text{original} - \text{quantized}$, and diffuses fractions of $e$ to neighboring unprocessed pixels using a kernel matrix.
- **Supported Kernels**:
  - **Floyd-Steinberg**: $7/16, 3/16, 5/16, 1/16$.
  - **Atkinson**: $1/8$ diffused to 6 neighboring pixels with $2/8$ deliberate error loss (high-contrast Macintosh 1-bit retro style).
  - **Burkes, Sierra, Sierra-Lite, Stucki, Jarvis-Judice-Ninke (JJN)**.
  - **Serpentine Scanning**: Alternating bidirectional scanning per row (even left-to-right, odd right-to-left) to suppress directional artifacts.
  - **Riemersma Dithering**: Hilbert space-filling curve with exponential error memory.
- **Performance**: Pre-allocate a 2-row or 3-row sliding error scratchpad (`Vec<f32>` reused across frames). Avoid per-pixel allocations.

## Palette Quantization & Color Models

- **Color Distance**:
  - Euclidean RGB: $\Delta = (r_1-r_2)^2 + (g_1-g_2)^2 + (b_1-b_2)^2$.
  - Perceptual Redmean approximation.
  - CIELAB $\Delta E^*$.
- **Fast Nearest-Neighbor Search**:
  - Use spatial k-d tree or octree to look up nearest palette colors in $O(\log K)$ time.

## Rust Implementation Best Practices

1. **Pure Decoupled Math**:
   - `crates/dither-core` must NEVER import `egui`, `eframe`, `wgpu`, or windowing crates.
   - Accept byte slices `&[u8]`, mutable buffers `&mut [u8]`, and scalar configurations.
2. **Bounds Safety**:
   - Use `.get_mut()` or clamp indices when distributing errors near image boundaries.
3. **No Panics**:
   - Functions return `Result<T, DitherError>`.
