#!/usr/bin/env bash
set -euo pipefail

# Run only against an operator-installed, approved Wasmtime executable.
engine="${WASMTIME_BIN:?Set WASMTIME_BIN to the approved Wasmtime executable}"
test -x "$engine" || { echo "Wasmtime executable is missing" >&2; exit 2; }
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$root/tests/fixtures/wasi_infinite_loop.wat"
test -f "$fixture"
"$engine" --version
output="$(mktemp)"
trap 'rm -f "$output"' EXIT
set +e
timeout --kill-after=2s 15s "$engine" run -W fuel=10000 "$fixture" >"$output" 2>&1
status=$?
set -e
if [ "$status" -eq 0 ]; then
  echo "FAIL: infinite-loop guest unexpectedly completed" >&2
  exit 1
fi
if [ "$status" -eq 124 ] || [ "$status" -eq 137 ] || [ "$status" -eq 125 ] || [ "$status" -eq 126 ] || [ "$status" -eq 127 ]; then
  echo "FAIL: watchdog stopped guest; fuel was not proven to work" >&2
  exit 1
fi
if ! grep -Eiq 'all fuel consumed|out of fuel|fuel exhausted|fuel has been consumed' "$output"; then
  echo "FAIL: guest failed without evidence of fuel exhaustion" >&2
  exit 1
fi
echo "PASS: Wasmtime terminated infinite-loop guest after exhausting fuel"
