#!/usr/bin/env bash
# CI: translate GPUI's geometry.rs and color.rs (zed at a pinned rev) and
# compile-check the output with `mojo package`. Non-blocking in CI.
#
#   scripts/ci_gpui.sh LOGDIR [ZED_DIR]
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGDIR="${1:-$ROOT/target/gpui-logs}"
ZED="${2:-$ROOT/target/corpus/zed}"
ZED_REV=596dfdcf32d85d4d8d17a8887b4d0979ec7c5e14
MOJO="${MOJO:-mojo}"
mkdir -p "$LOGDIR"
if [ ! -f "$ZED/crates/gpui/src/geometry.rs" ]; then
  mkdir -p "$ZED"
  git -C "$ZED" init -q
  git -C "$ZED" remote add origin https://github.com/zed-industries/zed 2>/dev/null || true
  git -C "$ZED" sparse-checkout set crates/gpui/src
  git -C "$ZED" fetch -q --depth 1 --filter=blob:none origin "$ZED_REV"
  git -C "$ZED" checkout -q FETCH_HEAD
fi
BIN="$ROOT/target/release/rust-to-mojo"
OUT="$LOGDIR/out"
rm -rf "$OUT"
"$BIN" "$ZED/crates/gpui/src/geometry.rs" -o "$OUT/geometry" --optimize-hints >"$LOGDIR/translate-geometry.txt"
"$BIN" "$ZED/crates/gpui/src/color.rs" -o "$OUT/color" --optimize-hints >"$LOGDIR/translate-color.txt"
# geometry.rs is a single module: wrap it as a package so `mojo package` checks it.
mkdir -p "$OUT/geometry_pkg/geometry"
cp "$OUT/geometry/geometry.mojo" "$OUT/geometry_pkg/geometry/__init__.mojo"
fail=0
check() { # name pkgdir includedir
  if "$MOJO" precompile -I "$3" "$2" -o "$LOGDIR/$1.mojoc" >"$LOGDIR/$1.log" 2>&1; then
    echo "ok    $1"
  else
    echo "FAIL  $1 ($(grep -c 'error:' "$LOGDIR/$1.log") errors)"
    grep -A3 'error:' "$LOGDIR/$1.log" | head -60
    fail=$((fail + 1))
  fi
}
check geometry "$OUT/geometry_pkg/geometry" "$OUT/geometry"
check color "$OUT/color/color" "$OUT/color"
exit "$fail"
