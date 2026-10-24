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

## Real KVM lab integration

`scripts/stage039_real_guest_vsock_egress.py` extends the existing privileged
KVM workflow with one extra, separately SHA-pinned read-only guest image.
The image boots a minimal PID1 that runs the guest client twice. A private
per-VM Firecracker `vsock` device uses `/e` inside the jail and the host
listener binds the corresponding `<jailer-root>/e_5000`. The listener accepts
only the exact jailed VMM PID/UID/start time/cgroup/host boot ID. A temporary
Ed25519-signed tenant permit authorizes a local TLS response; the second
request reuses the permit and must return DENY. The workflow requires matching
guest serial and host listener receipts and stores the KVM report as
`guest-vsock-egress.json`. The guest has zero virtio-net interfaces.

The lab permit, TLS certificate and trust key are generated for each run and
are not production credentials. This test covers the real KVM/vsock boundary
and replay denial only. `stage_039_complete` and `production_approved` stay
false in its report.

## Remaining acceptance limits

1. Bind the guest egress permit to the durable reservation lease and a
   long-lived operator-controlled signing authority instead of the lab key.
2. Run two simultaneous, independent egress VMs and prove tenant B survives
   tenant A's SIGKILL while both leases and listeners are independently
   revoked. Test metadata, mixed DNS, TLS, oversized response and stopped
   broker from the actual guest channel, including negative receipts.
3. Prove host crash recovery and a second durable host with independent
   signing and operator acceptance.

Local tests exercise real Linux `SO_PEERCRED`, process identity checks, local
TLS, Ed25519 verification, metadata and mixed-DNS denial, replay and broker
outage. The KVM workflow must pass before the guest/host channel is called
verified. Stage 0.39 egress acceptance remains open.
