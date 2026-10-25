# Stage 0.39.8 / 0.39.25: brokered VM with preboot host Aya

For signed WASI workloads, the two-tenant broker runner now prepares a fresh
root-owned cgroup for each VM and starts a pinned Aya `cgroup_skb/egress` guard
**before** spawning Firecracker. It verifies a real kernel program ID and an
actual denied TCP probe from that cgroup while a separate peer cgroup remains
reachable. The guard keeps its link open until the guest's signed WASI module
and guest Aya `openat` receipt have been observed and that VM has been killed
and reaped. The supervisor sends a one-byte release token, requires the guard's
detach receipt, and proves that the cgroup's host TCP probe recovers. Missing
attach, early guard exit, changed cgroup inode or missing detach aborts the
broker's proof; both SQLite reservations are released in `finally`.

The CI artifact binds each preboot host Aya program ID and cgroup inode to the
broker lease, signed manifest, jailed VM, guest Aya program ID, WASI nonce and
VM teardown receipt. The two host Aya program IDs must differ. No report may
set `stage_039_complete` or `production_approved` to true.

The host program denies a **host process** deliberately joined to the VM's
cgroup. It does not inspect guest kernel traffic; guest egress remains disabled
by the signed Firecracker policy. This test does not establish guest packet
filtering, host/guest/broker joint outage recovery, durable independent hosts,
or production signing authority. A real KVM workflow is required before this
new lifecycle can be called laboratory verified.
