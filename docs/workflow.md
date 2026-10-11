# DitherCraft (Rust) — Development Workflow Guide (WORKFLOW.md)

This document standardizes the development lifecycle for the Pure Rust clean-room implementation of DitherCraft.

---

## 1. Task Lifecycle Overview

```
 ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
 │  1. Explore │ ──► │  2. Plan     │ ──► │ 3. Implement│
 │  (KG / MCP) │     │  (Ponytail)  │     │ (Decoupled) │
 └─────────────┘     └──────────────┘     └─────────────┘
                                                 │
 ┌─────────────┐     ┌──────────────┐            ▼
 │ 6. Commit   │ ◄── │  5. Review   │ ◄── ┌─────────────┐
 │(Conventional│     │  (CRG / Diff)│     │ 4. Test &   │
 └─────────────┘     └──────────────┘     │    Validate │
                                          └─────────────┘
```

---

## 2. Step-by-Step Engineering Protocol

### Step 1: Structural Exploration (Graph-First)
- Before modifying or reading files, consult the knowledge graph:
  - `code-review-graph`: `semantic_search_nodes_tool`, `query_graph_tool` (`callers_of`, `callees_of`, `imports_of`).
  - `codebase-memory-mcp`: `search_graph`, `trace_path`, `get_code_snippet`.
- Trace the actual execution flow end-to-end to identify where shared logic resides.

### Step 2: Planning with Ponytail (Default Active)
- Apply the Ponytail ladder:
  - Can we reuse existing math in `crates/dither-core` or algorithms in `crates/dither-core/src/algorithms/`?
  - Does the Rust standard library (`std::simd`, `std::cmp`, `std::collections`) or existing crates (`rayon`, `image`) already solve it?
  - What is the minimal working diff? (YAGNI).

### Step 3: Clean Implementation
- **Strict Decoupling**: Keep `crates/dither-core` completely decoupled from presentation. No GUI or windowing dependencies.
- **Zero Allocations**: Avoid heap allocations (`Vec::new`, formatting strings) in per-pixel inner loops. Pre-allocate row scratchpads.
- **Non-Blocking GUI**: In `crates/dither-gui`, never run heavy dithering or video encoding on the main UI thread. Dispatch all tasks to background Rayon worker threads.
- **No Panics**: Never use `unwrap()`, `expect()`, `panic!`, `todo!`, or `unimplemented!` in runtime production code. Propagate errors via `Result<T, DitherError>` and `?`.

### Step 4: Verification & Testing
- Run format and lint checks:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  ```
- Run unit and integration tests:
  ```bash
  cargo test --workspace --all-targets
  ```

### Step 5: Graph-Powered Review
- Run `detect_changes_tool` to inspect risk metrics and affected symbols.
- Run `get_impact_radius_tool` to confirm that no unintended callers or modules are disrupted.

### Step 6: Conventional Git Commit
- Stage changes and commit with conventional commit format:
  ```
  <type>(<scope>): <imperative summary>

  - <bullet explaining what changed and why>
  - <bullet for each significant change>
  ```
- Valid scopes: `core`, `gui`, `cli`, `preset`, `ci`, `docs`, `ai`.
- No broken code may ever be committed.
