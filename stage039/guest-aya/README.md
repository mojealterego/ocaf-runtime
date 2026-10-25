# Aya guest `openat` probe for stage 0.39.27

This is a candidate replacement for the raw eBPF helper already tested in
Firecracker. The `ebpf` crate uses **Aya eBPF** to count actual
`syscalls/sys_enter_openat` events from one exact Wasmtime TGID. The `loader`
crate uses **Aya** inside the Linux guest to load that ELF, set the target
before the child is released, attach the tracepoint and read the kernel map.

The loader uses a statically linked x86_64 musl executable. It embeds the
compiled eBPF ELF and checks its SHA-256 before kernel load. Its first two
arguments retain the existing guest helper ABI:

```text
/usr/bin/ocaf-guest-openat-probe TARGET_TGID TIMEOUT_SECONDS [MIN_EVENTS]
```

It prints exactly these lines on success, with values read from the guest
kernel and Aya map:

```text
OCAF039_GUEST_OPENAT_PROBE_READY_ID=<positive kernel program ID>
OCAF039_GUEST_OPENAT_EVENT_ID=<same ID> COUNT=<positive> TGID=<target> PID=<positive>
```

PID1 must still hold the intended Wasmtime child behind a pipe until READY,
then require EVENT plus the child's exact WASIp1 output and exit code. Any
Aya load, attach, map, digest or timeout failure exits nonzero without a
success receipt. The target guest kernel is the SHA-pinned official
Firecracker 6.1.188 debug image with syscall tracepoints.
`MIN_EVENTS` defaults to one and is bounded to 1–16; the quarantine lab asks
for two kernel events before it can request a signed stop permit.

Run `bash stage039/guest-aya/build.sh /absolute/output/directory` on Linux
x86_64. The script pins Rust 1.99.0, nightly-2026-10-24, Aya 0.14.0,
Aya eBPF 0.2.1 and the bpf-linker 0.12.0 release archive by SHA-256. It
produces the static loader, separate eBPF ELF and their SHA manifest.

`scripts/stage039_prepare_guest_wasi.sh` builds the loader and installs it as
`/usr/bin/ocaf-guest-openat-probe` in each tenant's read-only rootfs. The
eBPF ELF is embedded in that loader and checked against its compile-time
SHA-256 before kernel load; the signed workload manifest pins the complete
loader separately from the rootfs. The build also exports the standalone ELF
and both digests for inspection. Real KVM
[run 40126628090](https://github.com/mojealterego/ocaf-runtime/actions/runs/40126628090)
proved guest Aya acceptance in two jailed VM kernels, with positive program
IDs and Wasmtime `openat` counts. The additional two-event quarantine path
introduced here still requires a real KVM run. Production acceptance and the
full milestone remain open.
