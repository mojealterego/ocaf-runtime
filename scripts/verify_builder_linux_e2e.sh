#!/usr/bin/env bash
# Linux engineering acceptance: real installed ELF, real GUI and mTLS backend.
# A temporary lab key exercises signing; it is NEVER an operator release key.
set -euo pipefail
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)"
cd "$ROOT"
if [[ $# != 3 || ! "$1" =~ ^[0-9a-f]{40}$ ]]; then
  echo "usage: $0 <expected-main-sha> <new-absolute-evidence-directory> <tauri-driver>" >&2
  exit 2
fi
sha="$1"
out="$2"
driver="$(realpath "$3")"
[[ "$out" == /* && -x "$driver" && "$(git symbolic-ref HEAD)" == refs/heads/main && "$(git rev-parse HEAD)" == "$sha" ]]
git diff --exit-code HEAD --
umask 077
mkdir -m 700 -- "$out"
mkdir -m 700 "$out/install"
desktop_target="$(realpath -m "${OCAF_DESKTOP_TARGET_DIR:-$ROOT/apps/agents-builder-desktop/src-tauri/target}")"
runtime_target="$(realpath -m "${OCAF_RUNTIME_TARGET_DIR:-$ROOT/target}")"
manifest=apps/agents-builder-desktop/src-tauri/Cargo.toml
{
  printf 'source_sha=%s\n' "$sha"
  printf 'tree_sha=%s\n' "$(git rev-parse 'HEAD^{tree}')"
  rustc --version; cargo --version; python3 --version; node --version; openssl version
  uname -sm
  pkg-config --modversion webkit2gtk-4.1
} > "$out/environment.txt"
python3 scripts/generate_builder_icons.py
CARGO_TARGET_DIR="$desktop_target" cargo build --locked --release --manifest-path "$manifest" > "$out/build.log" 2>&1
install -m 0755 "$desktop_target/release/ocaf-agents-builder-desktop" "$out/install/ocaf-agents-builder-desktop"
binary="$out/install/ocaf-agents-builder-desktop"
python3 scripts/verify_native_artifact.py linux "$binary" > "$out/install.log"
ldd "$binary" > "$out/ldd.txt"
if grep -q 'not found' "$out/ldd.txt"; then echo 'Missing shared library' >&2; exit 1; fi
CARGO_TARGET_DIR="$desktop_target" cargo test --locked --release --manifest-path "$manifest" > "$out/desktop-tests.log" 2>&1
node --test apps/agents-builder-web/tests/*.test.mjs > "$out/web-tests.log" 2>&1
xvfb-run -a -s '-screen 0 1920x1200x24' python3 scripts/verify_builder_linux_local_gui.py --binary "$binary" --driver "$driver" > "$out/local-gui.json"
CARGO_TARGET_DIR="$runtime_target" OCAF_LINUX_GUI_BINARY="$binary" OCAF_LINUX_GUI_DRIVER="$driver" \
  xvfb-run -a -s '-screen 0 1920x1200x24' cargo test --locked --no-default-features --test builder_cloud_full_stack -- --nocapture > "$out/cloud-stack.log" 2>&1
python3 scripts/generate_sbom.py --repo "$ROOT" --lockfile "$ROOT/apps/agents-builder-desktop/src-tauri/Cargo.lock" --output "$out/linux-lockfile.cdx.json" > "$out/sbom-generation.log"
python3 - "$out" "$sha" <<'PY'
import hashlib, json, sys
from pathlib import Path
out=Path(sys.argv[1])
sha=sys.argv[2]
local=json.loads((out/'local-gui.json').read_text())
cloud=[json.loads(line) for line in (out/'cloud-stack.log').read_text().splitlines() if line.startswith('{"binary_sha256"')]
assert len(cloud)==1
cloud=cloud[0]
digest=lambda p: hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
binary=digest(out/'install/ocaf-agents-builder-desktop')
assert local['result']==cloud['result']=='PASS'
assert local['binary_sha256']==cloud['binary_sha256']==binary
for key in ('native_rust_compile','code_nocode_export_import','no_overwrite','persistent_restart'):
    assert local[key] is True
for key in ('mtls_oauth','foreign_tenant_denied','foreign_import_denied','code_nocode_cas','code_nocode_export_import','local_isolation','restart_recovery'):
    assert cloud[key] is True
(out/'cloud-gui.json').write_text(json.dumps(cloud,sort_keys=True)+'\n')
paths=['install/ocaf-agents-builder-desktop','environment.txt','build.log','install.log','ldd.txt',
       'desktop-tests.log','web-tests.log','local-gui.json','cloud-gui.json','cloud-stack.log',
       'linux-lockfile.cdx.json','sbom-generation.log']
data={'schema_version':1,'platform':'linux','distribution':'linux-elf','source_sha':sha,
      'status':'ENGINEERING_E2E_PASS','production_approved':False,'signature_scope':'EPHEMERAL_LAB_ONLY',
      'files':{p:digest(out/p) for p in paths}}
(out/'LINUX_ENGINEERING_EVIDENCE.json').write_text(json.dumps(data,sort_keys=True,separators=(',',':'))+'\n')
PY
# Generate an ephemeral local key for the engineering signing roundtrip only.
# The private material never enters a published artifact or source commit.
keydir="$(mktemp -d)"
trap 'rm -f "$keydir/key.pem" "$keydir/tampered.json"; rmdir "$keydir"' EXIT
openssl genpkey -algorithm Ed25519 -out "$keydir/key.pem"
openssl pkey -in "$keydir/key.pem" -pubout -out "$out/lab-public.pem"
openssl pkeyutl -sign -rawin -inkey "$keydir/key.pem" -in "$out/LINUX_ENGINEERING_EVIDENCE.json" -out "$out/lab-signature.bin"
openssl pkeyutl -verify -rawin -pubin -inkey "$out/lab-public.pem" -in "$out/LINUX_ENGINEERING_EVIDENCE.json" -sigfile "$out/lab-signature.bin" > "$out/signature-test.log"
cp "$out/LINUX_ENGINEERING_EVIDENCE.json" "$keydir/tampered.json"
printf 'tamper\n' >> "$keydir/tampered.json"
if openssl pkeyutl -verify -rawin -pubin -inkey "$out/lab-public.pem" -in "$keydir/tampered.json" -sigfile "$out/lab-signature.bin" >> "$out/signature-test.log" 2>&1; then
  echo 'tampered evidence was accepted' >&2; exit 1
fi
printf 'tamper_rejected=true\n' >> "$out/signature-test.log"
[[ "$(git rev-parse HEAD)" == "$sha" ]]
git diff --exit-code HEAD --
sha256sum "$out/LINUX_ENGINEERING_EVIDENCE.json" "$out/install/ocaf-agents-builder-desktop" "$out/linux-lockfile.cdx.json"
printf 'LINUX_ENGINEERING_E2E_PASS (lab signing only; no production approval)\n'
