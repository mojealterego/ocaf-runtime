# OCAF 0.18 — remote worker pre-execution fencing

An active PostgreSQL leader binds the job store via `RpcGateway::with_pg_leader`.
PostgreSQL job claims record the positive durable election epoch and
send it in the signed `LeasedJob` payload. A worker calls
`RpcAction::ValidateLease` before executing a remote tool. Validation
requires a live current leader, matching worker, allowlisted tool,
attempt, random lease token, deadline and current DB generation.
Any failure prevents that worker from beginning the tool call.

Claim and finish transactions lock the durable epoch row `FOR SHARE`
before mutating job state. Publication of a new generation cannot
overtake an old in-flight job-state transaction on the same primary.
After takeover an old lease cannot successfully report completion to
the new coordinator. It remains running until its normal lease expiry
transitions to terminal `uncertain`, because remote effects may already
have happened.

The schema automatically adds `leader_epoch BIGINT NOT NULL DEFAULT 0`
for test/development convenience. Production should use reviewed
migrations and a restricted application database role. Legacy SQLite
and standalone PostgreSQL use generation 0, unchanged.

**Limitations**: this only checks authority immediately before a tool
starts, not throughout its lifetime and not atomically at the ultimate
external effect destination. Do not claim exactly-once side effects or
resistance to arbitrary network partitions. For strict fencing each
downstream sink must durably reject stale monotonic generations as part
of the same transaction that applies its external change. Production
multi-host PostgreSQL HA, replication and STONITH are still external
requirements. Coordinate cluster worker/coordinator upgrades.
