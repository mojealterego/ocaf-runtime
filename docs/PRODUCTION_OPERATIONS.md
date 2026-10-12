# OCAF Runtime 0.25.0 — production hardening and operator runbook

## v0.26 upgrade notice

After upgrading all coordinators to v0.26, the live capability
directory is stored in `ocaf_pg_nodes_v2`, `ocaf_pg_tools_v2`
and `ocaf_pg_cursor_v2`, keyed by `cluster_id`. The old v1
membership tables are **not migrated or deleted automatically**.
A coordinator promoted by PostgreSQL fencing starts with an empty v2
directory until signed agent heartbeats replenish the cluster.
Do not run mixed v0.25/v0.26 coordinator versions in one cluster,
because their directories are intentionally incompatible. Preserve
the original v1 tables and replay journal until an operator has
reconciled agent registration, outstanding jobs and effects.
Run `migrations/v0.26_directory_preflight.sql` against a reviewed
database to assess both table generations without modifying rows.
See [FINAL_ACCEPTANCE.md](FINAL_ACCEPTANCE.md).

## Scope and guarantees

Version 0.25 adds a read-only, cluster-scoped **job** health probe,
private configuration-file loading and concrete systemd templates.
It does **not** certify OCAF for production: multi-host failover,
database replication, mTLS rollover, chaos soak tests and a complete
restore drill still require real infrastructure and operator evidence.

## Installing isolated Linux services

Build with a pinned, reviewed Rust toolchain:

```sh
cargo build --locked --release --bin ocaf-coordinator --bin ocaf-cluster-health
cargo test
```

If no committed Cargo.lock is present, generate and review a lockfile
before using `--locked`; **do not** treat this instruction as a claim
that the repository already supplies a frozen dependency graph.

Install binaries under `/usr/local/bin`, create an unprivileged
`ocaf` user, install `deploy/systemd/*.service` and
`ocaf-cluster-health.timer` under `/etc/systemd/system`,
and create `/etc/ocaf` as a service-owned directory with
restrictive permissions. The sample units run unprivileged with
read-only system paths, minimal capabilities and resource limits.
Review limits against production load rather than copying blindly.

### Environment and keys

`/etc/ocaf/coordinator.env` (configuration, not passwords):

```sh
OCAF_CLUSTER_ID=production-cluster-1
OCAF_COORDINATOR_NODE_ID=coord
OCAF_ZENOH_CONFIG=/etc/ocaf/zenoh.json5
OCAF_PEERS_FILE=/etc/ocaf/peers.json
OCAF_ALERT_PENDING_THRESHOLD=100
OCAF_ALERT_UNCERTAIN_THRESHOLD=1
OCAF_ALERT_EXPIRED_AGENTS_THRESHOLD=1
```

Never enable `OCAF_ALLOW_INSECURE_LOOPBACK=1` for remote hosts.
The Zenoh policy requires a correctly configured mTLS-only transport
for non-loopback traffic. PostgreSQL must use verified TLS, pinned
server trust and a DB role granting only necessary privileges.

Configure the DB credential in a **service-user-owned absolute
file** at `/etc/ocaf/pg-dsn`, mode 0600. It should contain one
PostgreSQL connection string without shell quotes; for example,
`host=pg-primary.internal dbname=ocaf user=ocaf sslmode=verify-full`
plus credential material obtained from the operator's secret manager.
The sample unit sets `OCAF_CLUSTER_PG_DSN_FILE`. Never set
`OCAF_CLUSTER_PG_DSN` concurrently; both together fail closed.
The file loader refuses symlink final components, wrong owners,
group/world permissions, excess size and invalid UTF-8. Keep the
file directory itself restricted. Secrets in an environment or
a copied service definition can leak via process inspection or
deployment tooling; file loading narrows but does not eliminate risk.

Ensure `OCAF_PEERS_FILE` points to a service-user-owned mode-0600
regular JSON file, contains unique peer keys and ACLs, and has no
symlink final component. TLS private keys have their own
Zenoh policy checks. Rotation of controller, worker, peer and
certificate keys requires coordinated maintenance.

