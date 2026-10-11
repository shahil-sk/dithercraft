<div align="center">

# DitherCraft
<img src="assets/app-icon/dithercraft-1024.png" width="160" alt="DitherCraft Logo" />

**DitherCraft** is an open-source, studio-grade desktop workstation and DSP engine for retro graphical dithering, vintage CRT simulations, creative glitch effects, and black vector isolation for CNC/vinyl cutters, built in **Pure Rust**.

[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen?style=for-the-badge)](https://github.com/shahil-sk/dithercraft)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-black?style=for-the-badge)](https://github.com/shahil-sk/dithercraft)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue?style=for-the-badge)](LICENSE)

<br>

[Specification](docs/spec.md) •
[Roadmap](docs/roadmap.md) •
[Parity](docs/parity.md) •
[Shaper](docs/shaper.md) •
[Gaps](docs/gaps.md) •
[Workflow](docs/workflow.md) •
[Rules](docs/rules.md) •
[Deployment](docs/deployment.md)

</div>

---

## 1. Overview

**DitherCraft** provides studio-grade digital signal processing (DSP) for retro graphics, ordered and error-diffusion dithering, vintage CRT simulations, compression glitch effects, and precision vector contour tracing.

Engineered with VectorCraft-grade architecture:
- **Pure Decoupled Math**: `crates/dither-core` has zero dependencies on UI, egui, eframe, or windowing libraries.
- **Never Crash**: No panics or unwraps in production code. All fallible routines return `Result<T, DitherError>`.
- **Non-Blocking Multi-Threaded GUI**: Live 60+ FPS preview viewport with computation offloaded to Rayon background workers.
- **Zero-Allocation Hot Loops**: Image processing routines avoid dynamic heap allocations inside per-pixel inner loops.
- **Integrated Tooling**: Built-in `cargo xtask` for packaging, icon packing, layering gates, and CI validation.

---

## 2. Workspace Structure

```text
dithercraft/
├── apps/
│   ├── dithercraft/          # Native desktop workstation (egui / eframe / WGPU)
│   └── dithercraft-cli/      # Headless streaming CLI & batch processing tool
├── crates/
│   └── dither-core/          # Pure DSP math, 63 dithering algorithms, glitch core
├── xtask/                    # Pure Rust build, packaging, and layering tool (`cargo xtask`)
├── packaging/
│   ├── linux/                # Linux .desktop, AppStream metainfo, AppImage & tarball
│   ├── macos/                # macOS .app bundle, Info.plist, DMG packaging
│   └── windows/              # Windows WiX MSI installer & icon packing
├── assets/
│   ├── app-icon/             # Multi-resolution SVG, ICO, and PNG application icons
│   └── icons/                # Clean SVG iconography
└── docs/                     # Engineering specifications & guides
```

---

## 3. Quick Start

### Build and Run Native Desktop Workstation
```bash
cargo run -p dithercraft
```

### Run Headless Benchmark
```bash
cargo run -p dithercraft-cli -- bench
```

### Process an Image via CLI
```bash
cargo run -p dithercraft-cli -- single -i input.png -o output.png --algo floyd-steinberg --palette gameboy
```

### Export Black Vector Paths (SVG) for Vinyl Cutters & CNC
```bash
cargo run -p dithercraft-cli -- svg -i input.png -o output.svg --threshold 128
```

### Run Quality Gates & Layering Verification
```bash
cargo test --workspace
cargo xtask layers
cargo xtask stats
```

---

## 4. Engineering Standards

- [`docs/spec.md`](docs/spec.md) — Clean-room mathematical & functional specification (63 algorithms, 9 temporal CRT/LCD noise patterns, JPEG Glitch Core, 5 themes)
- [`docs/roadmap.md`](docs/roadmap.md) — Phased milestone execution roadmap
- [`docs/parity.md`](docs/parity.md) — Clean-room literature & reference parity matrix
- [`docs/shaper.md`](docs/shaper.md) — Vector contour isolation & SVG plotter exporter
- [`docs/gaps.md`](docs/gaps.md) — Architectural boundary & capability gap analysis
- [`docs/rules.md`](docs/rules.md) — Engineering invariants & Ponytail mode guidelines
- [`docs/workflow.md`](docs/workflow.md) — Development lifecycle & Conventional Commits
- [`docs/deployment.md`](docs/deployment.md) — Multi-platform packaging matrix
- [`docs/versioning.md`](docs/versioning.md) — Semantic versioning and release lifecycle
