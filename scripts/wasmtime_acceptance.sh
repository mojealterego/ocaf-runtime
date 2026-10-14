#!/usr/bin/env bash
set -euo pipefail

# Pinned upstream Wasmtime 49.0.2 x86_64 Linux distribution.
# Integrity is verified BEFORE extraction/execution, using upstream
# release-asset digest published by bytecodealliance/wasmtime.
version="v49.0.2"
archive="wasmtime-v49.0.2-x86_64-linux.tar.xz"
expected="a4d6e9e3a5a60f527cf7793d674c48930c80c2e8977995b8a275cad3254b9322"

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "WASI acceptance requires explicitly supported Linux x86_64 runner" >&2
  exit 2
fi

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
url="https://github.com/bytecodealliance/wasmtime/releases/download/$version/$archive"

curl --fail --silent --show-error --location --retry 3 \
  --proto '=https' --tlsv1.2 "$url" -o "$work/$archive"
printf '%s  %s\n' "$expected" "$work/$archive" | sha256sum --check --status
tar --extract --xz --file "$work/$archive" --directory "$work"
executable="$work/wasmtime-v49.0.2-x86_64-linux/wasmtime"
test -x "$executable"

"$executable" --version
OCAF_TEST_WASMTIME="$executable" cargo test --locked --no-default-features \
  --test wasi_sandbox -- --test-threads=1 --nocapture

# Ten bounded real-engine CI soak cycles with explicit resource ceilings.
# Operators may run 100+ cycles with explicit resource ceilings on their
# target host; a CI pass is not a multi-host security acceptance.
report="${OCAF_WASI_ACCEPTANCE_REPORT:-${GITHUB_WORKSPACE:-$PWD}/wasi-host-acceptance.json}"
OCAF_TEST_WASMTIME="$executable" \
OCAF_TEST_WASMTIME_SHA256="$(sha256sum "$executable" | cut -d ' ' -f 1)" \
python3 scripts/wasi_host_acceptance.py \
  --cycles "${OCAF_WASI_SOAK_CYCLES:-10}" \
  --timeout 120 \
  --max-rss-kib 1572864 \
  --max-fds 512 \
  --max-processes 64 \
  --report "$report"
