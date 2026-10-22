#!/usr/bin/env bash
set -euo pipefail

# Reproducible Rust entrypoint for hosts without a local cargo/rustc install.
# The image is intentionally pinned by tag in this repository workflow; CI
# still remains the authoritative remote evidence for release decisions.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ca_bundle="/etc/ssl/certs/ca-certificates.crt"

if [[ ! -f "$ca_bundle" ]]; then
  echo "rust-ci: host CA bundle not found: $ca_bundle" >&2
  exit 1
fi

if [[ $# -eq 0 ]]; then
  set -- cargo test --locked --no-default-features
fi

docker run --rm \
  -v "$repo_root:/work" \
  -v "$ca_bundle:/host-ca.crt:ro" \
  -w /work \
  -e SSL_CERT_FILE=/host-ca.crt \
  -e CARGO_HTTP_CAINFO=/host-ca.crt \
  -e RUSTUP_USE_CURL=1 \
  rust:1.90 sh -c '
    if [ "$1" = cargo ] && [ "$2" = clippy ]; then
      rustup component add clippy
    fi
    exec "$@"
  ' sh "$@"
