#!/usr/bin/env bash
# code-review-graph: incremental update after write/replace
set -euo pipefail

if [ ! -t 0 ]; then
    cat > /dev/null || true
fi

REPO_DIR="/mnt/5893accc-4b16-4de7-b2a7-6333530b2614/PROJECTS/21_Active/dither-guy"
code-review-graph update --skip-flows --repo "$REPO_DIR" >/dev/null 2>&1 || true
echo "{}"
exit 0
