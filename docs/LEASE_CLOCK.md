# OCAF 0.16 — shared PostgreSQL clock for job lease decisions

## Threat and correction

A previously active coordinator could carry a skewed host clock across
failover: claims were granted host-calculated deadlines, recovery used
a host timestamp, and finished work was accepted against a host timestamp.
This created both premature lease loss and delayed stale-result rejection.

In PostgreSQL cluster mode, all three decisions now compare against
`clock_timestamp()` inside PostgreSQL SQL statements. A claim stores
`database time + lease duration`, returning the stored deadline via
`UPDATE ... RETURNING`. Recovery and completion check the stored
deadline against PostgreSQL time in their atomic updates. Worker identity,
lease token, tool, attempt and owner checks remain enforced.

The `now_ms` argument remains in the public interface but is ignored
for PostgreSQL leases; SQLite preserves its existing local behavior.

## Evidence and verification

`tests/postgres_lease_clock.rs` injects `u64::MAX` coordinator time
and confirms that valid jobs remain claimable, cannot be prematurely
recovered and can be completed. It also expires a selected lease directly
in PostgreSQL and verifies that a backdated host cannot complete it.
Audit and original lease-recovery regressions use database-side expiry.

CI requires the real PostgreSQL 16 service (`OCAF_REQUIRE_PG_TESTS=1`);
run `cargo test --test postgres_lease_clock`, then `cargo test`
with `OCAF_TEST_PG_DSN` configured in an isolated test environment.

## Limitations

PostgreSQL clock reliability, backups/replication, primary failover,
network partition fencing and mTLS certificate rotation remain separate
operational requirements. This is not distributed consensus and cannot
undo side effects that finish outside the lease.
