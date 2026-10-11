#!/usr/bin/env bash
# Build and package DitherCraft for Linux (<arch> is x86_64 or aarch64):
#
#   $DIST/dithercraft-<version>-linux-<arch>.AppImage  any distro with glibc >= the build host's
#   $DIST/dithercraft-<version>-linux-<arch>.tar.gz    plain FHS-style tree (bin/, share/)
#
# Usage: packaging/linux/package.sh [--skip-build] [--formats "appimage tar"]
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/linux"
APP_ID=io.github.dithercraft

SKIP_BUILD=0
FORMATS="tar"
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --formats) FORMATS="$2"; shift 2 ;;
    -h | --help) exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ARCH="${CROSS_ARCH:-$(uname -m)}"
case "$ARCH" in
  x86_64) ARCH=x86_64 ;;
  aarch64 | arm64) ARCH=aarch64 ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
BASENAME="dithercraft-$VERSION-linux-$ARCH"

echo "==> DitherCraft $VERSION for Linux $ARCH ($FORMATS)"

if [ "$SKIP_BUILD" = 0 ]; then
  (cd "$ROOT" && cargo build --release -p dithercraft -p dithercraft-cli)
fi
BIN="$CARGO_TARGET_DIR/release"
WORK="$CARGO_TARGET_DIR/linux-package"
STAGE="$WORK/root"
rm -rf "$WORK"

mkdir -p "$STAGE/usr/bin" "$STAGE/usr/share/applications" "$STAGE/usr/share/metainfo"
cp "$BIN/dithercraft" "$STAGE/usr/bin/"
cp "$BIN/dithercraft-cli" "$STAGE/usr/bin/"
cp "$HERE/io.github.dithercraft.desktop" "$STAGE/usr/share/applications/"
cp "$HERE/io.github.dithercraft.metainfo.xml.in" "$STAGE/usr/share/metainfo/io.github.dithercraft.metainfo.xml"

if [[ " $FORMATS " =~ " tar " ]]; then
  echo "--> $DIST/$BASENAME.tar.gz"
  tar -czf "$DIST/$BASENAME.tar.gz" -C "$STAGE/usr" bin share
fi
echo "==> Linux package build complete."
