#!/usr/bin/env bash
# code-review-graph: session start status (AI workflow hook)
# Must output ONLY JSON on stdout. Logs go to stderr. Never blocks the session.
set -euo pipefail

if [ ! -t 0 ]; then
    cat > /dev/null || true
fi

REPO_DIR="/mnt/5893accc-4b16-4de7-b2a7-6333530b2614/PROJECTS/21_Active/dither-guy"
msg="$(code-review-graph status --repo "$REPO_DIR" 2>&1 | head -n 1 || true)"

CRG_MSG="$msg" python3 -c '
import json,os
m=os.environ.get("CRG_MSG","")
print(json.dumps({"systemMessage":m,"suppressOutput":True}))
' 2>/dev/null || echo '{"suppressOutput": true}'
exit 0