**Important:** `ocaf-coordinator` validates the presence of these
paths on startup; the file-backed DSN does not by itself grant database
schema privileges or configure PostgreSQL HA.

### Monitoring

`ocaf-cluster-health` opens its own bounded read-only PostgreSQL
session. It does **not** acquire the advisory leader lock.
Each invocation emits **one JSON report** with
`health`, `alerts` and `snapshot`, or static-label Prometheus
measurements when `--prometheus` is selected. The existing CLI exit
contract is **0 healthy, 1 warning, 2 critical**; inaccessible DB
also fails closed. Use `--no-fail` only for a protected passive
metrics/textfile collector. Use `--node <ID> --json` for a privileged
per-node lookup; this is not tenant-isolated. An unknown option is
rejected before connecting to PostgreSQL.

`leader_sessions=0` is **critical**, and
`jobs_running_expired>0` and `jobs_uncertain>0`
(default threshold 1) are critical alerts.
Pending backlog and expired agents are warnings by default.
Thresholds are positive and bounded to 1–1,000,000.

Since v0.26 **both jobs and registered agents** in the v2 directory
are scoped to the selected cluster ID. Read-only monitoring
counts and per-node lookup queries use the same namespace.
Prior v1 global tables are intentionally not imported automatically:
their original cluster identity cannot be inferred. Preserve those
tables, re-register authenticated agents under the elected leader,
and verify the namespace before permitting traffic. Database roles,
secrets, and workload identity still require strong isolation;
namespace-scoped SQL does not provide independent PostgreSQL tenants
or replace database row-level access controls.

Enable the health timer with `systemctl enable --now
ocaf-cluster-health.timer`. A timer only runs the probe:
**a separate monitoring service must consume journald and page
operators on a failed check**. A green process-exit status does not
prove the independent sink or an external API has completed work.

### Upgrades and migration

1. Quiesce new job submissions, snapshot the PostgreSQL primary
   using a consistent operator-managed backup and back up each sink's
   SQLite journal plus identity namespace. Test restoration to a
   different instance.
2. Run `migrations/v0.25_cluster_preflight.sql` against a **restored**
   copy or an explicitly authorized maintenance read-only connection.
   Verify unscoped `cluster_id=''` jobs and all `uncertain` records.
3. Match every legacy job to its original cluster using external
   provenance and operator-reviewed evidence. Do **not** blanket
   migrate all legacy rows into the current cluster or delete them.
4. Coordinate binary upgrades on all coordinator candidates,
   workers and effect sinks; do not mix incompatible sink v1/v2
   protocol versions.
5. Validate Zenoh certificate chains and private key ownership,
   peer ACLs and same-cluster configuration across standby hosts.
6. Exercise standby promotion under controlled primary-session
   failure, verify fencing epoch monotonicity and zero stale commits.
7. Enable monitoring, capture full audit and establish a documented
   reconciliation workflow for `uncertain` jobs.

**Rollback:** Stop accepting new operations, preserve the original
database/journals and service logs, restore from the verified backup
only after reconciling effects committed since the restore point.
Never roll back database generations or effect acknowledgment state
and then replay remote jobs blindly. The system does not implement
a safe automatic downgrade migration.

### CI coverage and acceptance

`tests/production_health.rs` checks that PostgreSQL job counters
cannot include another cluster's records, verifies false readiness
after loss of the leader session and checks that no job input leaks
into the health report. `tests/health_cli.rs` exercises actual
JSON, exit-code, file-secret and conflicting-secret behavior.
Core tests check private file metadata and symlink handling.
Existing Zenoh and chaos suites remain enabled.

GitHub Actions exercises one PostgreSQL primary and one runner. Before
claiming production readiness, collect **real, physical multi-host**
results, test mTLS certificate rotation and expiration, prove
PostgreSQL **single-primary fencing** with externally managed HA,
run long soak/performance tests, and perform real backup/restore
and controlled failback rehearsals. These requirements remain open,
not implied successful by a passing CI job.
