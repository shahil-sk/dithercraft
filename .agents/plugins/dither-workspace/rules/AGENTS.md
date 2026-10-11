# Dither Guy (Rust) — Workspace Rules (Plugin Bundle)

This plugin provides complete development rules, knowledge graph integration, and engineering guidelines for the Dither Guy Pure Rust clean-room implementation of Dither Boy.

## Key Invariants
1. **Ponytail Mode Default**: Channel a lazy senior dev. Stop at first rung: YAGNI -> reuse -> `std` -> native UI -> minimal crates -> one line -> shortest diff.
2. **Pure Decoupled Math**: `crates/dither-core` must remain 100% decoupled from any GUI or windowing libraries.
3. **Non-Blocking UI**: Image and video processing MUST execute on background Rayon worker threads.
4. **Knowledge Graphs First**: ALWAYS query `code-review-graph` and `codebase-memory-mcp` before Grep/Glob.
5. **Mandatory Conventional Commits**: Every task modifying/creating/deleting files must be committed before finalizing.
6. **Ripgrep**: Always use `rg` instead of `grep` for terminal search commands.
