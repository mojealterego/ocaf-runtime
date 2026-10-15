#!/usr/bin/env bash
# Single-use CI-only pinned Firecracker root registry for stage 0.39.
set -euo pipefail
if [[ "$#" != 2 || ! "$1" =~ ^[0-9]{6,15}$ || ! "$2" =~ ^[ab]$ ]]; then
  echo 'Expected trusted GitHub Actions run ID and test role a|b' >&2
  exit 2
fi
RUN_ID="$1"
ROLE="$2"
SOURCE="/tmp/ocaf39-cold-src-${RUN_ID}-${ROLE}"
PREFIX="/opt/o39c${RUN_ID}-${ROLE}"
umask 077
mkdir -p "$SOURCE"
cd "$SOURCE"
curl --fail --silent --show-error --location --retry 2 --max-time 120 \
  --proto '=https' --tlsv1.2 \
  https://github.com/firecracker-microvm/firecracker/releases/download/v1.17.0/firecracker-v1.17.0-x86_64.tgz \
  -o firecracker-v1.17.0-x86_64.tgz
echo '06094a1108ae9e82aa4c23a775aa92758f53f1175d422270d9d6162cb9ade558  firecracker-v1.17.0-x86_64.tgz' | sha256sum -c -
tar -xzf firecracker-v1.17.0-x86_64.tgz
curl --fail --silent --show-error --location --retry 2 --max-time 120 \
  --proto '=https' --tlsv1.2 \
  https://s3.amazonaws.com/spec.ccfc.min/img/hello/kernel/hello-vmlinux.bin \
  -o hello-vmlinux.bin
curl --fail --silent --show-error --location --retry 2 --max-time 120 \
  --proto '=https' --tlsv1.2 \
  https://s3.amazonaws.com/spec.ccfc.min/img/hello/fsfiles/hello-rootfs.ext4 \
  -o hello-rootfs.ext4
echo '882fa465c43ab7d92e31bd4167da3ad6a82cb9230f9b0016176df597c6014cef  hello-vmlinux.bin' | sha256sum -c -
echo '786f0612582cadcee4c8ad46e30e4cc93dff9a3c0b4ede3f3c597b4c241dd547  hello-rootfs.ext4' | sha256sum -c -
sudo -n install -d -o root -g root -m 0700 "$PREFIX" "$PREFIX/approved" "$PREFIX/jailer"
sudo -n install -o root -g root -m 0555 \
  "$SOURCE/release-v1.17.0-x86_64/firecracker-v1.17.0-x86_64" \
  "$PREFIX/approved/firecracker"
sudo -n install -o root -g root -m 0555 \
  "$SOURCE/release-v1.17.0-x86_64/jailer-v1.17.0-x86_64" \
  "$PREFIX/approved/jailer"
sudo -n install -o root -g root -m 0444 "$SOURCE/hello-vmlinux.bin" "$PREFIX/approved/kernel"
sudo -n install -o root -g root -m 0444 "$SOURCE/hello-rootfs.ext4" "$PREFIX/approved/rootfs"
echo "OCAF039_COLD_HOST_PREPARED: $ROLE"
