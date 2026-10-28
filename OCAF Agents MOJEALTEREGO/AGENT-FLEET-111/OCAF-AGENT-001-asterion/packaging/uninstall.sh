#!/bin/sh
set -eu
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
case "$PREFIX" in
  /*) ;;
  *) printf 'uninstall prefix must be an absolute path\n' >&2; exit 1 ;;
esac
if [ "$PREFIX" = "/" ] || [ "$(realpath -m -- "$PREFIX")" != "$PREFIX" ]; then
  printf 'uninstall prefix must be canonical and cannot be root\n' >&2
  exit 1
fi
for destination in "$PREFIX" "$PREFIX/bin" "$PREFIX/share" \
  "$PREFIX/bin/ocaf-agent-001" "$PREFIX/share/ocaf-agent-001"; do
  if [ -L "$destination" ]; then
    printf 'uninstall destination cannot be a symlink: %s\n' "$destination" >&2
    exit 1
  fi
done
rm -f "$PREFIX/bin/ocaf-agent-001"
rm -rf "$PREFIX/share/ocaf-agent-001"
printf 'removed local OCAF-AGENT-001 installation\n'
