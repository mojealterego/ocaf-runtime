# OCAF 0.17 — durable coordinator leadership fencing

## Failure mode addressed

PostgreSQL session advisory locks are **reentrant**. Concurrent
`try_acquire()` calls on one PgLeader can stack multiple acquisitions;
a single `release()` would then leave an invisible held lock and stop a
standby from promoting. Checking only `SELECT 1` likewise proves
backend reachability, not that it still owns the correct lock.

## Implemented in src/cluster/leadership.rs

- A Tokio mutex serializes election, verification, and release.
- After obtaining the session advisory lock, a durable row in
  `ocaf_pg_leader_epochs_v1(cluster_id,epoch)` is inserted or incremented.
  The same shared database is used across coordinator hosts. Epochs start
  at 1 and increase on each successful promotion.
- Verification compares the local epoch with the durable row **and**
  checks the dedicated backend's actual exclusive advisory lock via
  `pg_locks`, matching the full 64-bit advisory key and backend PID.
- A failed epoch publication or ambiguous unlock poisons the leadership
  session; the process must establish a new connection before election.
- `fencing_epoch()` returns a checked generation for future downstream
  fencing protocols. Epoch publication occurs before the coordinator
  starts serving requests.

## Acceptance evidence

Run the full GitHub Actions CI. Specific PostgreSQL regression tests:
`cargo test --test postgres_leadership` with `OCAF_TEST_PG_DSN`.

Tests cover parallel election calls on one PgLeader, single release
allowing immediate standby takeover, durable increasing epoch across
promotions, and rejection of an artificially advanced epoch while the
old PostgreSQL session is still live.

## Trust boundaries and deployment

The epoch record is meaningful **only against one authoritative
PostgreSQL primary and a shared database**. During a primary
split-brain partition, two divergent primaries can both elect a leader.
The code cannot certify physical host power loss, STONITH fencing,
PostgreSQL synchronous replication, automated DB failover, or stable
client route changes.

This epoch is **not yet propagated to remote workers or enforced by
external-effect systems**. Those sinks must perform atomic monotonic
epoch checks against a trusted authority before allowing changes.
Even a correctly fenced coordinator cannot retroactively undo effects
already sent to a non-cooperating external service.

Provision the `ocaf_pg_leader_epochs_v1` table through a separately
reviewed migration using a privileged role in production; the current
experimental runtime still offers CREATE TABLE IF NOT EXISTS on
leader connection for test/dev convenience. Runtime credentials need
SELECT, INSERT and UPDATE permissions on this table.

Use isolated database trust domains and verified TLS for database and
Zenoh. Test genuine multi-host network partitions before any HA claim.

## Cancellation of asynchronous elections

A dropped `try_acquire()` future can have acquired a PostgreSQL
session lock even if its caller never saw the response. A transition
guard now marks this PgLeader unusable and terminates its dedicated
database connection whenever acquisition or release is cancelled or
ends ambiguously. The regression test holds an epoch row uncommitted
to block generation publication, cancels the active election after the
advisory lock becomes visible, and checks that a standby can promote.

The assertion concerns release of one database session's advisory lock,
not network partition recovery or client routing.
