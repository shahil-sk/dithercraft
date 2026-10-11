<!-- code-review-graph instruction for GitHub Copilot and CI agents -->
## Knowledge Graph Integration
This repository uses `code-review-graph`.
When reviewing pull requests or generating diffs:
- Check `detect_changes_tool` for risk-scored change metrics.
- Check `get_affected_flows_tool` to understand impacted paths.
- Check `get_impact_radius_tool` to verify blast radius.
- Verify that UI threading constraints and Numba kernel boundaries are respected.
