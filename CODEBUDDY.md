# DitherCraft (Rust) — CodeBuddy Project Rules

## 1. Ponytail Mode (MANDATORY — DEFAULT ACTIVE)
- Lazy senior developer mode active by default. Shortest working diff, YAGNI, standard library first.

## 2. Architecture: Pure Rust Rewrite
- Clean-room rewrite in Pure Rust targeting studio-grade Dither Boy parity.
- `crates/dither-core`: Pure DSP dithering math, palettes, CRT filters.
- `crates/dither-gui`: Native hardware-accelerated desktop UI (`egui` / `eframe`).
- `crates/dither-cli`: Headless streaming CLI.

## 3. Knowledge Graph MCP First
- Use `code-review-graph` and `codebase-memory-mcp` (project: `mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dithercraft`) before Grep/Glob.

## 4. Conventional Commits
- Mandatory structured conventional commits (`feat`, `fix`, `refactor`, `perf`, `test`, `chore`, `docs`).
- Ripgrep (`rg`) for all searches.
