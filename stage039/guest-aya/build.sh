#!/usr/bin/env bash
# Build the actual Aya eBPF ELF and a static musl guest loader from pinned
# toolchains. All temporary artifacts live outside the Git repository.
set -euo pipefail
[[ $# -eq 1 && "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
  echo 'usage: build.sh ABSOLUTE_OUTPUT_DIRECTORY (Linux x86_64)' >&2
  exit 2
}
OUT="$1"
[[ "$OUT" == /* && "$OUT" != *'/../'* ]] || exit 2
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP="$(mktemp -d /tmp/ocaf039-guest-aya-XXXXXXXX)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT"
export CARGO_HOME="${CARGO_HOME:-$TMP/cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$TMP/rustup}"
mkdir -p "$CARGO_HOME/bin" "$RUSTUP_HOME"
export PATH="$CARGO_HOME/bin:$PATH"

if ! command -v rustup >/dev/null; then
  curl --fail --silent --show-error --location --retry 2 --max-time 120 \
    --proto '=https' --tlsv1.2 https://sh.rustup.rs -o "$TMP/rustup-init.sh"
  sh "$TMP/rustup-init.sh" -y --profile minimal --default-toolchain 1.99.0 --no-modify-path
fi
rustup toolchain install 1.99.0 --profile minimal
rustup toolchain install nightly-2026-10-24 --profile minimal --component rust-src
rustup target add x86_64-unknown-linux-musl --toolchain 1.99.0

LINKER_ARCHIVE="$TMP/bpf-linker-v0.12.0.tar.zst"
curl --fail --silent --show-error --location --retry 2 --max-time 120 \
  --proto '=https' --tlsv1.2 \
  https://github.com/aya-rs/bpf-linker/releases/download/v0.12.0/bpf-linker-x86_64-unknown-linux-musl.tar.zst \
  -o "$LINKER_ARCHIVE"
echo "6789717c8cb6a2a5d920f3b6bfac2bd7efc00f6228ecd149108a99699a75c679  $LINKER_ARCHIVE" | sha256sum --check -
tar --zstd -xf "$LINKER_ARCHIVE" -C "$CARGO_HOME/bin" bpf-linker
chmod 0755 "$CARGO_HOME/bin/bpf-linker"

export CARGO_TARGET_DIR="$TMP/ebpf-target"
cargo +nightly-2026-10-24 build -Z build-std=core --target bpfel-unknown-none \
  --release --locked --manifest-path "$ROOT/ebpf/Cargo.toml"
ELF="$CARGO_TARGET_DIR/bpfel-unknown-none/release/ocaf-stage039-guest-aya-ebpf"
[[ -s "$ELF" ]] || exit 3
export OCAF_GUEST_AYA_EBPF_PATH="$ELF"
export OCAF_GUEST_AYA_EBPF_SHA256="$(sha256sum "$ELF" | cut -d ' ' -f 1)"

export CARGO_TARGET_DIR="$TMP/loader-target"
cargo +1.99.0 test --locked --manifest-path "$ROOT/loader/Cargo.toml"
cargo +1.99.0 build --target x86_64-unknown-linux-musl --release --locked \
  --manifest-path "$ROOT/loader/Cargo.toml"
LOADER="$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/ocaf-guest-openat-aya"
[[ -s "$LOADER" ]] || exit 3
install -m 0444 "$ELF" "$OUT/ocaf-guest-openat-aya-ebpf.elf"
install -m 0555 "$LOADER" "$OUT/ocaf-guest-openat-aya"
sha256sum "$OUT/ocaf-guest-openat-aya-ebpf.elf" "$OUT/ocaf-guest-openat-aya" \
  > "$OUT/sha256sums.txt"
cat "$OUT/sha256sums.txt"
