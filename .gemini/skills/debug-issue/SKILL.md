---
name: debug-issue
description: >-
  Systematically debug issues, panics, and crashes in Dither Guy using graph-powered navigation and Rust diagnostics.
  Use when encountering errors, UI lags, dithering artifacts, or test failures.
---

# Debug Issue (Rust)

Systematically trace and resolve bugs in Dither Guy using structural knowledge graphs and Rust tooling.

## Debugging Procedure

1. **Reproduce & Capture Backtrace**:
   - Run with backtraces enabled:
     ```bash
     RUST_BACKTRACE=1 cargo test -- --nocapture
     RUST_BACKTRACE=1 cargo run -p dither-gui
     ```

2. **Locate Code via Knowledge Graph**:
   - Search the suspected error point using `semantic_search_nodes_tool` or `search_graph(project="mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dither-guy", ...)`.
   - Trace callers and callees: `query_graph_tool(pattern="callers_of", target="<symbol>")`.

3. **Check Recent Changes**:
   - Run `detect_changes_tool` to see if recent edits introduced regressions or broke contracts.
   - Use `get_impact_radius_tool` to identify affected subsystems.

4. **Common Dither Guy Bug Patterns**:
   - **Array Bounds / Index Overflow**: Unchecked pixel indexing at boundary rows/columns. Use `.get_mut()` or clamp coordinates.
   - **Main Thread Stalls**: Computation executed synchronously inside `egui::Ui` frame passes instead of background Rayon workers.
   - **Float NaN / Inf**: Unchecked division by zero when calculating error diffusion weights or normalized color distances.
   - **Memory Bloat**: Accumulating uncompressed frames in memory during video export instead of streaming through a pipe.

5. **Verify Fix**:
   - Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.
