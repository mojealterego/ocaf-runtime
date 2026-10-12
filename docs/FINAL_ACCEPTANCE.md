# OCAF Runtime 0.26.0 — final engineering acceptance

## Decision: development acceptance, not production certification

The agreed **v0.21–v0.26 development roadmap** is complete **subject
to passing CI for the final main commit**. Only code and automated
single-runner tests are in scope for that statement.

**Production deployment approval: CONDITIONAL NO-GO.**

There is no verified evidence in this repository of three
geographically/physically separated hosts, an independently fenced
replicated PostgreSQL primary, a tested mTLS credential rollover, a
privileged tool sandbox, a full backup/restore drill, or soak testing
under agreed SLOs. These must not be described as completed.
No codebase can independently guarantee exactly-once behavior for
arbitrary uncontrolled external side effects.

## Final security findings and remediation

| Finding | Severity | Final disposition |
| --- | --- | --- |
| Directory v1 shared node IDs, tool ACL routing and cursor across all clusters on a PostgreSQL instance | High | Fixed in v0.26: v2 composite `cluster_id,node_id` keys, scoped route lookups, scoped cursors, epoch-verified membership mutations |
| v0.25 agent health aggregate and per-node inspection traversed the global v1 directory | High | Fixed in v0.26: both queries now use v2 scoped directory |
| Stale leader may mutate directory after promotion | High | Scoped operations now obtain verified `PgEpochGuard` plus transaction-level `SELECT ... FOR SHARE` of authoritative epoch; deny stale generation |
| Old global directory rows have no trustworthy cluster provenance | High | **Intentional fail-closed migration**: v1 tables retained, never automatically adopted into v2; require signed re-registration or supervised, evidence-backed migration |
| Shared replay v1 nonce namespace and global 4096-entry limit allow unrelated clusters to collide or consume capacity | High | Fixed in v0.26: namespace-scoped v2 replay entries and per-cluster capacity, transactional fencing of replay admission. Preserve historical v1 replay records during upgrades; do not mix coordinator protocol versions. |
| PostgreSQL replication / split-brain of DB primaries | Critical | **OPEN**: rely on externally tested, fenced PostgreSQL HA; not a component of OCAF |
| Signed receiver permits during network partition | High | **OPEN**: local expiration/bounded duration; isolated receiver may still accept an unexpired signed permit |
| Arbitrary external effects exactly once / process sandbox | High | **OPEN**: no exactly-once guarantee for non-cooperating services; no untrusted tool sandbox |
| Key rotation, mTLS workload identity binding and certificate lifecycle | High | **OPEN**: operator-managed changes and testing still necessary |
| Dependency pinning, lockfile and third-party security assessment | Medium | **OPEN**: no committed Cargo.lock at this acceptance; pin/version-audit supply chain before deployment |

## Automated verification

The GitHub Actions `Rust CI` workflow covers formatting, core and full
Clippy, complete core and Zenoh test suites, PostgreSQL job/lease
fencing, independent effect sink, network partition chaos, coordinator
takeover, monitoring, and v0.26 isolation/fencing regressions.

**CI is not a physical infrastructure test**. Record the final
workflow URL and actual commit hash when CI finishes successfully;
do not infer success from earlier commits. Preserve failed runs
and remediation history as part of the audit trail.

The new `final_replay_acceptance.rs` test checks that identical signed
request correlation IDs are admitted independently by two cluster
leaders sharing PostgreSQL, rejected as duplicates *within* a
cluster, and rejected by a stale leader after generation transfer.
This is per-cluster replay admission, not a guarantee that different
clusters cannot reuse the same shared HMAC key; operators must use
distinct secrets per trust domain.

The new `final_security_acceptance.rs` test verifies that two
simultaneously live leader clusters can register the same node ID,
claim different capabilities, and resolve only their own tools;
the observability snapshot counts one live agent per cluster,
and the predecessor leader cannot mutate membership following
generation transfer.

## Upgrade and rollback gates

1. Stop incompatible older coordinator versions; take PostgreSQL
   and independent sink SQLite backups with verified restoration.
2. Run `migrations/v0.26_directory_preflight.sql` read-only.
   Determine the original namespace of v1 memberships and replay
   records from *external* operator evidence — it cannot be reconstructed
   from v1 records. v2 replay admission is isolated and does **not**
   import old global nonce records, so quiesce all old coordinators
   until every pre-upgrade validity window expires.
3. Deploy coordinated v0.26 binaries, protect secret material and
   ensure transport mTLS + PostgreSQL trust are correctly configured.
4. Allow trusted agents to **re-register via signed heartbeats**.
   Validate `ocaf_pg_nodes_v2` membership, tools, cursors, and
   the cluster-specific health report. Reject any unexpected
   cross-cluster entries and investigate before serving production.
5. Verify all historical `uncertain` jobs by checking downstream
   effects; never automatically retry possibly executed remote work.
6. Attempt controlled coordinator failover, network loss, database
   recovery and restore **on actual separate hosts**. Record
   monotonic epoch and no stale committed writes.
7. Rollback: preserve v1/v2 database states and all effect journals,
   revert executables only with explicit membership compatibility
   planning. Never blindly merge namespace-less directory rows.

## External production signoff requirements

- [ ] Independently fenced PostgreSQL HA with zero split-brain writes
- [ ] Multi-host certificate-authenticated Zenoh failover evidence
- [ ] Revocation and key/certificate rotation verified under load
- [ ] Receiver-side external effects fenced and idempotently applied
- [ ] Privilege-restricted untrusted tool execution or a narrow approved tool inventory
- [ ] Backup/restore and audited reconciliation of all uncertain jobs
- [ ] Signed release provenance and dependency vulnerability/license audit
- [ ] Agreed p95/p99 and availability SLOs tested through a long-running soak
- [ ] Separate external security review and production change approval

Only after all gates are evidenced can the system be designated
production-accepted. Completion of six planned implementation
milestones **does not make those external checks disappear**.
