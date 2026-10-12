# OCAF Runtime 0.13 — Cluster Observability & Operator Health

**Status:** experimental PostgreSQL-backed *read-only* operational monitoring.
This command does not create a public HTTP endpoint and does not certify
production high availability or network reachability of all Zenoh agents.

## Scope

The `PgClusterMonitor` reads a single database-clock snapshot from the
same PostgreSQL authority used by the opt-in active/passive coordinator:

- **Leadership:** number of *granted exclusive advisory lock sessions*
  matching the configured OCAF cluster ID, observed via `pg_locks`.
- **Agents:** how many registered heartbeat leases are alive or expired,
  evaluated against the PostgreSQL clock, not caller-supplied wall time.
- **Jobs:** pending, running, succeeded, failed and terminal `uncertain`
  counts, plus running jobs whose lease deadline has already passed.
- **Per-agent inspection:** `node_health(node_id)` returns the
  registered node ID, database observation time, lease expiration
  and whether that lease is alive. It never exports tool capabilities,
  peer secrets, job arguments, results, or lease tokens.

A single SQL SELECT provides the aggregate counters, minimizing
inconsistency across related counts. An advisory-lock view is still a
point-in-time observation and is **not** an external fencing token.

The monitor is read-only and does not initialize tables. It requires an
**already provisioned OCAF PostgreSQL cluster** with the `ocaf_pg_nodes_v1`
and `ocaf_pg_jobs_v1` tables. Database errors or missing tables are
returned as failures — never converted to a fictitious healthy state.

## Usage

Use the **same `OCAF_CLUSTER_ID` and PostgreSQL authority** as the
coordinator. Provision database credentials through your secret manager.

```bash
export OCAF_CLUSTER_ID=production_cluster_1
export OCAF_CLUSTER_PG_DSN='host=db.example.com port=5432 dbname=ocaf user=ocaf_monitor sslmode=require'

cargo run --bin ocaf-cluster-health -- --json
cargo run --bin ocaf-cluster-health -- --json --node agent_17
cargo run --bin ocaf-cluster-health -- --prometheus --no-fail
```

For **local development only** against plaintext PostgreSQL on literal
loopback, `OCAF_ALLOW_INSECURE_LOOPBACK=1` is permitted. Do not set it
in production.

The CLI never runs a network listener. `--prometheus` prints static,
unlabelled numeric time series to stdout, suitable for a protected
textfile collector or an external monitoring pipeline. Per-node
observations are only available in JSON (`--node <id>`): mixing
`--prometheus` and `--node` is rejected to prevent uncontrolled
cardinality or disclosure through node identity labels.

`--json` prints a report consisting of a health classification,
aggregate snapshot and active alert codes. With `--node`, the JSON
contains `{"report": ..., "node": ...}`; `node` is null for an unknown
ID.

## Alert classification and exit status

| Condition | Alert | Classification |
| --- | --- | --- |
| No advisory leader | `no_leader` | Critical |
| More than one advisory leader | `multiple_leaders` | Critical |
| At least one running lease expired | `expired_running_jobs` | Critical |
| `uncertain` jobs ≥ threshold | `uncertain_jobs` | Critical |
| Pending jobs ≥ threshold | `pending_backlog` | Warning |
| Zero agents with live leases | `no_live_agents` | Warning |
| Expired agent leases ≥ threshold | `expired_agents` | Warning |

Defaults: pending threshold **100**, uncertain threshold **1**,
expired-agent threshold **1**. Override with positive integers up to
1,000,000:

```bash
OCAF_ALERT_PENDING_THRESHOLD=250
OCAF_ALERT_UNCERTAIN_THRESHOLD=1
OCAF_ALERT_EXPIRED_AGENTS_THRESHOLD=3
```

Exit code: **0 = healthy; 1 = warning; 2 = critical**.
`--no-fail` always returns zero after *successful collection* while
still emitting the real classification, which is appropriate for
metrics exporters. Invalid configuration, unavailable PostgreSQL,
missing tables or invalid node identifiers produce a nonzero error.

Operators can schedule the CLI with systemd timers, cron or another
secured orchestration tool and route its exit status/JSON to their
alert manager. This release **does not send webhooks, email or SMS**
on its own and does not add a durable incident manager.

## Limitations and security boundaries

- Reporting reflects **database-stored heartbeat leases**, not an
  active TCP probe of each agent; a live lease can briefly outlive a
  disconnected worker.
- Expired membership rows are not automatically pruned. Historical
  stale rows may keep an expired-agent alert active. Tune thresholds
  and establish an operator-owned retention plan.
- Advisory leader count is a PostgreSQL view, not a statement that the
  Zenoh listener is reachable or that the coordinator completed startup.
- Counts are scoped to the configured PostgreSQL database/schema,
  **not tenant-isolated by `OCAF_CLUSTER_ID`**. Keep independent
  security domains in separate databases or schemas.
- PostgreSQL itself remains a possible single point of failure unless
  you deploy, test and operate its high-availability infrastructure.
- The monitor doesn't provide distributed tracing, exact network
  SLO/SLA measurement, automatically fired external notifications,
  replicated alert storage, or exactly-once external side effects.
- Protect the monitor database account with **read-only SELECT grants**
  to the OCAF node/job tables, restrict host/network access, and verify
  real PostgreSQL certificate trust and certificate hostname checks.
  Do not publish the CLI stdout or process environment to untrusted
  readers.

## CI verification

`tests/cluster_observability.rs` checks deterministic alert evaluation,
leader visibility from real `pg_locks`, database-clock job counting,
credential-free output and per-node heartbeat inspection.
`tests/cluster_health_cli.rs` checks the actual CLI JSON output,
Prometheus samples, return-code alert behavior, and refusal of
unauthorized/unsupported argument patterns against PostgreSQL 16.
The existing signed Zenoh RPC and PostgreSQL job suite continues to run.
