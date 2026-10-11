---
name: review-changes
description: >-
  Perform a risk-scored, graph-informed code review of modifications.
  Use before finalizing edits, submitting pull requests, or committing changes.
---

# Review Changes

Perform a structured, risk-aware review of code modifications using knowledge graph tools.

## Review Steps

1. **Detect Changes & Risk Scoring**:
   - Run `detect_changes_tool` to get risk scores and affected symbols.
   - Run `get_affected_flows_tool` to determine which user flows (image processing, video export, batch mode, preset load) are affected.

2. **Verify Blast Radius**:
   - Run `get_impact_radius_tool` on modified files.
   - For high-risk functions, run `query_graph_tool(pattern="callers_of", target="...")` to verify all callers adapt to new behavior.

3. **Check Code Quality & Invariants**:
   - UI thread safety: Verify no long loops run on the main thread.
   - Memory management: Verify NumPy arrays are freed or reused appropriately during video processing.
   - Error handling: Graceful handling for invalid files, corrupted presets, or unsupported video codecs.

4. **Output Format**:
   - Group findings by severity (High / Medium / Low).
   - Detail impact, potential edge cases, and merge recommendation.
