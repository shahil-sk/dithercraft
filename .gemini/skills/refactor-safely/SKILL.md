---
name: refactor-safely
description: >-
  Plan and execute safe refactoring across the Cargo workspace using dependency analysis and impact preview.
  Use when modifying kernel traits, reorganizing crates, or cleaning dead code.
---

# Refactor Safely (Rust)

Refactor components across the Dither Guy Cargo workspace safely without introducing regressions or breaking crate boundaries.

## Safety Guidelines

1. **Dead Code Elimination**:
   - Use `refactor_tool(mode="dead_code")` to detect unreferenced functions and unused structs.
   - Run `cargo check --workspace` to review `dead_code` compiler warnings.

2. **Renaming & Signature Changes**:
   - Preview changes before applying: `refactor_tool(mode="rename", target="<old_name>", new_name="<new_name>")`.
   - Use `get_impact_radius_tool` to understand all caller sites across `crates/dither-core`, `crates/dither-cli`, and `crates/dither-gui`.

3. **Crate Decoupling Invariants**:
   - `crates/dither-core`: Pure computational routines with NO GUI or windowing crate dependencies.
   - `crates/dither-cli`: Headless CLI and MCP server tools.
   - `crates/dither-gui`: egui/eframe presentation layer communicating with core algorithms via commands and background Rayon worker threads.

4. **Post-Refactoring Validation**:
   - Verify that the entire workspace builds and passes all tests:
     ```bash
     cargo check --workspace --all-targets
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace --all-targets
     ```
