#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT=${1:-"$ROOT/dist/ocaf-agent-001-asterion-0.1.0.tar.gz"}
CARGO=${CARGO:-cargo}
TARGET=${CARGO_TARGET_DIR:-"$ROOT/target"}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT HUP INT TERM

"$CARGO" test --locked --manifest-path "$ROOT/Cargo.toml"
CARGO_TARGET_DIR="$TARGET" "$CARGO" build --locked --release --manifest-path "$ROOT/Cargo.toml"

PKG="$WORK/ocaf-agent-001"
mkdir -p "$PKG/bin" "$PKG/share/ocaf-agent-001"
install -m 0755 "$TARGET/release/ocaf-agent-001" "$PKG/bin/ocaf-agent-001"
install -m 0644 "$ROOT/config/agent.toml" "$PKG/share/ocaf-agent-001/agent.toml"
install -m 0644 "$ROOT/config/autonomy.toml" "$PKG/share/ocaf-agent-001/autonomy.toml"
install -m 0644 "$ROOT/config/tool_policy.toml" "$PKG/share/ocaf-agent-001/tool_policy.toml"
install -m 0644 "$ROOT/docs/README.md" "$PKG/share/ocaf-agent-001/README.md"
install -m 0644 "$ROOT/docs/ARCHITECTURE.md" "$PKG/share/ocaf-agent-001/ARCHITECTURE.md"
install -m 0644 "$ROOT/docs/USER_MANUAL.md" "$PKG/share/ocaf-agent-001/USER_MANUAL.md"
install -m 0644 "$ROOT/certification/CERTIFICATE_RECORD.md" "$PKG/share/ocaf-agent-001/CERTIFICATE_RECORD.md"
(
  cd "$PKG"
  sha256sum bin/ocaf-agent-001 share/ocaf-agent-001/*.toml share/ocaf-agent-001/*.md > SHA256SUMS
)
mkdir -p "$(dirname -- "$OUTPUT")"
tar -C "$WORK" -czf "$OUTPUT" ocaf-agent-001
sha256sum "$OUTPUT" > "$OUTPUT.sha256"
printf 'built %s\n' "$OUTPUT"
