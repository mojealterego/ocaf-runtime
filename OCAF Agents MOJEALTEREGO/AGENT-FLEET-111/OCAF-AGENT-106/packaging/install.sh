#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/ocaf-agent-106-target}
command -v cargo >/dev/null 2>&1 || { echo "cargo is required to build OCAF-AGENT-106" >&2; exit 1; }
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --manifest-path "$ROOT/Cargo.toml" --jobs "${CARGO_BUILD_JOBS:-1}"
BINARY="$TARGET_DIR/debug/ocaf-agent-106"; [ -x "$BINARY" ] || exit 1
mkdir -p "$PREFIX/bin" "$PREFIX/share/ocaf-agent-106"; install -m 0755 "$BINARY" "$PREFIX/bin/ocaf-agent-106"; sha256sum "$PREFIX/bin/ocaf-agent-106" > "$PREFIX/share/ocaf-agent-106/artifact.sha256"; cp "$ROOT/config/agent.toml" "$PREFIX/share/ocaf-agent-106/agent.toml"
printf '%s\n' "installed OCAF-AGENT-106 as STAGED; ACTIVE denied"
