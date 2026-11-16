#!/usr/bin/env bash
# CI compile check: mojo_rt + every golden .mojo, one log per file.
#
#   scripts/ci_mojo.sh LOGDIR
#
# Each golden is copied to a sanitized module name (golden file names start
# with a digit, which is not a valid Mojo identifier) as the __init__.mojo of a
# throwaway package and checked with `mojo package`, so the whole module is
# parsed and type-checked without needing a `main`.
# Writes LOGDIR/summary.tsv (name<TAB>ok|FAIL) and returns the failure count.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGDIR="${1:-$ROOT/target/mojo-logs}"
MOJO="${MOJO:-mojo}"
mkdir -p "$LOGDIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
: >"$LOGDIR/summary.tsv"
fail=0

record() { # name status log
  printf '%s\t%s\n' "$1" "$2" >>"$LOGDIR/summary.tsv"
  if [ "$2" = ok ]; then echo "ok    $1"; else
    echo "FAIL  $1"; sed 's/^/      /' "$3" | grep -v '^      $' | head -30
    fail=$((fail + 1))
  fi
}

"$MOJO" --version | tee "$LOGDIR/version.txt"

# 1. runtime shim
if "$MOJO" package "$ROOT/mojo_rt" -o "$WORK/mojo_rt.mojopkg" >"$LOGDIR/mojo_rt.log" 2>&1; then
  record mojo_rt ok "$LOGDIR/mojo_rt.log"
else
  record mojo_rt FAIL "$LOGDIR/mojo_rt.log"
fi

# 2. goldens
for f in "$ROOT"/tests/golden/*.mojo; do
  stem="$(basename "$f" .mojo)"
  mod="g_${stem}"
  mkdir -p "$WORK/$mod"
  cp "$f" "$WORK/$mod/__init__.mojo"
  log="$LOGDIR/$stem.log"
  if "$MOJO" package -I "$ROOT" "$WORK/$mod" -o "$WORK/$mod.mojopkg" >"$log" 2>&1; then
    record "$stem" ok "$log"
  else
    record "$stem" FAIL "$log"
  fi
done

echo
echo "$fail failing"
[ "$fail" -gt 255 ] && fail=255
exit "$fail"
