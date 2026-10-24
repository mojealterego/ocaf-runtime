# Guest `openat` kernel probe (stage 0.39.22 / 0.39.27 increment)

`guest_openat_probe.c` loads an eBPF tracepoint program **inside** the Linux
Firecracker guest. The program counts `sys_enter_openat` events only when both
the Linux TGID equals the supplied Wasmtime child TGID and `comm` is exactly
`wasmtime`. The helper obtains the kernel assigned BPF program ID with
`BPF_OBJ_GET_INFO_BY_FD`, attaches through `perf_event_open`, and reads its
array map. It exits without an `EVENT` on any load, attach, lookup, or timeout
failure.

Build for the guest architecture:

```sh
gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
  stage039/guest-bpf/guest_openat_probe.c -o ocaf-guest-openat-probe
```

PID1 integration contract:

1. Fork the intended Wasmtime child, blocked on a private pipe **before**
   `execve`.
2. Run `/usr/bin/ocaf-guest-openat-probe <child-tgid> 30` and wait for its
   exact `OCAF039_GUEST_OPENAT_PROBE_READY_ID=<positive-id>` line.
3. Release the Wasmtime child, verify its exact WASIp1 output and zero exit.
4. Require `OCAF039_GUEST_OPENAT_EVENT_ID=<same-id> COUNT=<positive-count>
   TGID=<child-tgid> PID=<positive-pid>` and helper exit zero. Bind these
   fields to the signed tenant nonce, image digest, VM ID, and broker lease in
   the host receipt. Reject reused serial, mismatched PID, absent readiness,
   or an event produced after VM teardown.

The target guest kernel must expose `syscalls/sys_enter_openat` through
tracefs or debugfs and allow privileged `BPF_PROG_TYPE_TRACEPOINT` plus
`PERF_EVENT_IOC_SET_BPF`. This is a real guest kernel probe but does not use
Aya, enforce guest egress, quarantine a process, or constitute independent
guest attestation. These acceptance items remain open until integrated and
tested on the actual KVM runner.
