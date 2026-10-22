#!/usr/bin/env sh
set -eu
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
rm -f "$PREFIX/bin/ocaf-agent-068"; rm -rf "$PREFIX/share/ocaf-agent-068"
