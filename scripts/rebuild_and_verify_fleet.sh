#!/usr/bin/env sh
# Rebuild every agent and swarm several times; never labels an unbuilt artifact complete.
set -eu
REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUNS=${OCAF_REBUILD_RUNS:-3}
OUT=${OCAF_REBUILD_OUT:-"$REPO_ROOT/dist/rebuilds"}
[ "$RUNS" -ge 2 ] || { echo 'OCAF_REBUILD_RUNS must be at least 2' >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || exit 2
mkdir -p "$OUT"
for run in $(seq 1 "$RUNS"); do
  run_dir="$OUT/run-$run"
  mkdir -p "$run_dir"
  # This command creates artifacts only when Cargo and the complete fleet are present.
  OCAF_AGENT_FLEET_OUT="$run_dir/agent-fleet-111" \
    "$REPO_ROOT/scripts/build_agent_fleet_111.sh"
  OCAF_SWARM_FLEET_ROOT="$REPO_ROOT/OCAF Agents MOJEALTEREGO/SWARM-FLEET-33" \
    "$REPO_ROOT/scripts/install_swarm_fleet_33.sh"
  sha256sum "$run_dir/agent-fleet-111/artifacts.sha256" > "$run_dir/build-receipt.sha256"
done
printf '%s\n' "completed $RUNS independent fleet build runs"
