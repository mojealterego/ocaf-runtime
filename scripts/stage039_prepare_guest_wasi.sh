#!/usr/bin/env bash
# GitHub-hosted privileged KVM lab only. Clone the verified public guest ext4
# and install pinned Wasmtime v49.0.3, dynamic libs and static WASI-aware PID1.
# Leave the original official image completely unchanged.
set -euo pipefail
if [[ "$#" -ne 2 || "$EUID" -ne 0 ]]; then
  echo "usage: sudo bash prepare_guest_wasi.sh /opt/private-ocaf-prefix /absolute-workspace" >&2
  exit 2
fi
PREFIX="$1"; SRC="$2"
[[ "$PREFIX" == /opt/ocaf039-real-* && "$SRC" == /* && -d "$SRC/stage039/guest-wasi" ]] || exit 2
APPROVED="$PREFIX/approved"
[[ -f "$APPROVED/guest_rootfs" && -f "$APPROVED/request.json" ]] || exit 2
WORK="/tmp/ocaf39-real-guest-wasi-${GITHUB_RUN_ID:?}"
mkdir -p "$WORK" "$WORK/mount"
chmod 0700 "$WORK" "$WORK/mount"
cleanup() {
  if mountpoint -q "$WORK/mount";then umount "$WORK/mount" || true; fi
}
trap cleanup EXIT
ARCHIVE="$WORK/wasmtime-v49.0.3-x86_64-linux.tar.xz"
curl --fail --silent --show-error --location --retry 2 --max-time 120 \
 --proto '=https' --tlsv1.2 \
 https://github.com/bytecodealliance/wasmtime/releases/download/v49.0.3/wasmtime-v49.0.3-x86_64-linux.tar.xz \
 -o "$ARCHIVE"
echo '4001225413f208345b980525f31c4080e1873808b38d0ddc7918497340667fbb  '"$ARCHIVE" | sha256sum --check -
tar -xJf "$ARCHIVE" -C "$WORK"
WASMTIME="$WORK/wasmtime-v49.0.3-x86_64-linux/wasmtime"
[[ -x "$WASMTIME" ]] || { echo "trusted upstream Wasmtime executable missing";exit 3; }
file "$WASMTIME"
"$WASMTIME" --version
gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
 "$SRC/stage039/guest-wasi/guest_wasi_init.c" -o "$WORK/ocaf-wasi-init"
wat2wasm "$SRC/stage039/guest-wasi/hello.wat" -o "$WORK/hello.wasm"
wasm-objdump -x "$WORK/hello.wasm" | grep -F 'wasi_snapshot_preview1'
# Independently verify WASI import even on HOST before any Firecracker VM starts.
# This is a preflight; ONLY guest serial is counted as KVM execution.
output="$("$WASMTIME" run "$WORK/hello.wasm")"
[[ "$output" == 'OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE' ]] || exit 4
cp "$APPROVED/guest_rootfs" "$WORK/guest-rootfs-wasi.ext4"
truncate -s +384M "$WORK/guest-rootfs-wasi.ext4"
e2fsck -f -y "$WORK/guest-rootfs-wasi.ext4" || [[ "$?" -eq 1 ]]
resize2fs "$WORK/guest-rootfs-wasi.ext4"
mount -o loop,rw "$WORK/guest-rootfs-wasi.ext4" "$WORK/mount"
mkdir -p "$WORK/mount/usr/bin" "$WORK/mount/sbin" "$WORK/mount/opt/ocaf"
install -m 0555 "$WASMTIME" "$WORK/mount/usr/bin/wasmtime"
# Copy the *actual* dynamic loader and all dependencies of the pinned
# CLI into the guest, with their original ELF absolute paths.
while IFS= read -r line; do
  for part in $line; do
    case "$part" in
      /*)
        p="${part%%(*}"
        if [[ -f "$p" ]]; then
          mkdir -p "$WORK/mount$(dirname "$p")"
          install -m 0555 "$p" "$WORK/mount$p"
        fi;;
    esac
  done
done < <(ldd "$WASMTIME")
# Kernel defaults to /sbin/init when booting readonly rootfs.
# Replace only inside this disposable, digest-bound private copy.
rm -f "$WORK/mount/sbin/init"
install -m 0555 "$WORK/ocaf-wasi-init" "$WORK/mount/sbin/init"
install -m 0444 "$WORK/hello.wasm" "$WORK/mount/opt/ocaf/hello.wasm"
sync
umount "$WORK/mount"
install -o root -g root -m 0444 "$WORK/guest-rootfs-wasi.ext4" "$APPROVED/guest_rootfs_wasi"
sha256sum "$APPROVED/guest_rootfs_wasi" > "$WORK/rootfs.sha256"
sha256sum "$WORK/hello.wasm" > "$WORK/wasm.sha256"
sha256sum "$WASMTIME" > "$WORK/wasmtime.sha256"
python3 - "$APPROVED" <<'PY'
import json,sys,hashlib
from pathlib import Path
root=Path(sys.argv[1]);image=root/"guest_rootfs_wasi"
sha=hashlib.sha256(image.read_bytes()).hexdigest()
for name,tenant,agent,workload in (
 ("request-wasi-a.json","tenant-a","agent-a","ocaf-fc-wasi-a"),
 ("request-wasi-b.json","tenant-b","agent-b","ocaf-fc-wasi-b")):
    manifest=json.loads((root/"request.json").read_text())
    manifest["tenant_id"]=tenant;manifest["agent_id"]=agent
    manifest["workload_id"]=workload
    manifest["isolation"]["mem_mib"]=1024
    manifest["assets"]["guest_rootfs"]={"path":str(image),"sha256":sha}
    f=root/name
    f.write_text(json.dumps(manifest,sort_keys=True)+"\n")
    f.chmod(0o600)
print("STAGE039_REAL_GUEST_WASI_IMAGE_PINNED",sha)
PY
echo "STAGE039_REAL_GUEST_WASI_IMAGE_PREPARED"
