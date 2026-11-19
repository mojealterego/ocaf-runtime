#!/usr/bin/env bash
# Integration-test adapter only. The caller runs the Rust full-stack fixture.
# No persistent OAuth credentials or production identities are permitted.
set -euo pipefail
umask 077
root="$(realpath "${OCAF_KALI_ROOTFS:?provide an isolated test rootfs}")"
case "$root" in /tmp/ocaf-kali-e2e.*/rootfs) ;; *) echo "Unsafe Kali lab root" >&2; exit 2;; esac
[[ -d "$root" && -O "$root" && ! -L "$root/home/ocaf" ]]
grep -qx 'ID=kali' "$root/etc/os-release"
enter="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)/verify_builder_kali_namespace.sh"
[[ -f "$enter" && -O "$enter" && ! -L "$enter" ]]
[[ "${OCAF_BUILDER_CLOUD_ORIGIN:?}" =~ ^https://127[.]0[.]0[.]1:[0-9]+$ ]]
# Accept only the fixture filenames in one private temporary directory.
src="$(dirname "${OCAF_BUILDER_CLOUD_KEY:?}")"
[[ "$src" == /tmp/.tmp* && -O "$src" && ! -L "$src" ]]
[[ "$OCAF_BUILDER_CLOUD_KEY" == "$src/alice.key" && "$OCAF_BUILDER_CLOUD_CERT" == "$src/alice.crt" && "$OCAF_BUILDER_CLOUD_CA" == "$src/ca.crt" ]]
for file in alice.key alice.crt ca.crt; do
  [[ -f "$src/$file" && -O "$src/$file" && ! -L "$src/$file" ]]
done
fixture="$(mktemp -d "$root/home/ocaf/gui-fixture.XXXXXXXX")"
cleanup() {
  rm -f -- "$fixture/ca.crt" "$fixture/alice.crt" "$fixture/alice.key"
  rmdir -- "$fixture"
}
trap cleanup EXIT
for file in ca.crt alice.crt alice.key; do
  install -m 600 "$src/$file" "$fixture/$file"
done
inside="${fixture#"$root"}"
# Tokens pass via environment only, not command-line arguments or output.
sudo -n --preserve-env=OCAF_TEST_GUI_TOKEN,OCAF_TEST_GUI_FOREIGN_TOKEN,OCAF_BUILDER_CLOUD_ORIGIN \
  unshare -mpf --kill-child "$enter" "$root" \
  /usr/bin/setpriv --reuid "$(id -u)" --regid "$(id -g)" --clear-groups \
  /usr/bin/env HOME=/home/ocaf LANG=C.UTF-8 USER=ocaf \
  OCAF_BUILDER_CLOUD_CA="$inside/ca.crt" \
  OCAF_BUILDER_CLOUD_CERT="$inside/alice.crt" \
  OCAF_BUILDER_CLOUD_KEY="$inside/alice.key" \
  OCAF_LINUX_GUI_BINARY=/usr/bin/ocaf-agents-builder \
  OCAF_LINUX_GUI_DRIVER=/opt/ocaf-test/tauri-driver \
  dbus-run-session -- xvfb-run -a -s '-screen 0 1920x1200x24' \
  python3 /opt/ocaf-test/verify_builder_linux_cloud_gui.py
