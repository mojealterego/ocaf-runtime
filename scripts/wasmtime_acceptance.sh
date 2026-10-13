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
  --test wasi_sandbox real_wasmtime_cli_when_configured -- --exact --nocapture
