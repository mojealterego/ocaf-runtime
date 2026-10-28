#!/bin/sh
set -eu

ARCHIVE=${1:?usage: install.sh <ocaf-agent-001-asterion-0.1.0.tar.gz>}
PREFIX=${OCAF_AGENT_PREFIX:-"$HOME/.local"}
case "$PREFIX" in
  /*) ;;
  *) printf 'install prefix must be an absolute path\n' >&2; exit 1 ;;
esac
if [ "$PREFIX" = "/" ] || [ "$(realpath -m -- "$PREFIX")" != "$PREFIX" ]; then
  printf 'install prefix must be canonical and cannot be root\n' >&2
  exit 1
fi
for destination in "$PREFIX" "$PREFIX/bin" "$PREFIX/share" \
  "$PREFIX/bin/ocaf-agent-001" "$PREFIX/share/ocaf-agent-001"; do
  if [ -L "$destination" ]; then
    printf 'install destination cannot be a symlink: %s\n' "$destination" >&2
    exit 1
  fi
done
WORK=$(mktemp -d)
COMMITTED=0
rollback() {
  if [ "$COMMITTED" -eq 0 ] && [ -d "$WORK/backup" ]; then
    rm -f "$PREFIX/bin/ocaf-agent-001"
    rm -rf "$PREFIX/share/ocaf-agent-001"
    [ ! -f "$WORK/backup/ocaf-agent-001" ] || cp "$WORK/backup/ocaf-agent-001" "$PREFIX/bin/ocaf-agent-001"
    [ ! -d "$WORK/backup/share" ] || cp -R "$WORK/backup/share" "$PREFIX/share/ocaf-agent-001"
  fi
  rm -rf "$WORK"
}
trap rollback EXIT HUP INT TERM

# Check paths and entry types before extraction. Only the builder's fixed
# archive layout is accepted; links and extra files are rejected.
tar -tzf "$ARCHIVE" | while IFS= read -r item; do
  case "$item" in
    ocaf-agent-001/|ocaf-agent-001/bin/|ocaf-agent-001/share/|ocaf-agent-001/share/ocaf-agent-001/|\
    ocaf-agent-001/SHA256SUMS|ocaf-agent-001/bin/ocaf-agent-001|\
    ocaf-agent-001/share/ocaf-agent-001/agent.toml|\
    ocaf-agent-001/share/ocaf-agent-001/autonomy.toml|\
    ocaf-agent-001/share/ocaf-agent-001/tool_policy.toml|\
    ocaf-agent-001/share/ocaf-agent-001/README.md|\
    ocaf-agent-001/share/ocaf-agent-001/ARCHITECTURE.md|\
    ocaf-agent-001/share/ocaf-agent-001/USER_MANUAL.md|\
    ocaf-agent-001/share/ocaf-agent-001/CERTIFICATE_RECORD.md) ;;
    *) printf 'unexpected archive member: %s\n' "$item" >&2; exit 1 ;;
  esac
done
tar -tvzf "$ARCHIVE" | awk '{ type = substr($1, 1, 1); if (type != "-" && type != "d") exit 1 }'
tar -C "$WORK" -xzf "$ARCHIVE" --no-same-owner
cat > "$WORK/expected-paths" <<'EOF'
bin/ocaf-agent-001
share/ocaf-agent-001/ARCHITECTURE.md
share/ocaf-agent-001/CERTIFICATE_RECORD.md
share/ocaf-agent-001/README.md
share/ocaf-agent-001/USER_MANUAL.md
share/ocaf-agent-001/agent.toml
share/ocaf-agent-001/autonomy.toml
share/ocaf-agent-001/tool_policy.toml
EOF
awk 'NF != 2 || length($1) != 64 || $1 !~ /^[0-9A-Fa-f]+$/ { exit 1 } { print $2 }' \
  "$WORK/ocaf-agent-001/SHA256SUMS" > "$WORK/manifest-paths"
LC_ALL=C sort "$WORK/manifest-paths" > "$WORK/sorted-paths"
cmp -s "$WORK/expected-paths" "$WORK/sorted-paths" || {
  printf 'archive checksum manifest is incomplete or unexpected\n' >&2
  exit 1
}
(
  cd "$WORK/ocaf-agent-001"
  sha256sum -c SHA256SUMS >/dev/null
)
PACKAGE="$WORK/ocaf-agent-001"
[ -x "$PACKAGE/bin/ocaf-agent-001" ]
"$PACKAGE/bin/ocaf-agent-001" status "$PACKAGE/share/ocaf-agent-001/agent.toml" >/dev/null

mkdir -p "$PREFIX/bin" "$PREFIX/share"
mkdir "$WORK/backup"
if [ -f "$PREFIX/bin/ocaf-agent-001" ]; then
  cp "$PREFIX/bin/ocaf-agent-001" "$WORK/backup/ocaf-agent-001"
fi
if [ -d "$PREFIX/share/ocaf-agent-001" ]; then
  cp -R "$PREFIX/share/ocaf-agent-001" "$WORK/backup/share"
fi
install -m 0755 "$PACKAGE/bin/ocaf-agent-001" "$PREFIX/bin/ocaf-agent-001"
rm -rf "$PREFIX/share/ocaf-agent-001"
cp -R "$PACKAGE/share/ocaf-agent-001" "$PREFIX/share/ocaf-agent-001"
COMMITTED=1
printf 'installed OCAF-AGENT-001 locally as DRAFT; ACTIVE denied\n'
