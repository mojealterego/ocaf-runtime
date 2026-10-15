# OCAF 0.39 — Consolidated real-hardware progress and nonfabricated release boundary

Date: **2026-10-15**. Branch: **main**.

## Proven executed work on actual GitHub-hosted Linux/KVM

| Category | Measured behavior | Result | Actual evidence |
| --- | --- | --- | --- |
| Two jailed VMs | Concurrent tenant A/B Firecracker guest kernels, separate jailer UID, dedicated cgroup inode, CPU/memory/pids limits | **PASS** | [run 38721552535](https://github.com/mojealterego/ocaf-runtime/actions/runs/38721552535) |
| Real guest-originated traffic | Two jailed guests emit nonce-tagged UDP from their own PID1 over independently owned TAP interfaces | **PASS** | [run 38721552535](https://github.com/mojealterego/ocaf-runtime/actions/runs/38721552535) |
| Guest eBPF network isolation | Actual kernel `tc` BPF classifier attached to guest A TAP (kernel BPF program ID). Denies A's incoming-to-host packet stream while B continues, detaches and restores A | **PASS** | [run 38722682521](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521), [artifact 11966952515](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722682521/artifacts/11966952515) |
| Independent CI hosts | Two distinct Linux KVM kernel boot IDs, each independently boots jailed guest, SIGKILL and cgroup subtree drained; third job compares | **PASS** | [run 38720667372](https://github.com/mojealterego/ocaf-runtime/actions/runs/38720667372) |
| Cross-kernel recovery | Host A completes VM SIGKILL then transfers bound nonce checkpoint via GitHub Actions artifact; separate kernel B boots new jailed guests that originate the same checkpoint-bound UDP | **PASS — artifact-mediated cold restart** | [run 38722139701](https://github.com/mojealterego/ocaf-runtime/actions/runs/38722139701) |
| **Live TLS 1.3 mTLS RPC** | Actual Linux network namespaces and veth peer TCP, ephemeral CA client SAN authentication, hostile CA-signed rogue rejected, persistent single authority lease, real blackhole route partition, fail-closed client, server SIGKILL and state recovery | **PASS — single-kernel network-namespace lab** | [run 38724640542](https://github.com/mojealterego/ocaf-runtime/actions/runs/38724640542), [artifact 11967775330](https://github.com/mojealterego/ocaf-runtime/actions/runs/38724640542/artifacts/11967775330) |
| Human-signable audit dossier | Validate complete pinned artifact digests and **two different Ed25519 signatures** from pinned keys; deny unsigned/tampered/duplicate-key cases | **PASS for synthetic signer test cases ONLY** | [CI admission run 38725162927](https://github.com/mojealterego/ocaf-runtime/actions/runs/38725162927) |

SHA256 of the real mTLS-network namespace evidence ZIP from uploader: `472406bbb585322bbee053bc1ee7b2898615fbbda1f5edb04cb723550408d41d`.

## Deliberately separated factual scopes

- **TC BPF on guest TAP is real kernel filtering of traffic initiated inside the guest**, but `tc` is the loader for that classifier, not Aya userspace `CgroupSkb`; Aya's host-VMM-cgroup test is different.
- Linux namespaces `oc39a*/oc39b*` use a **real Linux kernel veth** and actual TLS1.3 mutual-certificate connection but **share one host kernel**. This is NOT a two-production-host transport test.
- Two ephemeral host runners are real distinct Linux/KVM kernels, but their state handoff is **serialized GitHub artifact transfer** and is NOT live mTLS HA, quorum consensus or measured automatic failover time.
- Ed25519 signature tests generate synthetic disposable keys **inside CI** to verify the cryptographic verifier. They do NOT mean an independent auditor actually reviewed the application or an operator signed anything.
- The `ops/OCAF_RELEASE_OPERATOR_DECISION_2026-10-15.json` administrative directive explicitly reports missing security evidence and is **not** a production audit certificate.

## Mandatory production acceptance / exact blockers

| Criterion | What must be observed | Current state |
| --- | --- | --- |
| 039-PROD-HOST | Two specifically identified, durable, authorized independent production KVM hosts with separately retained host attestations | **NOT PROVED** |
| 039-PROD-RPC | Live authenticated **inter-host** mTLS RPC while both hosts concurrently operate tenant VMs, rather than local netns or artifact-polling | **NOT PROVED** |
| 039-PROD-PARTITION | Real host-to-host partition, bounded safety quorum/fencing, split-brain prevention and automatic recovery from **whole host** loss | **NOT PROVED** |
| 039-PROD-SOAK | Sustained load, resource/noise-neighbor isolation and ingress/egress traces on target hosts, plus guest signed WASI runtime receipts | **NOT PROVED** |
| 039-AUDIT | Independent qualified external security assessment covering production scope, with authentic signed report and explicit signed key identity | **NOT PROVED** |
| 039-OPERATOR | Formal operator acceptance of exactly the reviewed deployment evidence, with signature distinct from external auditor | **NOT PROVED** |

The system CANNOT legitimately mark these rows PASS merely from an operator's chat authorization, CI simulation, an unsigned self-assessment, or synthetic signature fixtures.

### Explicit closure state

`stage_039_complete = false`, `production_approved = false`, `release_gate = NO_GO`.

The technical CI experiments **are finished and passed within their scoped testbeds**. The remaining blockers require production infrastructure and independent signatories, neither of which can be synthesized in code. This is a factual acceptance inventory, not an assertion that all stage-0.39 production goals were achieved.

## Reproducible code

- `scripts/stage039_mtls_fencing.py`: TLSv1.3 mutual certificate/RPC server and persistent single-authority fencing.
- `scripts/stage039_mtls_partition_lab.py`: real Linux netns/veth partition and server SIGKILL fault harness.
- `.github/workflows/stage039-real-mtls-partition.yml`: privileged ephemeral Ubuntu runner test and evidence ZIP.
- `scripts/stage039_signed_acceptance.py`: out-of-band public-key fingerprint-pinned Ed25519 dual-signature dossier integrity verifier. **Never** sets `production_approved=true`.
- `scripts/tests/test_stage039_mtls_fencing.py`, `scripts/tests/test_stage039_signed_acceptance.py`: TDD negative + positive regression, with synthetic signers only.

## Update 0.39.18 — real two-tenant SO_PEERCRED broker-to-jailer identity/cgroup binding

[**CI 38739812121, SUCCESS**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121) starts two actual jailed Firecracker guest kernels ONLY AFTER two separate kernel SO_PEERCRED broker socket reservations persisted in SQLite. Pinned manifest SHA256, tenant/agent/workload and executable guest/kernel digests are correlated to measured VM/cgroup/boot and SIGKILL/peer-survival evidence. All reservations released after crash. [Full reproducibility](STAGE039_040_BROKER_KERNEL_PG_INDEPENDENCE_2026-10-15.md). **This is not yet signed WASI execution within guest, production remote authentication or independent host HA; 0.39 IN PROGRESS.**
