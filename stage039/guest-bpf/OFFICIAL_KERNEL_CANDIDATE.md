# Official Firecracker tracing kernel candidate

The existing stage 0.39 KVM workflow pins the 2018 `hello-vmlinux.bin` guest
kernel (`sha256:882fa465c43ab7d92e31bd4167da3ad6a82cb9230f9b0016176df597c6014cef`).
Inspection of that exact unstripped ELF shows `__start___tracepoints_ptrs` and
`__stop___tracepoints_ptrs` at the same address (`ffffffff81b2faf8`) and no
`__tracepoint_*` or `perf_trace_sys_enter` symbols. It cannot currently prove
an in-guest `sys_enter_openat` BPF tracepoint.

Firecracker publishes CI guest kernels through its [getting started guide](
https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md).
The 2026-10-23 **debug** x86_64 CI kernel is a tracing-capable candidate:

| Asset | Public source | SHA-256 |
|---|---|---|
| Linux 6.1.188 ELF (`60,167,920` bytes) | `https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/20261023-dd7c24b0d165-0/x86_64/debug/vmlinux-6.1.188` | `d3a1cb6876fe58ca5c6e3d84324d0d805e7be5d3456a846934964b41bc6cf603` |
| Exact build config | Same URL plus `.config` | `05238c0baa2e1266abadaf5c2d90f70a04f2f2a644381f5c03452e9b98a624a4` |

The published config includes `CONFIG_BPF_SYSCALL=y`, `CONFIG_PERF_EVENTS=y`,
`CONFIG_TRACEPOINTS=y`, `CONFIG_FTRACE=y`, `CONFIG_FTRACE_SYSCALLS=y`,
`CONFIG_BPF_EVENTS=y`, `CONFIG_DEBUG_FS=y`, `CONFIG_VIRTIO_BLK=y`, and
`CONFIG_EXT4_FS=y`. Its unstripped ELF contains `__tracepoint_sys_enter` and
`perf_trace_sys_enter`. The corresponding non-debug Firecracker CI config has
`# CONFIG_FTRACE is not set`.

To test the candidate, replace only the guest kernel download and pinned SHA in
`.github/workflows/stage039-real-firecracker-jailer.yml` (source acquisition
and manifest pin). Keep the original artifact and its digest in the evidence
record. Firecracker v1.17 boot, the existing ext4 rootfs, guest tracefs mount,
and the BPF verifier must all pass on the real KVM runner before this can be
called compatible. The config and ELF inspection alone do not close 0.39.27.
