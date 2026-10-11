---
name: explore-codebase
description: >-
  Navigate and understand the Dither Guy codebase structure using knowledge graph tools.
  Use when exploring modules, classes, functions, caller/callee relationships, or architecture.
---

# Explore Codebase

Use knowledge graph tools (`code-review-graph` and `codebase-memory-mcp`) to explore and understand the Dither Guy codebase with token efficiency.

## Knowledge Graph Tools

### 1. code-review-graph MCP
- `list_graph_stats_tool`: View codebase size, nodes, edges, and languages.
- `get_architecture_overview_tool`: Understand architectural communities and modular structure.
- `list_communities_tool` & `get_community_tool`: Inspect major functional subsystems (`ui`, `utils/dither_kernels`, `utils/workers`, etc.).
- `semantic_search_nodes_tool`: Search functions and classes by semantic keywords or names.
- `query_graph_tool`: Trace relationships using patterns:
  - `callers_of`: Functions calling the target function.
  - `callees_of`: Functions called by the target function.
  - `imports_of`: Files imported by a target module.
  - `children_of`: Classes and functions defined in a target file.
- `list_flows_tool` & `get_flow_tool`: Inspect execution flows across the application.

### 2. codebase-memory-mcp
- Note: Always pass `project="mnt-5893accc-4b16-4de7-b2a7-6333530b2614-PROJECTS-21_Active-dither-guy"`.
- `search_graph`: Find symbol declarations by regex pattern (e.g., `.*dither.*`).
- `trace_path`: Trace inbound or outbound call hierarchies.
- `get_code_snippet`: Retrieve specific function or class definitions directly.
- `get_architecture`: View high-level architecture overview.

## Exploration Workflow

1. Start broad: Run `get_architecture_overview_tool` or `get_minimal_context_tool(task="<task>")`.
2. Locate specific symbols: Use `semantic_search_nodes_tool` or `search_graph`.
3. Trace connections: Use `query_graph_tool` with `callers_of` / `callees_of` before opening files.
4. Read targeted code: Use `get_review_context_tool` or `get_code_snippet` rather than reading entire source files.

## Token Efficiency Rules
- ALWAYS start with `get_minimal_context_tool(task="<your task>")` before reading large files.
- Prefer `detail_level="minimal"` on initial graph queries.
- Fall back to file viewing or ripgrep (`rg`) only when structural graph data is insufficient.
