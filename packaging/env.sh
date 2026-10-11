# shellcheck shell=bash
# Shared setup for DitherCraft packaging scripts. Source it: `. "$(dirname "$0")/../env.sh"`.
#
# Exports:
#   ROOT                    workspace root
#   VERSION                 [workspace.package] version from Cargo.toml (override: DITHERCRAFT_VERSION)
#   DIST                    output directory for release artifacts (default: $ROOT/dist/release)
#   DITHERCRAFT_BUILD_SHA   git commit baked into the binaries
#   DITHERCRAFT_BUILD_DATE  UTC build date, YYYY-MM-DD
#   CARGO_TARGET_DIR        cargo's target dir (default: $ROOT/target)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

# The version lives in `[workspace.package] version` in the root Cargo.toml.
workspace_version() {
  awk '
    /^\[/ { in_pkg = ($0 == "[workspace.package]") ; next }
    in_pkg && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }
  ' "$ROOT/Cargo.toml"
}

VERSION="${DITHERCRAFT_VERSION:-$(workspace_version)}"
if [ -z "$VERSION" ]; then
  echo "error: could not read [workspace.package] version from $ROOT/Cargo.toml" >&2
  exit 1
fi
export VERSION

DIST="${DIST:-$ROOT/dist/release}"
mkdir -p "$DIST"
export DIST

if [ -z "${DITHERCRAFT_BUILD_SHA:-}" ]; then
  DITHERCRAFT_BUILD_SHA="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || true)"
fi
export DITHERCRAFT_BUILD_SHA
export DITHERCRAFT_BUILD_DATE="${DITHERCRAFT_BUILD_DATE:-$(date -u +%Y-%m-%d)}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

warn() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning::$*"; else echo "warning: $*" >&2; fi
}

copy_docs() {
  local dest="$1" f
  for f in README.md LICENSE LICENSE-MIT LICENSE-APACHE; do
    if [ -f "$ROOT/$f" ]; then cp "$ROOT/$f" "$dest/"; fi
  done
}
