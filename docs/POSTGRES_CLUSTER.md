# OCAF 0.11 — PostgreSQL-backed coordinator and active/passive service

**Status: experimental; v0.11 now integrates shared replay, membership, job queue and leader-gated serving. It is not a production-certified multi-host HA system.** This release
adds (1) PostgreSQL session advisory leadership across independent
connections and (2) a globally shared PostgreSQL remote-job queue
behind the existing signed Zenoh RPC gateway. Both are tested against a
real PostgreSQL server in CI. They are designed as building blocks for
multi-machine coordination, **not a completed consensus/failover system**.

## What is genuinely distributed

- `PgLeader::connect(dsn, cluster, allow_insecure_loopback)` connects to
  the shared PostgreSQL primary through a **dedicated backend session**.
  `try_acquire()` uses `pg_try_advisory_lock` with a SHA-256-derived stable
  namespace key. Competing processes/machines connected to the same DB
  cannot both hold the same advisory lock. Abrupt connection loss releases
  it. `verify()` checks that the original session is still alive.
  `acquire_standby()` waits for ownership. Do **not** use connection
  poolers in transaction-pooling mode for session-scoped locks.
- `PgJobQueue` uses PostgreSQL transactions, unique job IDs and
  `SELECT ... FOR UPDATE SKIP LOCKED` to serialize claims from distinct
  coordinator processes. The durable queue uses one attempt per remote
  task, stores owners, and checks worker identity, tool, attempt, lease
  token and lease deadline before accepting result updates.
  Lost/expired jobs become terminal `uncertain`, never silently rerun.
- `RpcGateway::with_postgres_job_db(dsn, allow_insecure_loopback).await`
  replaces the gateway's SQLite job backend while preserving the
  existing role-restricted and signed `SubmitJob`, `ClaimJob`,
  `FinishJob`, and owner-only `GetJob` actions.

The SQLite-backed configuration and its regression suite are retained.

## Opt-in coordinator configuration

Set the existing environment variables for the coordinator's single-host
membership/replay databases, leader lock, peer permissions and TLS Zenoh
config (see [HA.md](HA.md)). Add:

```bash
OCAF_EXPERIMENTAL_PG_JOBS=1
OCAF_PG_JOBS_DSN='host=postgres.example.com port=5432 dbname=ocaf user=ocaf sslmode=require'
```

Provision database credentials outside the repository and environment
scripts that are logged or shared. This example does not contain a
password; use protected secret management. For the PostgreSQL transport,
**`sslmode=require` is mandatory for remote addresses**. The connector
uses native TLS with the operating system's certificate trust store and
DNS certificate verification. The only plaintext exception is explicit
`OCAF_ALLOW_INSECURE_LOOPBACK=1`, accepted only for literal localhost
or Unix socket connections. The same setting is separately restricted
by Zenoh's TLS policy preflight.

The database user needs permission to create tables and indexes during
initial schema setup and to read/write its own job records; production
deployment should move schema migrations to a separate privileged role
and use a least-privileged runtime account.

### Coexistence with the legacy single-host mode

For the legacy SQLite mode, **do not run two active OCAF coordinators
on separate machines**. That mode intentionally preserves local replay,
membership and OS-file leader election. The separate **v0.11 cluster
mode** migrates all three journals to PostgreSQL and binds serving to a
live PostgreSQL session leader.

Neither mode provides certified production HA. For cross-host cluster
deployment the operator still needs reliable database failover, a
stable client routing endpoint, proper Zenoh TLS/mTLS verification,
external fencing and crash/network-partition testing. A PostgreSQL
session advisory lock is **not** a fencing token recognized by an
external payment system, file server or arbitrary side-effecting tool.

The same limitations on exactly-once external effects, worker
attestation, tenant isolation, audit retention and certificate rotation
remain.

## CI evidence

