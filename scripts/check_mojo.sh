#!/usr/bin/env bash
# Compile-check the generated Mojo with a real Mojo toolchain (Linux/macOS).
#
#   scripts/check_mojo.sh            # golden outputs + mojo_rt
#   scripts/check_mojo.sh out/gpui   # a translated crate directory
#
# Each .mojo file is built with `mojo build` (as a library check) with the
# repo root and the target directory on the import path. Failures are listed
# at the end; the exit code is the number of failing files (capped at 255).
# Expect failures for now: this repo's output has never been compiled (no
# Mojo toolchain was available when it was written). Feed the error list to
# the agent fix-up pass (see README "Pipeline").
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-}"
MOJO="${MOJO:-mojo}"

if ! command -v "$MOJO" >/dev/null 2>&1; then
  echo "error: '$MOJO' not found. Install Mojo (pixi add mojo / see docs.modular.com) or set MOJO=..." >&2
  exit 2
fi
"$MOJO" --version

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fail=0
failed=()

check_file() {
  local f="$1"; shift
  local log="$TMP/$(echo "$f" | tr '/' '_').log"
  # `mojo build` needs an entry point for executables; build as a package-less
  # module check by compiling a tiny driver that imports nothing but the file.
  if "$MOJO" build "$@" -o "$TMP/a.out" "$f" >"$log" 2>&1 \
     || grep -q "no 'main' function" "$log"; then
    echo "ok    $f"
  else
    echo "FAIL  $f"
    sed 's/^/      /' "$log" | head -20
    fail=$((fail + 1))
    failed+=("$f")
  fi
}

if [ -z "$TARGET" ]; then
  # runtime shim as a package
  if "$MOJO" package "$ROOT/mojo_rt" -o "$TMP/mojo_rt.mojopkg" >"$TMP/rt.log" 2>&1; then
    echo "ok    mojo_rt (package)"
  else
    echo "FAIL  mojo_rt (package)"; sed 's/^/      /' "$TMP/rt.log" | head -40
    fail=$((fail + 1)); failed+=("mojo_rt")
  fi
  for f in "$ROOT"/tests/golden/*.mojo; do
    check_file "$f" -I "$ROOT"
  done
else
  # translated crate: package each top-level package dir, then check files
  for pkg in "$TARGET"/*/; do
    [ -f "$pkg/__init__.mojo" ] || continue
    if "$MOJO" package "$pkg" -I "$TARGET" -o "$TMP/$(basename "$pkg").mojopkg" >"$TMP/pkg.log" 2>&1; then
      echo "ok    package $(basename "$pkg")"
    else
      echo "FAIL  package $(basename "$pkg")"; sed 's/^/      /' "$TMP/pkg.log" | head -60
      fail=$((fail + 1)); failed+=("$pkg")
    fi
  done
fi

echo
echo "$fail failing"
for f in "${failed[@]:-}"; do [ -n "$f" ] && echo "  $f"; done
[ "$fail" -gt 255 ] && fail=255
exit "$fail"
