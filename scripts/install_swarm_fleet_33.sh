#!/usr/bin/env sh
# Install the 33 swarm manifests as an auditable, reversible control-plane bundle.
set -eu
REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SWARM_ROOT=${OCAF_SWARM_FLEET_ROOT:-"$REPO_ROOT/OCAF Agents MOJEALTEREGO/SWARM-FLEET-33"}
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
[ -f "$SWARM_ROOT/fleet_manifest.json" ] || { echo "swarm fleet manifest missing: $SWARM_ROOT/fleet_manifest.json" >&2; exit 2; }
count=$(find "$SWARM_ROOT" -mindepth 2 -maxdepth 2 -name manifest.json -type f | wc -l | tr -d ' ')
[ "$count" -eq 33 ] || { echo "expected 33 swarm manifests, got $count" >&2; exit 3; }
DEST="$PREFIX/share/ocaf-swarm-fleet-33"
BACKUP="$PREFIX/share/ocaf-swarm-fleet-33.rollback"
if [ -d "$DEST" ]; then rm -rf "$BACKUP"; mv "$DEST" "$BACKUP"; fi
mkdir -p "$DEST"
cp -p "$SWARM_ROOT/fleet_manifest.json" "$DEST/"
find "$SWARM_ROOT" -mindepth 2 -maxdepth 2 -name manifest.json -type f -exec cp -p {} "$DEST/" \;
printf '%s\n' "installed 33 verified swarm manifests; rollback=$BACKUP"
