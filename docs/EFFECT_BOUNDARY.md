# OCAF 0.20 — fail-closed effect authorization

This release closes three previously permitted paths by which signed
but insufficiently privileged remote peers could cause unsafe effects.

1. **Cluster direct Execute is denied.** `RpcGateway` rejects
   `RpcAction::Execute` whenever a PostgreSQL advisory leader is bound.
   Legacy non-cluster execution remains unchanged.
2. **Effect writer is a distinct capability.** A worker identity with
   `worker_jobs: true` can claim, validate and finish its leases but
   cannot use `ApplyFencedEffect` without `write_effects: true`.
   This is checked after HMAC authentication and before any sink write.
   Writing effects additionally requires the matching lease worker
   identity and tool ACL, and successful transactional fencing.
3. **Cluster worker tool execution defaults to deny.** In
   `ocaf-rpc-node`, the explicit operator-supplied
   `OCAF_CLUSTER_PURE_TOOLS` list allows only vetted *pure* tools
   when `leader_epoch > 0`. Any other tool is definitively rejected
   before the executor runs. The worker must still validate the live
   lease with the coordinator and check manifest authorization.

Migration: To continue the v0.19 PostgreSQL-local effect example,
add `write_effects: true` to the configured worker peer entry
alongside `worker_jobs: true`. To run the demonstration echo worker in
cluster mode, set `OCAF_CLUSTER_PURE_TOOLS=demo_echo`. Do not put
mutating tools on this list. Negative leader epochs are rejected.

Verification: run full CI (rustfmt, both Clippy modes, core tests,
Zenoh tests and PostgreSQL-backed ACL regression suite). The tests
cover an authenticated effect-unprivileged worker and a signed
allowlisted direct tool invocation on an elected coordinator.

**Limitations:** Any service outside the v0.19 PostgreSQL transaction
still needs its own trusted atomic fencing/idempotency adapter. A
manifest's free/permissionless status is not evidence of purity.
No physical multi-host network partition or independent downstream
side-effect sink has yet been verified; this must not be described
as production HA or exactly-once external effects.

## Builder-order and backend downgrade regression

If `RpcGateway::with_pg_leader` is called before
`with_postgres_job_db`, the queue now inherits the same leader
automatically. Attaching a local SQLite queue *after* a PostgreSQL
leader is rejected at construction; attaching it before the leader
causes signed request processing to fail closed with `Unavailable`.
Thus no valid cluster leader may silently admit epoch-zero job leases
because an application chained its gateway builders in a different
order. PostgreSQL integration tests verify both construction orders.
