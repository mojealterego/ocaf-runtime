#!/usr/bin/env bash
# Real installed Debian package against Kali userland, shared laboratory kernel.
# Requires a disposable, pre-provisioned Kali rootfs with GUI dependencies and
# unprivileged ocaf account matching the invoking UID/GID. No production signing.
set -euo pipefail
repo="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
cd "$repo"
[[ $# == 5 && "$1" =~ ^[a-f0-9]{40}$ ]] || {
  echo "usage: $0 <main-sha> <Kali-rootfs> <new-absolute-evidence-dir> <verified-ELF> <tauri-driver>" >&2; exit 2;
}
sha="$1"; root="$(realpath "$2")"; out="$3"; binary="$(realpath "$4")"; driver="$(realpath "$5")"
case "$root" in /tmp/ocaf-kali-e2e.*/rootfs) ;; *) exit 2;; esac
[[ -O "$root" && -d "$root" && -f "$binary" && -x "$driver" && "$out" == /* && "$(id -u)" != 0 ]]
grep -qx 'ID=kali' "$root/etc/os-release"
[[ "$(git symbolic-ref HEAD)" == refs/heads/main && "$(git rev-parse HEAD)" == "$sha" ]]
git diff --exit-code HEAD --
umask 077
mkdir -m 700 -- "$out"
enter="$repo/scripts/verify_builder_kali_namespace.sh"
kali() { sudo -n unshare -mpf --kill-child "$enter" "$root" "$@"; }
gui() {
  kali /usr/bin/setpriv --reuid "$(id -u)" --regid "$(id -g)" --clear-groups \
    /usr/bin/env HOME=/home/ocaf LANG=C.UTF-8 USER=ocaf \
    dbus-run-session -- xvfb-run -a -s '-screen 0 1920x1200x24' "$@"
}
[[ -d "$root/home/ocaf" && -O "$root/home/ocaf" && ! -L "$root/home/ocaf" ]]
mkdir -p "$root/opt/ocaf-test"
for f in verify_builder_linux_local_gui.py verify_builder_linux_cloud_gui.py; do
  install -m 644 "scripts/$f" "$root/opt/ocaf-test/$f"
done
install -m 755 "$driver" "$root/opt/ocaf-test/tauri-driver"
python3 scripts/generate_builder_icons.py
icon=apps/agents-builder-desktop/src-tauri/icons/icon.png
python3 scripts/package_builder_kali.py --binary "$binary" --icon "$icon" --out "$out/ocaf-kali.deb" --version 0.1.0 --commit "$sha" > "$out/package.json"
python3 scripts/package_builder_kali.py --binary "$binary" --icon "$icon" --out "$out/repeat.deb" --version 0.1.0 --commit "$sha" > "$out/repeat-package.json"
cmp "$out/ocaf-kali.deb" "$out/repeat.deb"
rm -- "$out/repeat.deb"
package="$(mktemp "$root/tmp/ocaf-package.XXXXXXXX")"
cleanup() { rm -f -- "$package"; }
trap cleanup EXIT
install -m 644 "$out/ocaf-kali.deb" "$package"
kali /usr/bin/dpkg -i "${package#"$root"}" > "$out/install.log" 2>&1
kali /bin/cat /etc/os-release > "$out/os-release.txt"
kali /usr/bin/dpkg-query -W '-f=${Package} ${Version} ${Status}\n' ocaf-agents-builder > "$out/install-status.txt"
grep -Fx 'ocaf-agents-builder 0.1.0 install ok installed' "$out/install-status.txt"
kali /usr/bin/sha256sum /usr/bin/ocaf-agents-builder > "$out/installed-binary.sha256"
[[ "$(cut -d ' ' -f1 "$out/installed-binary.sha256")" == "$(sha256sum "$binary" | cut -d ' ' -f1)" ]]
kali /usr/bin/ldd /usr/bin/ocaf-agents-builder > "$out/ldd.txt"
! grep -q 'not found' "$out/ldd.txt" || exit 1
kali /usr/bin/dpkg-query -W '-f=${Package}\t${Version}\t${Status}\n' > "$out/system-packages.tsv"
python3 -m unittest discover -s scripts/tests -p test_package_builder_kali.py -v > "$out/package-tests.log" 2>&1
python3 -m unittest discover -s scripts/tests -p test_builder_bundle_platform_coverage.py -v > "$out/bundle-tests.log" 2>&1
gui python3 /opt/ocaf-test/verify_builder_linux_local_gui.py --binary /usr/bin/ocaf-agents-builder --driver /opt/ocaf-test/tauri-driver > "$out/local-gui.log" 2>&1
OCAF_KALI_ROOTFS="$root" OCAF_KALI_GUI_RUNNER="$repo/scripts/verify_builder_kali_cloud_adapter.sh" \
 OCAF_LINUX_GUI_BINARY=/usr/bin/ocaf-agents-builder OCAF_LINUX_GUI_DRIVER=/opt/ocaf-test/tauri-driver \
 CARGO_TARGET_DIR="${OCAF_RUNTIME_TARGET_DIR:-$repo/target}" \
 cargo test --locked --no-default-features --test builder_cloud_full_stack -- --nocapture > "$out/cloud-gui.log" 2>&1
python3 scripts/generate_sbom.py --repo "$repo" --lockfile "$repo/apps/agents-builder-desktop/src-tauri/Cargo.lock" --output "$out/kali-lockfile.cdx.json" > "$out/sbom-generation.log"
python3 - "$out" "$sha" <<'PY'
import hashlib,json,sys
from pathlib import Path
out=Path(sys.argv[1])
def receipt(name):
    hits=[json.loads(line) for line in (out/name).read_text().splitlines()
          if line.startswith('{') and '"binary_sha256"' in line]
    assert len(hits)==1
    return hits[0]
local=receipt('local-gui.log'); cloud=receipt('cloud-gui.log')
package=json.loads((out/'package.json').read_text())
assert local['result']==cloud['result']=='PASS'
assert package['binary_sha256']==local['binary_sha256']==cloud['binary_sha256']
for key in ('native_rust_compile','code_nocode_export_import','no_overwrite','persistent_restart'):
    assert local[key] is True
for key in ('mtls_oauth','foreign_tenant_denied','foreign_import_denied','code_nocode_cas','code_nocode_export_import','local_isolation','restart_recovery'):
    assert cloud[key] is True
assert 'ID=kali' in (out/'os-release.txt').read_text().splitlines()
assert package['source_commit']==sys.argv[2]
digest=lambda p: hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
assert digest(out/'ocaf-kali.deb')==package['package_sha256']
(out/'local-gui.json').write_text(json.dumps(local,sort_keys=True)+'\n')
(out/'cloud-gui.json').write_text(json.dumps(cloud,sort_keys=True)+'\n')
manifest={'schema_version':1,'platform':'kali-linux','scope':'REAL_KALI_USERLAND_SHARED_LAB_KERNEL',
 'distribution':'kali-deb-amd64','source_sha':sys.argv[2],'status':'ENGINEERING_E2E_PASS',
 'production_approved':False,'signature_scope':'EPHEMERAL_LAB_ONLY',
 'files':{p.name:digest(p) for p in sorted(out.iterdir()) if p.is_file()}}
(out/'KALI_ENGINEERING_EVIDENCE.json').write_text(json.dumps(manifest,sort_keys=True,separators=(',',':'))+'\n')
PY
keydir="$(mktemp -d)"
trap 'cleanup; rm -f -- "$keydir/key.pem" "$keydir/tampered.json"; rmdir -- "$keydir"' EXIT
openssl genpkey -algorithm Ed25519 -out "$keydir/key.pem"
openssl pkey -in "$keydir/key.pem" -pubout -out "$out/lab-public.pem"
openssl pkeyutl -sign -rawin -inkey "$keydir/key.pem" -in "$out/KALI_ENGINEERING_EVIDENCE.json" -out "$out/lab-signature.bin"
openssl pkeyutl -verify -rawin -pubin -inkey "$out/lab-public.pem" -in "$out/KALI_ENGINEERING_EVIDENCE.json" -sigfile "$out/lab-signature.bin" > "$out/signature-test.log"
cp "$out/KALI_ENGINEERING_EVIDENCE.json" "$keydir/tampered.json"
printf 'tamper\n' >> "$keydir/tampered.json"
if openssl pkeyutl -verify -rawin -pubin -inkey "$out/lab-public.pem" -in "$keydir/tampered.json" -sigfile "$out/lab-signature.bin" >> "$out/signature-test.log" 2>&1; then
  echo 'tampered evidence was accepted' >&2; exit 1
fi
printf 'tamper_rejected=true\n' >> "$out/signature-test.log"
[[ "$(git rev-parse HEAD)" == "$sha" ]]
git diff --exit-code HEAD --
sha256sum "$out/ocaf-kali.deb" "$out/kali-lockfile.cdx.json" "$out/KALI_ENGINEERING_EVIDENCE.json"
printf 'KALI_ENGINEERING_E2E_PASS (Kali userland; lab signing only)\n'
