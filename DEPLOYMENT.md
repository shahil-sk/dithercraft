# DitherCraft (Rust) — Deployment & Packaging Guide (DEPLOYMENT.md)

This document specifies the compilation, optimization, and packaging architecture for the pure Rust implementation of DitherCraft.

---

## 1. Cargo Optimization Profiles

Production binaries are built with Link-Time Optimization (LTO) and binary stripping configured in [`Cargo.toml`](file:///mnt/5893accc-4b16-4de7-b2a7-6333530b2614/PROJECTS/21_Active/dithercraft/Cargo.toml):

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### Profile Benefits:
- **Maximized Inlining**: Inter-crate optimizations between `dither-core` and `dither-gui`.
- **Minimal Binary Footprint**: Redundant symbols, panic unwinding tables, and dead code are stripped ($< 20\text{ MB}$ standalone binary).
- **Peak Vectorization**: LLVM generates optimal AVX2/NEON vector instructions.

---

## 2. Multi-Platform Build Targets

| OS / Architecture | Target Triple | Artifact Output | Package Format |
| :--- | :--- | :--- | :--- |
| **Windows x86_64** | `x86_64-pc-windows-msvc` | `dither-gui.exe` | Standalone `.exe` & `.zip` |
| **Linux x86_64** | `x86_64-unknown-linux-gnu` | `dither-gui` | Standalone binary & `.tar.gz` |
| **macOS Apple Silicon** | `aarch64-apple-darwin` | `DitherGuy.app` | `.dmg` |
| **macOS Intel** | `x86_64-apple-darwin` | `DitherGuy.app` | `.dmg` |

---

## 3. GitHub Actions CI/CD Release Pipeline

Continuous deployment is orchestrated via [`.github/workflows/release.yml`](file:///mnt/5893accc-4b16-4de7-b2a7-6333530b2614/PROJECTS/21_Active/dithercraft/.github/workflows/release.yml).

### Trigger Conditions:
- Pushing a Git tag matching `v*` (e.g. `v6.0.0`).
- Manual trigger via `workflow_dispatch`.

### Automated Release Steps:
1. Matrix build compiles native binaries across `windows-latest`, `ubuntu-latest`, and `macos-latest`.
2. SHA-256 checksums are generated and validated.
3. A GitHub Release is created automatically with release notes and binary attachments.
