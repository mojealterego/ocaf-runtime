# OCAF 0.19 — transactional PostgreSQL-local effect sink

## Implemented boundary

`RpcAction::ApplyFencedEffect { lease, effect }` is a signed, authenticated
worker RPC, allowed only to the configured tool-allowlisted lease owner.
It is deliberately **not** wired as a generic runtime tool action.

`PgJobQueue::apply_fenced_effect` writes to
`ocaf_pg_fenced_effects_v1(job_id PRIMARY KEY, worker_id, tool,
leader_epoch, effect, committed_at_ms)`. A database transaction:

1. Verifies the active dedicated PostgreSQL leadership session.
2. Locks the current durable epoch row (`FOR SHARE`).
3. Locks the job row (`FOR UPDATE`), validates the exact worker,
   tool, attempt number, unpredictable lease token, expiry timestamp,
   epoch and `running` status.
4. Rechecks the database-clock deadline within the effect INSERT.
5. Inserts at most one effect per job. An identical request on the same
   live lease returns `applied=false` with its original timestamp.
   Different JSON or an old generation is rejected without overwrite.
6. Commits once. Any SQL/storage failure rolls back the entire effect.

The resulting receipt contains only job ID, leader generation,
newly-applied boolean and original commit timestamp.

## Verification

`tests/postgres_fenced_effect.rs` uses a real PostgreSQL server and
signed coordinator gateways. It tests one write, identical retry,
conflicting duplicate, leader failover, and expiry rejection.

Run `cargo test --test postgres_fenced_effect` under the existing
isolated PostgreSQL test configuration, then the full CI workflow.
Do not use development loopback plaintext in production.

## Non-goals and residual risk

This table is a concrete transactional **local sink**, not an adapter
for arbitrary side effects. External tools, including HTTP, file writes,
payments and messaging, are not fenced by it. The worker pre-execution
lease check from v0.18 remains point-in-time. PostgreSQL is assumed to
have one authoritative primary; divergent network-partition primaries
invalidate the correctness assumptions. A verified HA/STONITH stack
and externally enforced fencing protocols remain required.

A privileged DBA can modify rows, so use least-privilege migrations,
restricted application roles, audit and backups. The API is opt-in,
rather than silently rerouting existing remote tools into this store.

## Local release serialization

The sink additionally holds a verified `PgEpochGuard` for its
transaction's duration. Graceful leader release on that same coordinator
cannot race ahead of its local effect commit. Another coordinator's
publication of a successor epoch is ordered by the PostgreSQL epoch row
share lock. This does not extend to effects stored outside PostgreSQL.
