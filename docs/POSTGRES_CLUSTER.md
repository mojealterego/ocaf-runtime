# OCAF 0.10 — PostgreSQL-backed cluster foundations

**Status: experimental, not production multi-host HA.** This release
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

### IMPORTANT: still no automatic cross-host failover

**Do not run two active OCAF coordinator services on separate machines.**
The CLI currently retains **local** SQLite replay and capability
membership stores, and its active/passive guard remains a local OS
file lock. This is intentional: activating a cluster leader while
critical security state is not yet shared would risk split-brain,
replay regression and diverging discovery. The new PostgreSQL
leader primitive is provided as a library and is not yet wired as
the authoritative service election mechanism.

To reach real coordinator HA, the next release must move *all* state
(replay journal, membership and job state) into a shared transactional
backend, bind server serving to an actively monitored PostgreSQL
leadership session, test the loss-of-leadership shutdown path, and
configure a stable/virtual Zenoh endpoint or client failover strategy.
PostgreSQL itself requires separately engineered replication,
failover, backup and recovery; this library does not implement Raft.

The same limitations on exactly-once external effects, worker
attestation, per-tenant isolation, audit retention and end-to-end
certificate rotation remain. PostgreSQL advisory locks are NOT
fencing tokens recognized by unrelated external systems.

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
