#!/usr/bin/env bash
# OCAF Agents Builder GATE-10: verify signed evidence, never infer live deployment.
set -euo pipefail
if [[ $# -ne 3 ]]; then
  printf 'Usage: %s <full-main-git-sha> <release-dir> <trusted-ed25519-public-key-hex>\n' "$0" >&2
  exit 2
fi
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd -P)"
exec python3 "${SCRIPT_DIR}/verify_builder_gate10.py" \
  --repo "${REPO_ROOT}" \
  --release-dir "$2" \
  --head-sha "$1" \
  --trusted-public-key-hex "$3"
