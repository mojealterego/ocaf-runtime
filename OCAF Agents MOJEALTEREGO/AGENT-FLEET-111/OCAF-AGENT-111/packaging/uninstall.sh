#!/usr/bin/env sh
set -eu
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
rm -f "$PREFIX/bin/ocaf-agent-111"; rm -rf "$PREFIX/share/ocaf-agent-111"
