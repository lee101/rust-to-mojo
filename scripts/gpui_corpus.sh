#!/usr/bin/env bash
# Run the converter over Zed's gpui crate and refresh reports/gpui/.
#
#   scripts/gpui_corpus.sh [ZED_CHECKOUT_DIR] [OUT_DIR]
#
# Clones zed (sparse, crates/gpui only) if no checkout is given.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ZED="${1:-$ROOT/target/corpus/zed}"
OUT="${2:-$ROOT/target/corpus/out}"

if [ ! -d "$ZED/crates/gpui" ]; then
  git clone --depth 1 --filter=blob:none --sparse https://github.com/zed-industries/zed "$ZED"
  git -C "$ZED" sparse-checkout set crates/gpui
fi
ZED_REV="$(git -C "$ZED" rev-parse HEAD 2>/dev/null || echo unknown)"

cargo build --release --manifest-path "$ROOT/Cargo.toml"
BIN="$ROOT/target/release/rust-to-mojo"
[ -x "$BIN" ] || BIN="$BIN.exe"

mkdir -p "$ROOT/reports/gpui"
rm -rf "$OUT"
# single files
"$BIN" "$ZED/crates/gpui/src/geometry.rs" -o "$OUT/geometry" --optimize-hints
"$BIN" "$ZED/crates/gpui/src/color.rs" -o "$OUT/color" --optimize-hints
# whole crate
"$BIN" "$ZED/crates/gpui" -o "$OUT/crate" --optimize-hints --top 15 | tee "$ROOT/reports/gpui/summary.txt"

cp "$OUT/geometry/geometry.mojo" "$ROOT/reports/gpui/geometry.mojo"
cp "$OUT/geometry/coverage.json" "$ROOT/reports/gpui/geometry.coverage.json"
cp "$OUT/geometry/coverage.md" "$ROOT/reports/gpui/geometry.coverage.md"
cp "$OUT/geometry/hints.json" "$ROOT/reports/gpui/geometry.hints.json"
# color.rs has an inline `mod`, so it becomes a package
cp "$OUT/color/color/__init__.mojo" "$ROOT/reports/gpui/color.mojo"
cp "$OUT/color/color/property.mojo" "$ROOT/reports/gpui/color.property.mojo"
cp "$OUT/color/coverage.json" "$ROOT/reports/gpui/color.coverage.json"
cp "$OUT/crate/coverage.json" "$ROOT/reports/gpui/crate.coverage.json"
cp "$OUT/crate/coverage.md" "$ROOT/reports/gpui/crate.coverage.md"
cp "$OUT/crate/hints.json" "$ROOT/reports/gpui/crate.hints.json"
echo "zed revision: $ZED_REV" > "$ROOT/reports/gpui/SOURCE.txt"
echo "reports refreshed in $ROOT/reports/gpui"
