# OCAF 0.38.8–0.38.9 — Linux WASIp1 subprocess hardening / acceptance

**Branch:** `main` only. **Decision:** engineering security work in progress;
**production: NO-GO**. This document records executable checks and what they
**do not** establish. The final GitHub Actions status must be read for the
final `main` SHA; a successful prior revision is not release evidence.

## Security boundaries

Core OCAF authenticates tenant/agent and enforces ACL, budgets, replay and
fencing. The restricted WASIp1 adapter executes an operator-approved module
using a separately pinned Wasmtime CLI. A signed manifest and input validation
are admission controls; neither is an OS kernel isolation primitive.

### 0.38.8 — lifecycle and process hardening

| Control | Implementation / evidence | Guarantee boundary |
| --- | --- | --- |
| Cancellation/timeout | `WasiProcessGroup` in `src/executor/wasi.rs`; process is started in a dedicated Linux process group using `process_group(0)` and group is signalled with `SIGKILL` on Drop. `kill_on_drop(true)` is retained. | Kills processes still in that group. A compromised native engine that invokes `setsid`, escapes the group, or is able to interfere with the supervisor is **not** contained by this check. |
| No new privileges | Child-only `PR_SET_NO_NEW_PRIVS=1` via `pre_exec`, spawn fails closed if the syscall fails. | Limits privilege gain via `execve`; not an alternative to seccomp, namespaces, cgroups or microVM. |
| Core files | Child-only `RLIMIT_CORE` soft/hard = 0; spawn fails closed on error. | Limits guest core-dump disclosure, not arbitrary host reads from a compromised native engine. |
| Bounded subprocess I/O | Existing independent stdout/stderr limits and guarded stdin, wall deadline, fuel, Wasm memory/stack limits; per-executor concurrency. | Wasmtime compiler/process RSS and Linux CPU consumption are not fully bounded by Wasm fuel. |
| Private workspace | `EphemeralModule` staged under `/tmp/ocaf-wasi-UUID`, directory `0700`, module `0600`, cleaned on Rust Drop. | Abrupt host termination (SIGKILL/power loss) bypasses Rust destructors; startup residue management remains a deployment concern. |

### 0.38.9 — repeatable CI acceptance

| Check | File / test | Expected behavior |
| --- | --- | --- |
| Cancellation of child and background descendant | `tests/wasi_sandbox.rs::cancelling_guest_kills_background_descendants` | Descendant is not running after cancellation; Linux `/proc` supports state check. |
| Prior cancellation and cleanup | `cancellation_reaps_guest_and_removes_private_workspace` | Child terminates and temporary module is removed. |
| Native process privileges/core limit | `wasi_guest_inherits_no_new_privileges_and_core_limit` | Child sees `NoNewPrivs: 1`, `ulimit -c = 0`. |
| stdout and stderr bombs | `stdout_bomb_is_rejected_without_host_log_leak`, `stderr_bomb_is_bounded_and_not_returned_to_callers` | Limit violation; no guest stderr returned to requestor. |
| Wasm parser mutations | `mutated_wasm_binaries_do_not_panic_import_validation` | Bounded deterministic input-mutation smoke test, not coverage-guided fuzzing. |
| Signed policy/tenant isolation | `publisher_signature_binds_module_tool_policy_and_principal` | Reject wrong signer, tenant, agent, manifest, digest. |
| Real Wasmtime 49.0.2 | `scripts/wasmtime_acceptance.sh` | SHA-256 checked before extraction; run the **entire** `wasi_sandbox` integration suite with the real pinned engine; bounded execution repetition, fuel trap, memory rejection. |

Run the documented CI acceptance on Linux x86_64:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features --test wasi_sandbox -- --test-threads=1
bash scripts/wasmtime_acceptance.sh
cargo test --locked
```

Production GO requires green Rust CI **and** green Supply Chain / Provenance
on the final SHA; both are necessary but **not sufficient**. Release review
must additionally cover:

1. Independent multi-tenant concurrent execution and leak/soak testing,
   including measured RSS, file descriptors, CPU, PIDs and residual files.
2. Independently administered host-level hardening (unprivileged account,
   restricted writable mounts and immutable pinned-engine path),
   anti-TOCTOU review of engine `execve`, parent-directory ownership,
   and system-wide process limits.
3. Recovery after OCAF `SIGKILL`, process crash, host restart and partition;
   no automatic replay of uncertain external effects.
4. Publisher key rotation and revocation, tool provenance and upstream
   Wasmtime security advisories; artifact signature beyond a bare digest.
5. Full independent adversarial security assessment, signed evidence and
   documented operator acceptance.

**Open risk:** process-group cleanup cannot contain a malicious Wasmtime
binary escaping its group, arbitrary native syscalls, or host compromise.
Kernel-level seccomp/cgroups/namespaces and Firecracker/eBPF are scoped to
0.39. Tests in this stage are implementation evidence, not third-party
attestation and not a production security certification.
