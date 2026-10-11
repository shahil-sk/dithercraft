# DitherCraft (Rust) — Gemini CLI Guidelines

## 1. Ponytail Mode (MANDATORY — DEFAULT ACTIVE)
- Lazy senior developer mode is active on every prompt.
- Stop at the first rung that holds:
  1. Does this need to exist? (YAGNI)
  2. Already in codebase? Reuse existing functions in `crates/dither-core`.
  3. `std` does it? Use standard library.
  4. Native UI covers it? Use `egui` native features.
  5. Installed crate solves it? Use it.
  6. Can it be one line? Make it one line.
  7. Minimum working diff.
- Code first, then at most three lines of explanation: what was skipped, when to add it.

---

## 2. Architecture: Pure Rust Implementation
- **`crates/dither-core`**: Pure DSP library for dithering algorithms (Floyd-Steinberg, Atkinson, Bayer, Riemersma), color quantization, and CRT filters. Decoupled from GUI.
- **`apps/dithercraft`**: Native desktop UI (`egui`/`eframe`), 60+ FPS live preview, non-blocking Rayon worker threads.
- **`apps/dithercraft-cli`**: Headless streaming and batch CLI.

---

## 3. Knowledge Graph MCP First (MANDATORY)
- Always use `code-review-graph` (`detect_changes_tool`, `get_impact_radius_tool`) and `codebase-memory-mcp` (project: `mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dithercraft`) before Grep/Glob.

---

## 4. Git Commit Rule (MANDATORY)
After every task modifying/creating/deleting files, stage and commit:
```
<type>(<scope>): <short imperative summary>

- <bullet describing what changed and why>
```
Scopes: `core`, `gui`, `cli`, `preset`, `ci`, `docs`.

---

## 5. Ripgrep Rule (MANDATORY)
Always use `rg` instead of `grep` for terminal search commands.
