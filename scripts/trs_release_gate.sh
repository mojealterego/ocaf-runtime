#!/usr/bin/env bash
# OCAF TRS release gate. Runs locally/on a self-hosted machine; GitHub Actions not required.
set -Eeuo pipefail
umask 077
export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=1
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${OCAF_EVIDENCE_DIR:-$ROOT/target/trs-release-evidence}"
mkdir -p "$OUT"
LOG="$OUT/verification.log"
exec > >(tee "$LOG") 2>&1
printf 'OCAF TRS verification started: %s\n' "$(date -u +%FT%TZ)"
# Never certify a source tree different from the commit packaged by git archive.
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo 'BLOCKED: tracked working tree is dirty; commit or revert before release' >&2
  exit 2
fi
git rev-parse HEAD | tee "$OUT/commit.txt"
command -v cargo >/dev/null || { echo 'BLOCKED: cargo missing' >&2; exit 127; }
command -v rustc >/dev/null || { echo 'BLOCKED: rustc missing' >&2; exit 127; }
cargo --version
rustc --version
# Fail closed: any failed step aborts packaging.
cargo fmt --all -- --check
cargo test --workspace --no-default-features --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
cargo test --locked --test trs_dtn_sqlite_concurrent_claim
cargo test --locked --test trs_dtn_sqlite_e2e_recovery
cargo test --locked --test trs_dtn_sqlite_total_bytes
cargo test --locked --test trs_dtn_sqlite_oversized_blob
printf 'PASS: %s\n' "$(date -u +%FT%TZ)" | tee "$OUT/STATUS"
# Only build distribution from a fully verified source checkout.
PKG="$OUT/ocaf-trs-source-$(git rev-parse --short HEAD).tar.gz"
git archive --format=tar HEAD | gzip -n > "$PKG"
sha256sum "$PKG" > "$PKG.sha256"
echo "Release gate PASS; package: $PKG"
