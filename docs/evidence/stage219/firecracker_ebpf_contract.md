# Stage 219 — Firecracker/KVM and Aya/eBPF contract

This change adds a typed, fail-closed boundary in
`src/firecracker_ebpf_contract.rs`. It validates pinned Firecracker, kernel,
and rootfs digests; non-root jailer and cgroup identity; bounded VM resources;
and an explicit default-deny egress policy. It then accepts a receipt only
when the executor reports all of the following facts:

* `/dev/kvm` is present and KVM API version 12 is available;
* a VM was created and the pinned Firecracker process and guest booted;
* an eBPF program was loaded and attached, with a non-zero kernel program ID;
* the egress policy was enforced and teardown was verified.

The module does not claim to launch Firecracker or attach Aya. A receipt with
`production_approved=false` is a verified laboratory contract result. A
production result additionally requires `production_host_attested=true` from
the privileged executor. Missing KVM, guest, eBPF, or teardown evidence is
rejected before a receipt is created.

The focused Rust tests cover valid evidence, missing KVM, missing eBPF attach,
receipt tampering, invalid digest, and a network default-deny conflict. A
real KVM/Aya run remains an external backend gate and is not fabricated by the
unit fixture.
