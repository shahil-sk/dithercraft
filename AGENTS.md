# DitherCraft (Rust) — Instructions for AI Agents

**DitherCraft** is a clean-room, open-source, studio-grade desktop workstation and DSP engine for retro graphical dithering and vintage CRT effects, undergoing a **full rewrite in Pure Rust**.

Inspired by [VectorCraft](https://github.com/storytold/vectorcraft), the project adheres to strict clean-room engineering, no-panic reliability, command-driven architecture, and automated multi-platform delivery.

---

## 1. Autonomous Operation Protocol

Follow this protocol for every session and task:
1. **Orient**: Consult knowledge graph MCP tools (`code-review-graph`, `codebase-memory-mcp`) and read [`ROADMAP.md`](ROADMAP.md) / [`SPEC.md`](SPEC.md).
2. **Plan (Ponytail Default)**: Channel a lazy senior developer. Select the first rung that holds:
   - Does this need to exist? (YAGNI) $\to$ Reuse existing code $\to$ `std` $\to$ Native UI $\to$ Installed crates $\to$ One line $\to$ Shortest working diff.
3. **Implement**: Decouple math from presentation. Everything is a command.
4. **Verify**: Ensure code passes formatting, clippy without warnings, and tests (`cargo test`).
5. **Review**: Run `detect_changes_tool` and `get_impact_radius_tool`.
6. **Commit**: Mandatory Conventional Git Commit before finalizing (`feat`, `fix`, `perf`, `refactor`, `test`, `chore`, `docs`).

---

## 2. Non-Negotiables & Architectural Invariants

### 2.1 Clean-Room Development (Strict Invariant)
- **Zero Proprietary Code/Assets**: Never disassemble, decompile, or copy binary code, icons, presets, or shaders from Dither Boy or third-party proprietary applications.
- **Academic & Public DSP Literature Only**: All algorithms are implemented strictly from public scientific publications (Bayer 1973, Floyd & Steinberg 1976, Atkinson 1984, Ulichney 1987, Riemersma 1998, Ostromoukhov 2001). Cite sources by author and year.

### 2.2 Never Crash: No Panics in Shipped Code
- **No unhandled aborts**: Files, dropped data, and preset parameters are untrusted inputs.
- Fallible functions return `Result<T, DitherError>` or `Option<T>`. Propagate errors with `?`.
- Workspace clippy bans `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` in non-test code.
- Guard against index out-of-bounds: use `.get()` or clamp coordinates in diffusion passes.
- Engine panic boundary: Top-level command dispatchers catch unexpected panics, revert state, and return a clean error message to the UI.

### 2.3 Everything is a Command
- User-visible behavior is encapsulated as an engine command with an ID, parameter dictionary, undo/redo support, and execution handler.
- Tools and UI panels emit commands (`Begin`, `Preview`, `Commit`).
- Control & Automation: The application exposes an IPC / JSON-RPC control protocol (compatible with Model Context Protocol) allowing headless testing and scripted manipulation.

### 2.4 Strict Crate Layering
```
┌────────────────────────────────────────────────────────┐
│ Layer 3: crates/dither-gui  (egui / eframe / WGPU UI)   │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Layer 2: crates/dither-cli  (Clap CLI, MCP, Batch tool) │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│ Layer 1: crates/dither-core (Pure DSP, Algorithms, CRT)│
└────────────────────────────────────────────────────────┘
```
- **Rule**: `dither-core` has ZERO dependencies on UI, egui, eframe, or windowing libraries. It operates purely on raw byte slices, typed image buffers, and configuration structs.

### 2.5 Non-Blocking UI & Zero-Allocation Hot Loops
- The GUI thread runs at $\ge 60$ FPS. Dithering computations, video frame conversions, and batch exports execute asynchronously on background Rayon worker threads.
- Pixel loops must not allocate on the heap (`Vec::new`, formatting strings) inside per-pixel hot iterations.

---

## 3. Knowledge Graph MCP Reference

### code-review-graph
- `detect_changes_tool`: Review modifications with risk scoring.
- `get_impact_radius_tool`: Trace blast radius across crates.
- `get_affected_flows_tool`: Trace user execution flows.
- `query_graph_tool`: Trace `callers_of`, `callees_of`, `imports_of`.

### codebase-memory-mcp
- Pass `project="mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dithercraft"`.
- Use `search_graph`, `trace_path`, `get_code_snippet`.

### headroom
- Context compression and retrieval: `headroom_compress`, `headroom_retrieve`.

---

## 4. Conventional Commits Standard (MANDATORY)

Every task that touches files MUST be committed before concluding:
```
<type>(<scope>): <short imperative summary>

- <bullet describing what changed and why>
```
Valid scopes: `core`, `gui`, `cli`, `preset`, `ci`, `docs`, `ai`.
Always use `rg` (ripgrep) for searching.
