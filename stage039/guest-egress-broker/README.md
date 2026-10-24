# Stage 0.39 guest egress boundary increment

The guest client sends a bounded, signed destination permit over Firecracker
virtio-vsock to host CID 2, port 5000. It has no DNS, TCP or HTTPS path of its
own. The host listener at Firecracker's per-VM `uds_path_5000` authenticates
the connecting VMM with Linux `SO_PEERCRED`, live PID start ticks, UID,
cgroup membership and host boot ID. It derives the tenant from that reserved
VMM identity; guest-supplied tenant strings have no authority. The existing
`stage039_egress_broker.EgressBroker` verifies the Ed25519 permit, pins one
DNS result to a literal IP, verifies TLS hostname/certificate, rejects private
and metadata addresses, and refuses redirects. Any transport or broker error
returns a zero-length DENY frame.

The pinned official Linux 6.1.188 debug guest config has
`CONFIG_VSOCKETS=y`, `CONFIG_VIRTIO_VSOCKETS=y`, and
`CONFIG_VIRTIO_VSOCKETS_COMMON=y`. The guest binary builds with:

```sh
gcc -std=c11 -O2 -static -Wall -Wextra -Werror \
  stage039/guest-egress-broker/guest_vsock_egress.c -o guest-vsock-egress
```

## Wire contract

The guest sends `u32_be permit_length` followed by 1–65536 signed-envelope
bytes. The host returns `u8 status` (`1` granted, `0` denied), `u32_be
response_length`, and at most 1 MiB response bytes. The guest reports only
the approved byte count. The current guest client reads
`/opt/ocaf/egress-permit.json` from its read-only rootfs and allows only the
fixed host CID/port. The host bridge chooses the authenticated tenant from
kernel process identity before reading this frame.

## Required KVM integration and limits

1. Reserve each broker lease, verify its signed permit, then configure a
   distinct Firecracker `vsock` guest CID and private `uds_path` per VM.
   Keep direct guest TAP egress disabled for this test.
2. Create each `uds_path_5000` listener in the jailer's private directory,
   map it to the exact live Firecracker PID/UID/start tick/cgroup and start
   listening before PID1 invokes the guest client. Tear it down with the VM.
3. Install the statically built guest client and permit into each separately
   SHA-pinned rootfs. Run it from guest PID1 under a bounded deadline. Check
   both guest serial and host broker receipts against the lease and VM ID.
4. On real KVM, prove an authorized local TLS destination succeeds while
   metadata IP, mixed DNS/rebinding, mismatched tenant/signature, TLS failure,
   oversized response and stopped broker all fail without direct guest TCP.
   SIGKILL tenant A must leave B's bridge and lease independent.

Current tests exercise real Linux `SO_PEERCRED`, process identity checks, a
local TLS server, Ed25519 permit verification, metadata and mixed-DNS denial,
replay and broker-outage denial. The guest binary is compiled but the host
listener and guest client have **not yet run together inside Firecracker**.
This is an implementation increment; 0.39 egress acceptance stays open.
