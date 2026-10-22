#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/ocaf-agent-110-target}
command -v cargo >/dev/null 2>&1 || { echo "cargo is required to build OCAF-AGENT-110" >&2; exit 1; }
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --manifest-path "$ROOT/Cargo.toml" --jobs "${CARGO_BUILD_JOBS:-1}"
BINARY="$TARGET_DIR/debug/ocaf-agent-110"; [ -x "$BINARY" ] || exit 1
mkdir -p "$PREFIX/bin" "$PREFIX/share/ocaf-agent-110"; install -m 0755 "$BINARY" "$PREFIX/bin/ocaf-agent-110"; sha256sum "$PREFIX/bin/ocaf-agent-110" > "$PREFIX/share/ocaf-agent-110/artifact.sha256"; cp "$ROOT/config/agent.toml" "$PREFIX/share/ocaf-agent-110/agent.toml"
printf '%s\n' "installed OCAF-AGENT-110 as STAGED; ACTIVE denied"
