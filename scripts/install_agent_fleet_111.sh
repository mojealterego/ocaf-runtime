#!/usr/bin/env sh
# Transactional installer for already-built, digest-recorded 111-agent fleet.
set -eu
REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ARTIFACT_ROOT=${OCAF_AGENT_FLEET_OUT:-"$REPO_ROOT/dist/agent-fleet-111"}
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
[ -d "$ARTIFACT_ROOT/bin" ] || { echo "artifact directory missing: $ARTIFACT_ROOT/bin" >&2; exit 2; }
[ -s "$ARTIFACT_ROOT/artifacts.sha256" ] || { echo 'artifact digest manifest missing' >&2; exit 2; }
command -v sha256sum >/dev/null 2>&1 || { echo 'sha256sum is required' >&2; exit 2; }
# Verify every listed digest before mutating the destination.
( cd "$ARTIFACT_ROOT" && sha256sum -c artifacts.sha256 )
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/ocaf-fleet-stage.XXXXXX")
BACKUP="$PREFIX/share/ocaf-agent-fleet-111.rollback"
cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT INT TERM
mkdir -p "$STAGE/bin" "$STAGE/share"
cp -p "$ARTIFACT_ROOT"/bin/OCAF-AGENT-* "$STAGE/bin/"
if [ -d "$PREFIX/share/ocaf-agent-fleet-111" ]; then
  rm -rf "$BACKUP"
  mv "$PREFIX/share/ocaf-agent-fleet-111" "$BACKUP"
fi
mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp -p "$STAGE/bin"/OCAF-AGENT-* "$PREFIX/bin/"
mv "$STAGE" "$PREFIX/share/ocaf-agent-fleet-111"
printf '%s\n' "installed 111 verified agent artifacts; rollback=$BACKUP; OSTc remains disabled"
