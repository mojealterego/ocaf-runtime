# Durable jobs and crash recovery — OCAF 0.6

This module is an **embedded, single-host SQLite job orchestration layer**.
It is not yet a distributed scheduler or a replacement for a fault-tolerant
database. It is intended for a trusted local supervisor managing OCAF agent
work. Remote peers currently **cannot submit arbitrary jobs over Zenoh RPC**.

## State machine

```text
submit -> pending --claim/lease--> running --succeed--> succeeded
                 ^                  |
                 |                  +--definitive failure--> failed
                 |                  |    \--retry-safe with budget--> pending
                 |                  |
                 |                  +--lease expiry:
                 |                       retry_safe and attempts left -> pending
                 |                       otherwise -> uncertain
                 +-------------------------------------------+
```

The `uncertain` state is terminal for the automatic worker. It means a
worker disappeared, its lease expired, or a non-retry-safe tool returned an
ambiguous execution error after external side effects might have occurred. An
operator must reconcile the effect before submitting a *new* job UUID.

## Core guarantees

- SQLite WAL with `synchronous=FULL`; writes execute in `spawn_blocking`
  and use IMMEDIATE transactions for queue admission, claim and recovery.
- Job UUID is an idempotent **submission identifier**. Identical
  submissions return the existing record, while reusing a UUID with
  different tool, arguments or retry policy returns a conflict.
- Only a single pending job can be claimed by one worker at a time,
  including independent SQLite connections to the same database file.
- Each lease has a UUID fencing token and expiry. A worker holding a
  previous token cannot commit results, failures or heartbeats after
  losing its lease or after task reassignment.
- Worker classifies uncertain tool errors (timeout, executor error or invalid
  output) as terminal `uncertain` for non-idempotent jobs rather than
  incorrectly recording a definitive failure.
- Worker performs crash recovery before every claim. Expired work that
  is explicitly retry-safe may be retried subject to `max_attempts`;
  otherwise the job is marked `uncertain` and never auto-restarted.
- Worker uses normal `OcafRuntime::execute` (auth, schemas, timeout).
  It denies paid, privileged, disabled, non-allowlisted and unknown
  tools. Local jobs cannot provide arbitrary authentication credentials.
- Worker requires the manifest timeout plus a safety margin to be
  shorter than its lease. The lease is never allowed to exceed five
  minutes and must be at least 100 ms.

**Not guaranteed:** exactly-once external effects; cancellation of a
running tool after a lost lease; replication; recovery of completed results
not committed before process failure; fair scheduling across peers; global
rate limits; or distributed leader election. Fencing guards the
**SQLite journal**, not resources the tool modifies. Marking a tool
`retry_safe=true` is a privileged author/operator decision: setting it
for non-idempotent operations can duplicate real-world effects.

## API

```rust
use std::collections::HashSet;
use ocaf_runtime::jobs::{JobSpec, JobStore};
use ocaf_runtime::jobs::worker::JobWorker;
use ocaf_runtime::runtime::OcafRuntime;

// Use a dedicated directory with restrictive permissions.
let store = JobStore::open(std::path::Path::new("/var/lib/ocaf/jobs.sqlite"))?;
let task = JobSpec {
    id: uuid::Uuid::new_v4().to_string(),
    tool: "safe_echo".into(),
    args: serde_json::json!({"message": "hello"}),
    retry_safe: true,   // ONLY for genuinely idempotent tools
    max_attempts: 3,
};
store.submit(task).await?;

// The tool must already be registered with a concrete ToolExecutor in runtime.
// Do not use this sample's empty runtime without registering a tool.
let runtime = OcafRuntime::new(0);
let worker = JobWorker::new(
    store,
    runtime,
    "local-worker-1",
    10_000,
    HashSet::from(["safe_echo".into()]),
)?;
worker.run_once().await?;
```

The example demonstrates public API usage and does not start a background
daemon. `run_once` executes at most one queued task. A trusted
supervisor may invoke it repeatedly. Integration tests register real echo
executors, verify persisted results, simulate an abandoned lease, enforce
ACLs, and exercise competing workers.

## Operational requirements

Keep the SQLite database and its WAL/SHM files on a reliable **local disk**
and restrict OS file and directory permissions. On Unix, the job-store opener
creates the SQLite file with mode `0600` and tightens permissions of an
existing database; this is not a substitute for a private parent directory
or secure backups. Do not share this DB across
multiple hosts on a network filesystem. Each process requires write access
to the same SQLite file for on-host work stealing. Set alerts for stuck
`uncertain` jobs and storage errors, and create a documented reconciliation
procedure. Protect database backups, because job arguments/results may be
sensitive.

## Next production milestones

- Enforced mTLS workload identity, certificate rotation and revocation.
- Distributed, replicated scheduler / durable queue outside a single host.
- Leases bound to a trusted monotonic coordinator and fencing propagated
  to external side-effecting services.
- Tool sandboxing and per-tool CPU/memory/network restrictions.
- Durable audit events, per-tenant quotas, metrics and trace correlation.
- Real-world disaster recovery and multi-host chaos tests.
