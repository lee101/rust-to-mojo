#!/usr/bin/env bash
# Runtime-correctness diff: for each tests/runtime/NAME.mojo + NAME.rs pair,
# run the Mojo test (which imports the translated golden) and the Rust test
# (which includes the original golden .rs) and diff their stdout.
#
#   scripts/ci_runtime.sh LOGDIR
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGDIR="${1:-$ROOT/target/runtime-logs}"
MOJO="${MOJO:-mojo}"
mkdir -p "$LOGDIR"
: >"$LOGDIR/summary.tsv"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
# goldens as importable modules: tests/golden/02_newtype_ops.mojo -> g02_newtype_ops.mojo
mkdir -p "$WORK/mods"
for f in "$ROOT"/tests/golden/*.mojo; do
  cp "$f" "$WORK/mods/g$(basename "$f")"
done
fail=0
shopt -s nullglob
for t in "$ROOT"/tests/runtime/*.mojo; do
  name="$(basename "$t" .mojo)"
  rs="$ROOT/tests/runtime/$name.rs"
  st=ok
  if ! "$MOJO" run -I "$ROOT" -I "$WORK/mods" "$t" >"$LOGDIR/$name.mojo.out" 2>"$LOGDIR/$name.mojo.err"; then
    st="FAIL(mojo)"
  fi
  if ! rustc --edition 2021 -O -A warnings -o "$WORK/$name" "$rs" 2>"$LOGDIR/$name.rustc.err" \
     || ! "$WORK/$name" >"$LOGDIR/$name.rust.out"; then
    st="FAIL(rust)"
  fi
  if [ "$st" = ok ] && ! diff -u "$LOGDIR/$name.rust.out" "$LOGDIR/$name.mojo.out" >"$LOGDIR/$name.diff"; then
    st="FAIL(diff)"
  fi
  printf '%s\t%s\n' "$name" "$st" >>"$LOGDIR/summary.tsv"
  echo "$st  $name"
  if [ "$st" != ok ]; then
    fail=$((fail + 1))
    head -40 "$LOGDIR/$name.mojo.err" "$LOGDIR/$name.diff" 2>/dev/null | sed 's/^/      /'
  fi
done
echo "$fail failing"
exit "$fail"
