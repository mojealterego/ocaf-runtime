#!/usr/bin/env sh
# Build evidence producer for 111 separate agent packages.
# No synthetic artifacts: every package needs its own Cargo build and digest.
set -eu
REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FLEET_ROOT=${OCAF_AGENT_FLEET_ROOT:-"$REPO_ROOT/OCAF Agents MOJEALTEREGO/AGENT-FLEET-111"}
OUT_ROOT=${OCAF_AGENT_FLEET_OUT:-"$REPO_ROOT/dist/agent-fleet-111"}
PROFILE=${OCAF_AGENT_PROFILE:-dev}
BUILD_ONLY=${OCAF_BUILD_ONLY:-0}

[ -d "$FLEET_ROOT" ] || { echo "agent fleet root missing: $FLEET_ROOT" >&2; exit 2; }
command -v cargo >/dev/null 2>&1 || { echo 'cargo is required; no artifacts were created' >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo 'sha256sum is required' >&2; exit 2; }
mkdir -p "$OUT_ROOT" "$OUT_ROOT/logs"
count=0
: > "$OUT_ROOT/artifacts.sha256"
for manifest in "$FLEET_ROOT"/OCAF-AGENT-*/Cargo.toml; do
  [ -f "$manifest" ] || continue
  agent_dir=$(dirname "$manifest")
  agent_id=$(basename "$agent_dir")
  package=$(sed -n 's/^name = "\([^"]*\)"/\1/p' "$manifest" | head -n 1)
  [ -n "$package" ] || { echo "missing package name: $manifest" >&2; exit 3; }
  target="$OUT_ROOT/targets/$agent_id"
  mkdir -p "$target"
  if [ -f "$agent_dir/Cargo.lock" ]; then LOCKED=--locked; else LOCKED=; fi
  CARGO_TARGET_DIR="$target" cargo build --manifest-path "$manifest" --profile "$PROFILE" $LOCKED --jobs "${CARGO_BUILD_JOBS:-1}" 2>&1 | tee "$OUT_ROOT/logs/$agent_id.log"
  if [ "$PROFILE" = dev ]; then binary="$target/debug/$package"; else binary="$target/$PROFILE/$package"; fi
  [ -x "$binary" ] || { echo "missing artifact for $agent_id: $binary" >&2; exit 4; }
  if [ "$BUILD_ONLY" = 1 ]; then continue; fi
  mkdir -p "$OUT_ROOT/bin"
  install -m 0755 "$binary" "$OUT_ROOT/bin/$agent_id"
  sha256sum "$OUT_ROOT/bin/$agent_id" >> "$OUT_ROOT/artifacts.sha256"
  count=$((count + 1))
done
[ "$count" -eq 111 ] || { echo "expected 111 built artifacts, got $count" >&2; exit 5; }
printf '%s\n' "built and recorded $count agent artifacts in $OUT_ROOT"
