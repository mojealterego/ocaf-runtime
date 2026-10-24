#!/usr/bin/env bash
# GitHub-hosted privileged KVM lab only. Prepare TWO digest-bound, distinct
# tenant WASIp1 guests from the verified official Linux ext4 image.
set -euo pipefail
if [[ "$#" -ne 2 || "$EUID" -ne 0 ]]; then
  echo "usage: sudo bash prepare_guest_wasi.sh /opt/private-ocaf-prefix /absolute-workspace" >&2
  exit 2
fi
PREFIX="$1"; SRC="$2"
[[ "$PREFIX" == /opt/ocaf039-real-* && "$SRC" == /* && -d "$SRC/stage039/guest-wasi" ]] || exit 2
APPROVED="$PREFIX/approved"
[[ -f "$APPROVED/guest_rootfs" && -f "$APPROVED/request.json" ]] || exit 2
WORK="/tmp/ocaf39-real-guest-wasi-$(basename "$PREFIX")"
mkdir -p "$WORK" "$WORK/mount"
chmod 0700 "$WORK" "$WORK/mount"
cleanup() {
  if mountpoint -q "$WORK/mount"; then umount "$WORK/mount" || true; fi
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
[[ -x "$WASMTIME" && -x /usr/bin/sha256sum ]] || { echo "trusted guest tooling missing" >&2; exit 3; }
"$WASMTIME" --version
bash "$SRC/stage039/guest-aya/build.sh" "$WORK/aya-build"
install -m 0555 "$WORK/aya-build/ocaf-guest-openat-aya" \
  "$WORK/ocaf-guest-openat-probe"
cmp "$WORK/aya-build/ocaf-guest-openat-aya" "$WORK/ocaf-guest-openat-probe"
install -o root -g root -m 0555 "$WASMTIME" "$APPROVED/wasmtime_runtime"
install -o root -g root -m 0555 "$WORK/ocaf-guest-openat-probe" "$APPROVED/guest_openat_probe"
cmp "$WASMTIME" "$APPROVED/wasmtime_runtime"
cmp "$WORK/ocaf-guest-openat-probe" "$APPROVED/guest_openat_probe"

# Build real negative WASIp1 modules and the static, bounded guest supervisor.
# The resulting bytes become part of each signed, read-only tenant rootfs.
echo 'c2cde9d58faa091dd504e732a216677535fb662f5b4345f3c363924900d73494  '"$SRC/stage039/guest-wasi-negative/fuel_spin.wat" | sha256sum --check -
echo '932cf0d95f147d552b8580a0a0b7726abea7904e7ed7f7a68eaf04af3357b3a1  '"$SRC/stage039/guest-wasi-negative/memory_grow.wat" | sha256sum --check -
wat2wasm "$SRC/stage039/guest-wasi-negative/fuel_spin.wat" -o "$WORK/negative-spin.wasm"
wat2wasm "$SRC/stage039/guest-wasi-negative/memory_grow.wat" -o "$WORK/negative-memory.wasm"
wasm-objdump -x "$WORK/negative-spin.wasm" | grep -F 'wasi_snapshot_preview1'
wasm-objdump -x "$WORK/negative-memory.wasm" | grep -F 'wasi_snapshot_preview1'
gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
  "$SRC/stage039/guest-wasi-negative/guest_negative_probe.c" \
  -o "$WORK/ocaf-guest-negative-probe"

# Install the binaries and their actual ELF loader/dependencies into an
# official-rootfs COPY. Each tenant receives its own later copy.
install_guest_binary() {
  local binary="$1" destination="$2" line part p
  install -m 0555 "$binary" "$WORK/mount$destination"
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
  done < <(ldd "$binary")
}

BASE="$WORK/guest-rootfs-wasi-base.ext4"
cp "$APPROVED/guest_rootfs" "$BASE"
truncate -s +384M "$BASE"
e2fsck -f -y "$BASE" || [[ "$?" -eq 1 ]]
resize2fs "$BASE"
mount -o loop,rw "$BASE" "$WORK/mount"
mkdir -p "$WORK/mount/usr/bin" "$WORK/mount/sbin" "$WORK/mount/opt/ocaf" \
  "$WORK/mount/sys/kernel/tracing" "$WORK/mount/sys/kernel/debug" "$WORK/mount/proc"
install_guest_binary "$WASMTIME" /usr/bin/wasmtime
install_guest_binary /usr/bin/sha256sum /usr/bin/sha256sum
install -m 0555 "$WORK/ocaf-guest-openat-probe" "$WORK/mount/usr/bin/ocaf-guest-openat-probe"
install -m 0555 "$WORK/ocaf-guest-negative-probe" "$WORK/mount/usr/bin/ocaf-guest-negative-probe"
install -m 0444 "$WORK/negative-spin.wasm" "$WORK/mount/opt/ocaf/negative-spin.wasm"
install -m 0444 "$WORK/negative-memory.wasm" "$WORK/mount/opt/ocaf/negative-memory.wasm"
sync
umount "$WORK/mount"

nonce_a=""
for key in a b; do
  tenant="tenant-$key"
  nonce="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
  [[ "$nonce" =~ ^[0-9a-f]{32}$ && "$nonce" != "$nonce_a" ]] || exit 3
  if [[ "$key" == a ]]; then nonce_a="$nonce"; fi
  python3 "$SRC/stage039/guest-wasi/build_tenant_module.py" \
    "$tenant" "$nonce" "$WORK/hello-$key.wat"
  wat2wasm "$WORK/hello-$key.wat" -o "$WORK/hello-$key.wasm"
  wasm-objdump -x "$WORK/hello-$key.wasm" | grep -F 'wasi_snapshot_preview1'
  # Host execution checks module construction only. Guest serial is the sole
  # accepted evidence of execution inside Firecracker/KVM.
  expected="OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE:$tenant:$nonce"
  output="$("$WASMTIME" run -C cache=no "$WORK/hello-$key.wasm")"
  [[ "$output" == "$expected" ]] || { echo "WASIp1 module preflight mismatch" >&2; exit 4; }
  wasm_sha="$(sha256sum "$WORK/hello-$key.wasm" | cut -d ' ' -f 1)"
  gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
    "-DOCAF_WASI_TENANT=\"$tenant\"" \
    "-DOCAF_WASI_NONCE=\"$nonce\"" \
    "-DOCAF_WASI_MODULE_SHA256=\"$wasm_sha\"" \
    "$SRC/stage039/guest-wasi/guest_wasi_init.c" -o "$WORK/ocaf-wasi-init-$key"
  image="$WORK/guest-rootfs-wasi-$key.ext4"
  cp "$BASE" "$image"
  mount -o loop,rw "$image" "$WORK/mount"
  # Replace PID1 only inside this disposable, digest-bound tenant image.
  rm -f "$WORK/mount/sbin/init"
  install -m 0555 "$WORK/ocaf-wasi-init-$key" "$WORK/mount/sbin/init"
  install -m 0555 "$WORK/ocaf-wasi-init-$key" "$WORK/mount/ocaf-wasi-init"
  install -m 0444 "$WORK/hello-$key.wasm" "$WORK/mount/opt/ocaf/hello.wasm"
  sync
  umount "$WORK/mount"
  debugfs -R "dump /ocaf-wasi-init $WORK/verified-guest-init-$key" "$image" >/dev/null 2>&1
  cmp "$WORK/ocaf-wasi-init-$key" "$WORK/verified-guest-init-$key"
  debugfs -R "dump /opt/ocaf/hello.wasm $WORK/verified-guest-wasm-$key" "$image" >/dev/null 2>&1
  cmp "$WORK/hello-$key.wasm" "$WORK/verified-guest-wasm-$key"
  debugfs -R "dump /usr/bin/ocaf-guest-openat-probe $WORK/verified-guest-openat-$key" "$image" >/dev/null 2>&1
  cmp "$WORK/ocaf-guest-openat-probe" "$WORK/verified-guest-openat-$key"
  for negative in negative-spin.wasm negative-memory.wasm; do
    debugfs -R "dump /opt/ocaf/$negative $WORK/verified-$key-$negative" "$image" >/dev/null 2>&1
    cmp "$WORK/$negative" "$WORK/verified-$key-$negative"
  done
  debugfs -R "dump /usr/bin/ocaf-guest-negative-probe $WORK/verified-guest-negative-$key" "$image" >/dev/null 2>&1
  cmp "$WORK/ocaf-guest-negative-probe" "$WORK/verified-guest-negative-$key"
  install -o root -g root -m 0444 "$WORK/hello-$key.wasm" "$APPROVED/wasi_module_$key"
  cmp "$WORK/hello-$key.wasm" "$APPROVED/wasi_module_$key"
  install -o root -g root -m 0444 "$image" "$APPROVED/guest_rootfs_wasi_$key"
  python3 - "$APPROVED" "$key" "$nonce" "$wasm_sha" <<'PY'
import hashlib, json, sys
from pathlib import Path

root, key, nonce, wasm_sha = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
image = root / f"guest_rootfs_wasi_{key}"
with image.open("rb") as stream:
    image_sha = hashlib.file_digest(stream, "sha256").hexdigest()
manifest = json.loads((root / "request.json").read_text())
manifest["tenant_id"] = f"tenant-{key}"
manifest["agent_id"] = f"agent-{key}"
manifest["workload_id"] = f"ocaf-fc-wasi-{key}"
manifest["isolation"]["mem_mib"] = 1024
manifest["assets"]["guest_rootfs"] = {"path": str(image), "sha256": image_sha}
manifest["wasi"] = {"task_nonce": nonce, "module_sha256": wasm_sha}
for asset, filename in (("wasmtime_runtime", "wasmtime_runtime"),
                        ("guest_openat_probe", "guest_openat_probe"),
                        ("wasi_module", f"wasi_module_{key}")):
    file = root / filename
    with file.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    manifest["assets"][asset] = {"path": str(file), "sha256": digest}
assert manifest["assets"]["wasi_module"]["sha256"] == wasm_sha
path = root / f"request-wasi-{key}.json"
path.write_text(json.dumps(manifest, sort_keys=True) + "\n")
path.chmod(0o600)
print("STAGE039_REAL_GUEST_WASI_IMAGE_PINNED", key, image_sha, wasm_sha)
PY
done
echo "STAGE039_TWO_DISTINCT_REAL_GUEST_WASI_IMAGES_PREPARED"
