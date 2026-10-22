#!/usr/bin/env sh
set -eu
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
rm -f "$PREFIX/bin/ocaf-agent-016"; rm -rf "$PREFIX/share/ocaf-agent-016"
