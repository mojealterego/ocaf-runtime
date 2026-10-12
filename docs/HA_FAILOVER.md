# OCAF 0.23 — PostgreSQL active/passive HA coordinator

## What changed

The existing `OCAF_CLUSTER_MODE=postgres` coordinator now has
`OCAF_COORDINATOR_MODE=ha`. In this opt-in mode each node acts as an
active/passive candidate for the *same* PostgreSQL advisory leader lock:

1. On startup or after a failure, open a **new PostgreSQL session**.
   Retry with bounded exponential backoff (250ms up to 5s) while the
   database remains unavailable. Unhealthy candidates expose no RPC
   service; standby candidates do not serve until they acquire leadership.
2. Publish a monotonic PostgreSQL fencing epoch on election.
3. **Before exposing a Zenoh queryable**, reconcile previously running
   jobs: any lease from a predecessor epoch is marked
   `uncertain`, even when its lease timeout has not yet elapsed.
   Never silently restart these potentially effectful tasks.
4. Resume accepting signed traffic using fresh PG replay, coordinator
   directory and jobs connections under the verified leader session.
5. Check PostgreSQL leadership every 500ms, and independently reconcile
   expired leases every 2s while serving. RPC admission and replies
   also verify the live leader.
6. When serving fails or leadership is lost, drop the queryable,
   release/poison the leader session, and restart election with new
   connections. In-flight worker effects are protected by their own
   cooperating fencing adapters, not by stopping a Zenoh reply.

The older `active` and `standby` modes retain one-shot behavior.
`ha` is intended for supervised deployments and can be paired with
systemd or an orchestration restart policy.

## Deploy two candidates

On **both independent hosts**, use matching cluster ID, coordinator
node ID, PostgreSQL primary/service endpoint and peer authorization
configuration. Configure the same Zenoh RPC address/route, with vetted
**mTLS** and endpoint policies, and separate host-unique logging:

```sh
OCAF_CLUSTER_MODE=postgres
OCAF_COORDINATOR_MODE=ha
OCAF_CLUSTER_ID=<same-cluster-id-on-both-hosts>
OCAF_COORDINATOR_NODE_ID=<shared-logical-coordinator-id>
OCAF_CLUSTER_PG_DSN=<verified-TLS-primary-service-DSN>
OCAF_ZENOH_CONFIG=/etc/ocaf/zenoh.json5
OCAF_PEERS_FILE=/etc/ocaf/peers.json
```

Starting with a single PostgreSQL primary remains a single point of
failure. Use an operator-managed PostgreSQL HA system with known-good
fencing for primary failover, a stable service endpoint, tested backups,
and a TLS trust chain. **OCAF is not a PostgreSQL consensus/replication
manager and does not verify primary split-brain safety itself.**

### Database migration / namespace isolation

The PostgreSQL job queue now stores `cluster_id` in
`ocaf_pg_jobs_v1`. Job submission, owner read/audit, claim, signed
lease validation, completion and local effect commit are scoped to the
attached leader's cluster identity. Jobs without a configured leader
have the legacy namespace `''`. A job UUID remains globally unique,
which may yield a conflict (never a cross-cluster read) if reused.

Existing installations running v0.22 or older may have pre-existing
jobs with `cluster_id=''` even though they belonged to a leader.
The v0.23 schema adds the column but **does not guess** their original
cluster ownership. Before enabling production operation, inspect and
back up the database, verify the old cluster membership, and execute
an explicit, reviewed migration mapping each legacy job to the
correct cluster. Never blindly assign all rows in a shared database,
and do not delete uncertain records to make claims restart.

### Recovery semantics

`recover_expired` is now a leader-verified, epoch-row-locked
transaction. It never operates across cluster namespaces, and
only quarantines expired leases (or predecessor-epoch leases for
an active PostgreSQL leader). Recovery audit records are atomically
created once per status transition. The service logs the number
reconciled at promotion.

The `uncertain` state is **terminal until manual reconciliation**:
a network-disconnected worker may have already completed a
third-party side effect. A newly elected coordinator must not
automatically retry effectful jobs.

## Verification

Run CI with a real PostgreSQL 16 service:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --no-default-features --test postgres_ha_recovery
cargo test
```

Integration tests cover a predecessor's long-lived unexpired lease,
instant quarantine and non-retry after takeover, isolation between
two clusters sharing the same database, and a real PostgreSQL
backend termination followed by a fresh election.

A completed GitHub Actions job is **not** physical multi-host validation.
Before production HA, perform network partition/dual-primary tests,
Zenoh mTLS failover checks across hosts and load balancers, long-running
fuzz/soak tests, database backup/restore and restore-point verification,
and host clock drift testing. Verify that only one writable PostgreSQL
primary can exist at a time.

## Residual risk

A stale external effect permit remains usable at an isolated sink until
its authorization expires or a higher epoch activation arrives (at most
15 seconds for the current local sink). OCAF cannot guarantee
exactly-once execution of arbitrary external side effects. Full
physical disaster recovery and cryptographic key rollover remain open.
