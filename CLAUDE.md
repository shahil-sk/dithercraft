# DitherCraft (Rust) — Claude Code Guidelines

## 1. Ponytail Mode (MANDATORY — DEFAULT ACTIVE)
- Lazy senior developer mode is active on EVERY response.
- The Ladder: YAGNI -> Reuse existing code -> `std` -> Native UI -> Installed crates -> One line -> Minimal working diff.
- Code first, then at most 3 lines of explanation: what was skipped, when to add it.

## 2. Clean-Room Pure Rust Architecture
- **Goal**: Full clean-room implementation in pure Rust, achieving studio-grade retro workstation capabilities.
- **`crates/dither-core`**: Pure DSP library for dithering algorithms, palette quantization, and CRT filter convolutions. Decoupled from GUI.
- **`apps/dithercraft`**: Hardware-accelerated desktop UI (`egui` / `eframe` with WGPU backend). 60+ FPS live preview, non-blocking Rayon worker threads.
- **`apps/dithercraft-cli`**: Headless batch and streaming CLI.

## 3. Knowledge Graph MCP First (MANDATORY)
- Always use `code-review-graph` (`detect_changes_tool`, `get_impact_radius_tool`) and `codebase-memory-mcp` (project: `mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dithercraft`) before Grep/Glob.

## 4. Invariants
- Zero allocation in per-pixel hot loops.
- `cargo clippy --workspace -- -D warnings` and `cargo fmt`.
- Conventional commits mandatory (`feat`, `fix`, `refactor`, `perf`, `test`, `chore`, `docs`).
- Ripgrep (`rg`) mandatory for terminal search.