GitHub Actions runs the standard Rust format/Clippy/tests plus a
PostgreSQL 16 container. `OCAF_REQUIRE_PG_TESTS=1` ensures the
PostgreSQL tests do not silently skip in CI.
`tests/postgres_leadership.rs` validates independent sessions,
leader exclusivity, connection-drop takeover, namespace isolation,
remote plaintext rejection and invalid identities.
`tests/postgres_jobs.rs` validates independent connections, atomic
claims, owner isolation, UUID collision handling, restart recovery
and stale lease rejection.
`tests/postgres_rpc_e2e.rs` verifies the existing signed RPC gateway
can submit through one PostgreSQL connection and execute/report/read
the job through a second gateway connection.


## OCAF 0.11 — opt-in active/passive coordinator with shared security state

Version 0.11 adds **shared PostgreSQL replay admission** and a
**shared PostgreSQL agent capability directory** alongside the durable
PostgreSQL job queue. All three use transactional storage in the same
PostgreSQL primary. The service exposes a separate deployment mode:

```bash
OCAF_CLUSTER_MODE=postgres
OCAF_COORDINATOR_NODE_ID=coord
OCAF_CLUSTER_ID=production_cluster_1
OCAF_CLUSTER_PG_DSN='host=db.example.com port=5432 user=ocaf dbname=ocaf sslmode=require'
OCAF_COORDINATOR_MODE=active       # or standby
OCAF_PEERS_FILE=/etc/ocaf/peers.json
OCAF_ZENOH_CONFIG=/etc/ocaf/coordinator.json5
cargo run --bin ocaf-coordinator
```

The two coordinator instances must use **identical cluster IDs, peer
policies, node IDs, shared PostgreSQL DSN and compatible secure Zenoh
transport configuration**. For distinct physical hosts, each host needs
appropriate listener certificates and a client discovery strategy or
an externally managed stable virtual endpoint. DNS names, external
load balancers, host fencing and PostgreSQL failover are not managed by
OCAF.

This mode does not read the local SQLite replay, membership or job paths,
and does not need a local coordinator lock file. Legacy deployment
continues to require its prior SQLite settings and OS lock. It is
important not to configure both modes simultaneously by accident.

### Leadership and safe promotion

- A PostgreSQL session advisory lock is acquired before any Zenoh
  queryable or gateway is opened. `OCAF_COORDINATOR_MODE=active` fails
  on an existing leader; `standby` blocks until the original
  PostgreSQL session releases the lock.
- The leader has a dedicated PostgreSQL connection that is **not**
  transaction-pooled. The application checks this session before
  accepting each RPC request and approximately every 500 ms during
  operation. The coordinator stops serving on detected leadership or
  database connectivity loss.
- In-flight RPC tasks already spawned when leadership is lost can
  finish or fail independently. Shared PostgreSQL transactions enforce
  uniqueness and lease fencing, but **external effects are not
  exactly-once**. Operators still must reconcile `uncertain` jobs.
- Replay protection is now global to all coordinator instances sharing
  the database, with a transaction-level advisory mutex guarding
  duplicate admission, expiration purge and a bounded replay ledger.
- Membership heartbeats and capability replacement are atomic, with
  leases evaluated against coordinator reception time. Round-robin
  selection uses a transactional shared cursor. These records survive
  changes of process and machine as long as PostgreSQL remains
  available.

### Explicit exclusions

**OCAF 0.11 is an experimental active/passive multi-host service
integration, not certified production high availability.** The tests
use independent DB connections and three Zenoh TCP sessions on one
CI host, not separately provisioned physical machines or real TLS
certificate handshakes. An operator must separately deploy and verify
PostgreSQL replication/failover, network discovery, secure certificate
provisioning, backup/restore, secret rotation, monitoring, hard server
fencing and fault-injection tests.

Do not use `OCAF_ALLOW_INSECURE_LOOPBACK=1` outside single-machine
development. Encrypted PostgreSQL does not automatically secure
Zenoh; the Zenoh CLI security preflight remains mandatory.

See `tests/postgres_coordinator_state.rs` for cross-connection
authentication, replay and capability lease regressions, and
`tests/postgres_zenoh_e2e.rs` for end-to-end authenticated job
execution through the PostgreSQL-controlled gateway.
