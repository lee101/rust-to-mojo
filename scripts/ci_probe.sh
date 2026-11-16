#!/usr/bin/env bash
# Run each tests/probe/p_*.mojo (one Mojo syntax/semantics question each) and
# record whether the compiler accepted it. Informational: always exits 0.
#   scripts/ci_probe.sh LOGDIR
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGDIR="${1:-$ROOT/target/probe-logs}"
MOJO="${MOJO:-mojo}"
mkdir -p "$LOGDIR"
: >"$LOGDIR/summary.tsv"
for p in "$ROOT"/tests/probe/p_*.mojo; do
  n="$(basename "$p" .mojo)"
  if "$MOJO" run -I "$ROOT" "$p" >"$LOGDIR/$n.log" 2>&1; then st=ok; else st=FAIL; fi
  printf '%s\t%s\n' "$n" "$st" >>"$LOGDIR/summary.tsv"
  echo "$st  $n: $(grep -m1 'error:' "$LOGDIR/$n.log" | sed 's|.*error: ||' | cut -c1-160)"
done
exit 0
