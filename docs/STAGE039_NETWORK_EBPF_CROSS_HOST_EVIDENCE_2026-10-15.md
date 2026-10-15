# OCAF stage 0.39 — Evidence ledger: jailed guest UDP, kernel TC eBPF and cross-host cold recovery

**Observation date:** 2026-10-15. **Repository branch:** `main`.
All CI evidence below originates from actual GitHub-hosted ephemeral x86_64 Linux/KVM runners, not mocked KVM.
A CI pass is **not a production rollout certificate, physical independent-host attestation, or an auditor/operator signature**.

## Verified technical results

| ID | Empirical requirement | Result | Proven limits |
| --- | --- | --- | --- |
| 039-LIVE-1 | KVM_CREATE_VM + Firecracker 1.17 jailed guest kernel boot, cgroup v2 CPU, RAM and PID limits, Aya CgroupSkb on host VMM cgroup | **PASS** | Aya blocks host processes in VMM cgroup, not packets emitted by guest virtio-net |
| 039-LIVE-2 | **Two simultaneous jailed guest VMs** with distinct tenant IDs, VM IDs, unprivileged UID/GID, dedicated pinned cgroup inodes | **PASS** | Same ephemeral host, Linux KVM |
| 039-LIVE-3 | **Guest-originated UDP** via distinct tenant TAPs while BOTH VMs jailed: nftables negative B unaffected, recovery, then SIGKILL A and continued B traffic | **PASS** | nftables tests only dedicated host TAP/IP/UDP, no internet egress |
| 039-LIVE-4 | Guest-originated UDP **kernel TC eBPF direct-action ingress** attached to guest A TAP, kernel BPF program ID observed, packets dropped, guest B unaffected, recovery after detach | **PASS** | TC BPF via Linux `tc` tool, not Aya userspace loader or per-guest netns |
| 039-HOST-1 | Separate kernel host A and B, each with KVM guest boot and verified SIGKILL/cgroup subtree drained, separate host boot IDs, independent validation job | **PASS** | Ephemeral CI kernels, not managed production machines |
| 039-HOST-2 | After host A crashes its test VMs, GitHub immutable artifact checkpoint transfers nonce, different host B boots new jailed guest Linux and originates nonce-bound UDP; separate boot IDs, bounded chronology | **PASS** | Serialized **artifact-mediated cold restart**, not automatic inter-host RPC failover |
| 039-OPER-1 | External signed independent security review | **NOT VERIFIED** | Human/independent auditor has not signed |
| 039-OPER-2 | Formal operator acceptance with independent production host and network evidence | **NOT VERIFIED** | No signed production GO |
| 039-HA-1 | Inter-host mTLS RPC live failover / partition / lease and crash recovery on durable separate production hosts | **NOT VERIFIED** | CI cold restart does not substitute for online HA |

## Primary GitHub Actions evidence and hashes

**Real jailed two-tenant guest UDP + host nftables, host cgroup Aya + SIGKILL:** 
- [Run 38721552535](https://github.com/mojealterego/ocaf-runtime/actions/runs/38721552535), `conclusion=success`.
- [Artifact 11965547090](https://github.com/mojealterego/ocaf-runtime/actions/runs/38721552535/artifacts/11965547090) SHA-256 ZIP `08f4fc112602d35bd84ab5331db21449af9d8f6d326a6e11a0ba96f0cf251f63`.
- Marker `STAGE039_REAL_TWO_JAILED_GUEST_VM_UDP_DENY_RECOVERY_AND_SIGKILL_PROVED`.

**Real kernel BPF classifier filtering packets emitted from a JAILED guest TAP (new proof):**
- [Run 38722682521](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521), `conclusion=success`; reproducible substantive source commit `9dc90890b3c5997afbbe9a281d33b7993ffc142b`.
- [Artifact 11966952515](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521/artifacts/11966952515), ZIP SHA-256 `ae1304357906d2e596c0137a1664684db1bf1045963630aca9054b46af29f4c1`.
- Marker `STAGE039_REAL_JAILED_GUEST_TAP_TC_EBPF_KERNEL_DENY_RECOVERY_PROVED`.
- Independent initial successful TC implementation proof: [run 38722652009](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722652009), [artifact 11965723904](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722652009/artifacts/11965723904), SHA-256 `86be34c62c6d87ee6366888ab0e0a06f889be35edcc1974de515efa8274c9a29`.
- The runtime reports `tc_ebpf_guest_tap.kernel_bpf_program_id`, compiled kernel ELF SHA-256, separate tenant B connectivity, tenant A negative/recovery and detached eBPF. It explicitly keeps `aya_cgroup_guest_network_tested=false` because a TC filter loaded by `tc` is not the same as Aya CgroupSkb enforcement.

**Two independently booted Linux/KVM kernels, each SIGKILL:**
- [Run 38720667372](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372), **3 jobs success** (tenant-a, tenant-b, independent verifier), source kernel boot IDs `39a9d1b0-757b-48b6-a255-f157ffe3e5ec` and `e20b7a42-963a-4a1f-893b-65a807a5091d`.
- See previous [two-host report](STAGE039_TWO_TENANTS_GUEST_NETWORK_INDEPENDENT_HOSTS_2026-10-15.md).

**Checkpoint transferred only AFTER source guest VMs failed, then an actual jailed Linux guest boots on independent recovery host:**
- [Run 38722139701](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701), **both source and recovery jobs success**.
- [Source artifact 11966656489](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701/artifacts/11966656489), ZIP SHA-256 `da6a58ef549afa50aefe2a2b5943597197cf007c66c6b8f701a1e658d2ed2ec1`.
- [Recovered artifact 11965912990](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701/artifacts/11965912990), ZIP SHA-256 `cff62bd31e59d5b3e151534e625b983f4fb5ca901ac185d5e14dd86508b72a68`.
- Marker `STAGE039_REAL_SECOND_KERNEL_JAILED_GUEST_ARTIFACT_COLD_RESTART_PROVED`, status `TWO_KERNEL_JAILED_GUEST_NONCE_COLD_RESTART_PASSED`.
- **Checkpoint is SHA-256 correlated but not cryptographically signed by an independent auditor.** It is transferred by workflow artifact, and must not be conflated with synchronous stateful distributed failover.

## Required production closure / explicit refusal to self-approve

1. Real **durable and operationally independent host** pair with separate tenant machines plus **authenticated mTLS RPC** between hosts, peer failure & network partition injection, bounded failover time, split-brain/lease fencing, recovery from a hard host crash, and intact tenant isolation after failover.
2. Full signed runtime module execution inside guests and verified policy enforcement across host and guest network boundaries, production network namespaces and per-tenant no-default-route egress, independent ingress/egress traces, and sustained concurrency/soak.
3. **Independent security audit**, finding remediation and a verifiable signature from a trusted auditor **independent of the system developer**.
4. **Operator's signed formal acceptance** of collected machine evidence. A chat instruction to continue is authorization to implement, not proof of these absent signatures.

**Stage state: 0.38 COMPLETE administratively; 0.39 IN PROGRESS; production NO-GO.** A complete acceptance record cannot be generated truthfully from current temporary runner artifacts.
