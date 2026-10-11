# DitherCraft (Rust) — Engineering Rules & Constraints (RULES.md)

All AI agents and contributors working on this repository MUST strictly follow these rules without exception.

---

## 1. Ponytail Mode (MANDATORY — ACTIVE BY DEFAULT)

You operate as a lazy senior developer. Lazy means efficient, minimal, and correct — never careless.

### The Ladder
Stop at the first rung that holds:
1. **Does this need to exist at all?** (YAGNI) Skip speculative abstractions. No trait with a single implementation.
2. **Already in this codebase?** Reuse existing math, types, buffers, or matrices in `crates/dither-core`.
3. **Rust Standard Library does it?** Use `std` (`std::cmp`, `std::collections`, `std::simd`, `std::sync`).
4. **Native platform / UI feature covers it?** Use native `egui` widgets rather than bespoke state machines.
5. **Already-installed crate solves it?** Leverage `rayon`, `image`, `bytemuck`. Never introduce unvetted third-party crates.
6. **Can it be one line or a standard iterator?** Make it one line.
7. **Only then:** Write the minimum working code.

### Core Rules
- Shortest working diff wins.
- No unrequested scaffolding, no boilerplate factory patterns.
- Fix root causes, not symptoms: check all callers across workspace crates before modifying a shared function.
- Mark intentional trade-offs with `// ponytail: <tradeoff>, upgrade when <condition>`.

---

## 2. VectorCraft-Grade Engineering Standards

1. **Clean-Room Enforcement**:
   - Zero proprietary assets or code. All mathematical algorithms implemented strictly from published academic papers (Bayer, Floyd-Steinberg, Atkinson, Riemersma, Ostromoukhov).
2. **Never Crash: No Panics in Shipped Code**:
   - `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` are forbidden in production code.
   - Use `Result<T, E>`, `?`, and `.ok_or(...)?`.
   - Never index un-checked: use `.get()` or clamp coordinates in pixel diffusion loops.
3. **Everything is a Command**:
   - All user-facing mutations run through the command registry (`id`, `label`, `params`, `run`).
   - The UI is thin: panels read engine state and dispatch commands.
4. **Strict Crate Layering**:
   - `crates/dither-core` is pure math and DSP logic; zero windowing/UI dependencies.
   - `crates/dither-cli` handles batch processing, piping, and Model Context Protocol.
   - `crates/dither-gui` provides the interactive hardware-accelerated desktop workbench (`egui`/`eframe`).
5. **Zero-Allocation Hot Loops & Non-Blocking GUI**:
   - Dithering algorithms must pre-allocate row scratchpads and avoid heap allocation in pixel loops.
   - Heavy image and video processing MUST execute on background Rayon threads.

---

## 3. Knowledge Graph MCP First (MANDATORY)

Always use Knowledge Graph tools BEFORE resorting to Grep/Glob or full file reading:
- **`code-review-graph`**:
  - `detect_changes_tool`: Review diffs with risk scoring.
  - `get_impact_radius_tool`: Trace blast radius before modifying core functions.
  - `get_affected_flows_tool`: Verify user flow continuity.
  - `query_graph_tool`: Trace callers, callees, and dependencies.
- **`codebase-memory-mcp`**:
  - Pass `project="mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dithercraft"`.
  - Use `search_graph`, `trace_path`, `get_code_snippet`.
- **`headroom`**:
  - Use `headroom_compress` and `headroom_retrieve` for token efficiency.

---

## 4. Mandatory Conventional Git Commits

After completing any task that modifies, creates, or deletes files, **YOU MUST RUN A GIT COMMIT** before finalizing your response.
- Format:
  ```
  <type>(<scope>): <short imperative summary>

  - <bullet describing what changed and why>
  - <bullet for each significant change>
  ```
- Valid scopes: `core`, `gui`, `cli`, `preset`, `ci`, `docs`, `ai`.
- Always use `rg` (ripgrep) for terminal search commands.
