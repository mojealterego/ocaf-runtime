# Guest process quarantine helper (stage 0.39.24 / 0.39.28 increment)

`quarantine.c` verifies an Ed25519 operator permit against a public key pinned
inside the read-only guest image. It attaches to one exact Linux PID with
`PTRACE_SEIZE`, interrupts it, rechecks its `/proc/<pid>/stat` start tick after
the stop, holds it for 10–5000 ms, then detaches. This is **reversible**; it
does not send `SIGKILL`. No unsigned intent, wrong tenant, old boot ID, stale
PID, expired permit, missing cross-boundary BPF identifiers, or single event
can stop a process. The signed evidence digest is a binding field; the
signing authority must independently evaluate the underlying evidence.

The permit is an exact UTF-8 ASCII line, signed over its complete bytes:

```text
OCAF039Q1|tenant-a|<32-lowercase-hex-nonce>|<guest-boot-uuid>|<decimal-pid>|<decimal-proc-start-ticks>|<decimal-guest-bpf-id>|<decimal-host-bpf-id>|<decimal-event-count>=2|<decimal-hold-ms-10..5000>|<decimal-expiry-unix-within-300-s>|<64-lowercase-hex-evidence-sha256>\n
```

Signature: raw 64-byte Ed25519. Trusted public key: raw 32-byte Ed25519 at
`/opt/ocaf/quarantine-authority.pub`, compiled into the helper, inside the
digest-pinned guest rootfs. The tenant is also compiled into the helper
per image. Example build after installing `libssl-dev`:

```sh
gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
  -DOCAF_Q_TENANT='"tenant-a"' \
  stage039/guest-quarantine/quarantine.c -o ocaf-guest-quarantine \
  -lcrypto -lz -lzstd -ldl -lpthread
```

## Integration contract

1. Broker verifies a real guest openat event plus independent host cgroup
   evidence, applies an operator-authorized policy, then signs the exact
   permit. A filename or entropy score alone must never trigger it.
2. Broker binds guest boot UUID, tenant, target PID and start tick, two BPF
   program IDs, event count, bounded expiry/hold, and evidence SHA to its
   durable lease. The public key is provisioned before sealing the guest image.
3. PID1 relays the permit and signature to the target guest, executes this
   helper, checks both `STOPPED` and `RELEASED` receipts and the child state,
   while the second tenant VM remains running. On helper crash, PID1 must
   recover or terminate the VM within a bounded deadline.
4. `scripts/stage039_real_guest_quarantine.py` now provisions a dedicated
   Firecracker lab image and two concurrent jailed VMs. It waits for two real
   guest Aya `openat` events from the selected Wasmtime PID and a separate
   host Aya cgroup result, then sends a short-lived Ed25519 permit over a
   Firecracker AF_VSOCK path bound to the VMM through `SO_PEERCRED`. The guest
   helper must report `STOPPED` and `RELEASED`; host code verifies tenant B
   remains live while A is stopped. The workflow is a candidate until its
   actual KVM run passes. The operator and image signing keys are generated
   in that one CI run, so independent provisioning, revocation, durable
   multi-host evidence, and production acceptance remain open. This lab does
   not bind the permit to a durable broker lease. The host Aya cgroup probe
   finishes and detaches before the permit is sent; simultaneous host/guest
   BPF enforcement and the joint failure drill in 0.39.25 remain open.

Run the current-host tests with
`python3 -m unittest discover -s stage039/guest-quarantine/tests -v`.
They exercise a real ptrace stop/release and rejection of altered signatures,
wrong tenant, stale PID, insufficient event count, and expired permits. They
do not prove Firecracker, guest security policy, or production acceptance.
