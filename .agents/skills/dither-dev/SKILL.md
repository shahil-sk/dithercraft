---
name: dither-dev
description: >-
  Development runbook for Dither Guy Pure Rust implementation: building, running, testing, benchmarking, and packaging.
  Use when compiling the workspace, running cargo commands, testing algorithms, or tuning release profiles.
---

# Dither Guy (Rust) — Development Runbook

Comprehensive engineering runbook for developing, testing, and building Dither Guy in Pure Rust.

## Prerequisites
- Rust stable toolchain (2021/2024 edition).
- Components: `rustfmt`, `clippy`.

## Common Commands

1. **Build Workspace**:
   ```bash
   cargo build --workspace
   ```
2. **Run Desktop Application**:
   ```bash
   cargo run -p dither-gui
   ```
3. **Run Headless CLI**:
   ```bash
   cargo run -p dither-cli -- single input.png output.png --algo floyd-steinberg --palette gameboy
   ```
4. **Quality Gates (MANDATORY before commit)**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```
5. **Micro-benchmarks (Criterion)**:
   ```bash
   cargo bench -p dither-core
   ```

## Production Release Builds

```bash
cargo build --release --workspace
```
Binaries output to `target/release/`:
- `target/release/dither-gui` (interactive desktop application)
- `target/release/dither-cli` (headless batch processor & MCP server)

Production builds enable:
- `opt-level = 3`
- `lto = "fat"`
- `codegen-units = 1`
- `panic = "abort"`
- `strip = true`
