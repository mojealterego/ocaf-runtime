#!/bin/bash
# Only a caller-created isolated Kali rootfs is accepted.
set -euo pipefail
[[ $(id -u) == 0 ]]
root="$(realpath "$1")"; shift
case "$root" in /tmp/ocaf-kali-e2e.*/rootfs) ;; *) exit 2;; esac
grep -qx 'ID=kali' "$root/etc/os-release"
mount --make-rprivate /
mount -t proc proc "$root/proc"
mount -t tmpfs -o mode=755,nosuid tmpfs "$root/dev"
for spec in 'null 1 3' 'zero 1 5' 'random 1 8' 'urandom 1 9' 'tty 5 0' 'full 1 7'; do
    read -r name major minor <<< "$spec"
    mknod -m 666 "$root/dev/$name" c "$major" "$minor"
done
mkdir -p "$root/dev/shm" "$root/dev/pts"
mount -t tmpfs -o mode=1777,nosuid,nodev tmpfs "$root/dev/shm"
mount -t devpts -o newinstance,ptmxmode=0666,mode=0620 devpts "$root/dev/pts"
ln -s pts/ptmx "$root/dev/ptmx"
ln -s /proc/self/fd "$root/dev/fd"
ln -s /proc/self/fd/0 "$root/dev/stdin"; ln -s /proc/self/fd/1 "$root/dev/stdout"; ln -s /proc/self/fd/2 "$root/dev/stderr"
mount -t tmpfs -o mode=755,nosuid,nodev tmpfs "$root/run"
mount -t sysfs -o ro,nosuid,nodev,noexec sysfs "$root/sys" || true
exec chroot "$root" "$@"
