# OCAF Runtime — Rust / Tokio / Zenoh

**Status: experimental PostgreSQL-backed active/passive coordinator, signed Zenoh RPC and durable distributed jobs. Production-certified HA is not established.**

OCAF provides an async local tool registry and execution engine, plus a
**real Zenoh query/reply protocol** for pairwise authenticated agent calls.
The protocol uses HMAC-SHA256, a per-peer tool allowlist, bounded frames,
request expiration and either an in-memory or durable SQLite replay journal. Remote execution is
deliberately limited to **free, enabled tools without special permissions**.

## Capabilities

| Component | Status |
| --- | --- |
| Rust/Tokio in-process tool execution | Implemented |
| Manifest registry and JSON Schema input/output validation | Implemented |
| Local timeouts, atomic budget escrow and accounting | Implemented |
| Versioned Zenoh remote query/reply | Implemented |
| Pairwise HMAC-SHA256 authenticated request/response | Implemented |
| Exact tool allowlist and signed tool discovery | Implemented |
| SQLite WAL replay journal / expiration / bounded concurrency | Implemented; mandatory in RPC server CLI |
| Authenticated peer-specific RPC rate limit | Default 64/s; tunable through PeerPolicy |
| Two-session Zenoh TCP integration test | Automated test |
| TLS/mTLS transport policy | Fail-closed CLI preflight; requires TLS-only/mTLS settings and certificate files |
| Durable admission (at-most-once within TTL) | Implemented with SQLite; not exactly-once execution |
| SQLite job queue, worker lease/fencing, restart recovery | Implemented for one host |
| Authenticated agent membership, 15-second leases, capability routing | Implemented (single-host coordinator) |
| Signed remote job submit/pull/finish and owner-only results | Implemented; single coordinator |
| On-host coordinator active/passive leadership | Implemented with exclusive OS file lock and standby takeover |
| PostgreSQL cross-host advisory leadership | Integrated into opt-in active/passive coordinator service |
| Shared PostgreSQL remote job queue | Experimental, role-restricted and transactional; opt-in only |
| Shared PostgreSQL replay and agent directory | Implemented for opt-in PostgreSQL cluster mode |
| Cross-host active/passive process promotion | Experimental; requires a shared PostgreSQL primary and external stable endpoint |
| Built-in PostgreSQL replication / Raft consensus | Not implemented |
| L402 Lightning payments and cryptographic payment validation | Not implemented |
| Untrusted executable tool sandbox and production rate limiting | Not implemented |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --no-default-features
cargo test
cargo run --bin ocaf_runtime
```

The library builds without Zenoh using `--no-default-features`. The
`ocaf-rpc-node` binary requires the Zenoh feature.

## Local two-process RPC demo

**Security warning:** The following files use plaintext TCP bound to
`127.0.0.1`. This is a *local-only test*, not secure remote deployment.

Provision a shared 32-byte key to each process, for example:

```bash
export OCAF_SHARED_KEY_B64="$(openssl rand -base64 32)"
```

Copy that environment variable securely into both local shell sessions.
Do not check the key into source control or expose it in screenshots.

**Server terminal:**

```bash
export OCAF_ROLE=server
export OCAF_NODE_ID=agent-b
export OCAF_PEER_ID=agent-a
export OCAF_ZENOH_CONFIG=examples/insecure-local/server.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Local-only unsafe demo
export OCAF_REPLAY_DB_PATH="$(pwd)/ocaf-replay.sqlite"
cargo run --bin ocaf-rpc-node
```

**Client terminal (with the same shared key):**

```bash
export OCAF_ROLE=client
export OCAF_NODE_ID=agent-a
export OCAF_PEER_ID=agent-b
export OCAF_ZENOH_CONFIG=examples/insecure-local/client.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Local-only unsafe demo
export OCAF_ACTION=discover
cargo run --bin ocaf-rpc-node
export OCAF_ACTION=call
export OCAF_MESSAGE="authenticated request over Zenoh"
cargo run --bin ocaf-rpc-node
```

The server exports only `demo_echo`; it executes through the real
`OcafRuntime` after verifying the caller's signature, ID, TTL, replay
status, ACL and manifest safety. Replies are independently signed.
Calling unknown/paid/restricted tools is denied.

## Deploying across machines

Use a reviewed Zenoh JSON5 configuration via `OCAF_ZENOH_CONFIG` for
both peers, with reachability and an authenticated encrypted transport
(e.g. configured TLS/mTLS). The **presence of a config file is NOT proof**
of TLS or peer identity. Confirm effective Zenoh transport behavior and
certificates in the deployment environment. The in-process HMAC
authenticator offers integrity and peer-key authentication, **not**
confidentiality.

Both RPC and coordinator CLIs **require** `OCAF_ZENOH_CONFIG` and run a
fail-closed transport policy validator. Secure mode requires explicit
TLS-only endpoints/protocol whitelist and mTLS certificate settings.
Plaintext TCP is permitted only with the explicit development flag
`OCAF_ALLOW_INSECURE_LOOPBACK=1` and literal loopback endpoints;
non-loopback TCP is rejected even with that flag. These checks do not
replace actual Zenoh certificate handshake verification. See
[docs/HA.md](docs/HA.md) and [examples/secure-mtls](examples/secure-mtls).


## Wire contract

- Fixed keyspace: `ocaf/v1/nodes/{target-node}/rpc`.
- `RpcRequest` includes version, UUID correlation ID, sender,
  recipient, issuance time, short TTL and signed discovery, execution,
  coordinator and job control actions.
- The body bytes are base64-encoded inside a JSON envelope and signed
  using per-peer HMAC-SHA256. Both sides verify the signature and ID
  binding. No arbitrary tool code is transferred.
- The library can use volatile replay protection, but the RPC server CLI
  requires an absolute `OCAF_REPLAY_DB_PATH` and uses durable SQLite WAL.
  It commits an admission claim before execution; repeating a signed request
  ID after restart is rejected within its TTL window. It is **not exactly-once
  execution**. If the server crashes after admission but before a tool finishes,
  the outcome may be unknown and retries with the same ID are denied.
  SQLite is intended for a single host or replicas sharing the same on-disk
  filesystem; **it is not a cross-host consensus or distributed database**.
- Per-peer authenticated RPC quota defaults to 64 requests/s per gateway;
  `PeerPolicy::with_rate_limit` can set an explicit 1–10,000 requests/s.
  Quotas are fixed-window and per-process, not globally synchronized.
- Size limits: 64 KiB frame, 32 KiB body; maximum request TTL: 30 s.
- Do not send personal/private data over the demo's plaintext transport.

See [docs/SECURITY.md](docs/SECURITY.md) for threat model and limitations.

## Durable local jobs (v0.6)

The `JobStore` module implements persistent FIFO work admission on SQLite
with UUID-based idempotent submission, atomic claims, lease fencing,
result persistence and retry policies. `JobWorker` integrates claims with
the existing runtime and requires an explicit tool allowlist; paid and
permissioned tools remain blocked. Expired safe-to-retry jobs can be
reclaimed; otherwise they become `uncertain` pending manual investigation.

**A lease does not provide exactly-once side effects.** Mark jobs
`retry_safe` only when operations are actually idempotent. This is
single-host orchestration, not cluster consensus. As of v0.8, a separate
**signed, role-restricted remote job API** uses this SQLite store.

See [docs/JOBS.md](docs/JOBS.md) for API examples, persistence guarantees,
operating requirements and remaining architectural work.

## Authenticated capability coordinator (v0.7)

The standalone `ocaf-coordinator` service accepts signed agent heartbeats
and capability lookups over Zenoh. Each agent advertises only tools explicitly
authorized in its administrator-configured HMAC peer policy. The persistent
SQLite directory expires membership after 15 seconds without a heartbeat
and rotates lookups fairly among currently advertised nodes.

The demo RPC server can automatically renew its membership every five
seconds when `OCAF_COORDINATOR_NODE_ID` and `OCAF_COORDINATOR_KEY_B64`
are provisioned. HMAC does not encrypt traffic; the operator must separately
enforce verified Zenoh TLS/mTLS for non-loopback deployments.

See [docs/COORDINATOR.md](docs/COORDINATOR.md) for the separate daemon,
peer provisioning, acceptance tests, and important non-production limitations.

## Distributed job orchestration (v0.8)

The coordinator now accepts signed `SubmitJob` from explicitly designated
submitters, assigns queued work via `ClaimJob` to allowlisted remote workers,
and accepts fenced `FinishJob` reports. Only the original authenticated
submitter can retrieve its result through `GetJob`. Distinct submitter and
worker permissions are configured in `OCAF_PEERS_FILE`.

The `ocaf-rpc-node` CLI now supports `OCAF_ROLE=worker` (polling executor)
and `OCAF_ACTION=submit` / `OCAF_ACTION=status` in client mode. The
coordinator requires `OCAF_JOBS_DB_PATH` in addition to the previous two
SQLite paths.

Remote jobs are limited to one attempt, are never retried automatically
after ambiguous failures, and may enter the terminal `uncertain` state.
HMAC authenticates peers but does not encrypt job arguments or results.
There is **no replicated scheduler, enforced TLS/mTLS or exactly-once side
effect guarantee**.

See [docs/DISTRIBUTED_JOBS.md](docs/DISTRIBUTED_JOBS.md) for the complete
protocol, role ACLs, local demo and security boundaries.

## Local active/passive coordinator and transport preflight (v0.9)

The coordinator now requires an absolute `OCAF_COORDINATOR_LOCK_PATH`
in the same local directory as its SQLite job database. It refuses to
serve without exclusive ownership of the process-held OS file lock.
A second coordinator started with `OCAF_COORDINATOR_MODE=standby`
waits and promotes after the original leader releases the lock, including
abnormal process exit. Both instances must share the **same local host,
local disk, endpoint, databases and node identity**; external process
supervision is required. This is not HA for a failed machine or a
replicated multi-server cluster.

See [docs/HA.md](docs/HA.md) for configuration, security acceptance tests,
limits, and the next cross-host architecture milestones.

## PostgreSQL active/passive coordinator (v0.11)

Set `OCAF_CLUSTER_MODE=postgres` to enable the standalone coordinator's
PostgreSQL-backed path. The gateway stores its replay ledger, capability
directory and remote-job queue in a **single shared PostgreSQL primary**,
while a dedicated session advisory lock elects one active coordinator
across hosts. The standby does not open its Zenoh listener until the
database lock is available. A per-request leadership check and periodic
monitor terminate the service on detected loss of leadership.

Local SQLite mode remains available unchanged. The PostgreSQL cluster
mode requires `OCAF_CLUSTER_ID`, `OCAF_CLUSTER_PG_DSN`, existing secure
Zenoh and peer settings, and `OCAF_COORDINATOR_MODE=active` or `standby`.
Neither `OCAF_COORDINATOR_LOCK_PATH` nor local SQLite paths are required
in this mode. There is **no automatic migration** of existing SQLite
data.

Real PostgreSQL integration tests check concurrent leadership, atomic
job claims, shared replay rejection, capability expiry, signed gateway
routing, loss-of-leadership denial and real Zenoh-to-PostgreSQL job
execution.

**Production gaps:** PostgreSQL HA/failover and replication, a stable
network endpoint for promoted nodes, actual multi-host outage
injection, operational PKI testing, strong external-effect fencing,
certificate rotation and independent security review. The runtime
does not supply Raft or eliminate the possibility of
`uncertain` external side effects.

See [docs/POSTGRES_CLUSTER.md](docs/POSTGRES_CLUSTER.md) for deployment
configuration, acceptance tests and threat boundaries.

## Transactional audit and crash fault injection (v0.12)

The PostgreSQL job backend now records metadata-only submission, claim,
completion and crash-recovery events in the same transaction as the
associated job-state transition. Signed `GetJobAudit { id, limit }` allows
an authenticated original submitter (not a worker or another submitter)
to inspect at most 100 audit entries. The log does **not** contain tool
inputs, outputs, credentials or raw lease tokens. Legacy SQLite mode
rejects this audit operation instead of inventing audit history.

PostgreSQL fault-injection tests terminate the active leader's actual
database backend using `pg_terminate_backend`, verify standby
promotion, deny requests through the previous leader and ensure the
submitted task and replay-admission record survive the promotion.
A separate test verifies stale completion cannot turn an expired,
`uncertain` job into a successful one.

These are PostgreSQL **session-failure** experiments run in CI; they do
not establish physical cross-host failover, distributed partition safety,
production mTLS validation or exactly-once external effects.
See [docs/RESILIENCE.md](docs/RESILIENCE.md).

## Cluster observability and operator controls (v0.13)

The new **read-only operator CLI**, `ocaf-cluster-health`, collects
PostgreSQL-clock snapshots of advisory leader sessions, live/expired
agent heartbeat leases, durable jobs by status, and expired running
leases. It prints **JSON reports or label-free Prometheus-style metrics**
to stdout and returns 0 (healthy), 1 (warning), or 2 (critical).
`--no-fail` preserves collection success exit codes for external
exporters. Alert thresholds are configurable through environment
variables. `--node <ID>` reports a single agent's lease status only
in protected JSON output.

The CLI does not expose HTTP and does not publish signing keys,
job arguments, results, tool ACLs, or lease tokens. It requires a
provisioned cluster database and operator-approved PostgreSQL
credentials. The cluster ID selects the **advisory leader key**, not
a tenant namespace; use an isolated PostgreSQL database or schema
for each trust domain.

This is **operational visibility, not automatic repair**. Heartbeat
leases do not prove that workers are reachable; PostgreSQL advisory
locks do not prove client routing works; and external notifications,
database replication and host/network-partition chaos testing remain
operator responsibilities.

See [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) for usage, thresholds,
exit codes, safeguards and CI coverage.

## Stable-endpoint reconnect and deployment hardening (v0.14)

The `ocaf-rpc-node` **client** role now uses a reconnecting
authenticated Zenoh transport. It recreates a vetted Zenoh session
with bounded exponential backoff after transport failure and permits
automatic retry **only** for read-only discovery, owner-only job status
and audit, or UUID-idempotent job submission. Remote tool `Execute`,
worker `ClaimJob`, `FinishJob` and `Heartbeat` are **never**
automatically retried, because a lost reply may conceal side effects
or an active lease. No retries are unbounded.

`OCAF_RECONNECT_ATTEMPTS` controls the attempt count (1–8, default 4).
`OCAF_ACTION=submit` now **requires a stable, explicit
`OCAF_JOB_ID`**, reused for every attempt and invocation. Do not
generate a new job UUID when a previous response is ambiguous.

The coordinator validates its PostgreSQL leader session **again just
before sending a signed reply**. This reduces stale-leader
acknowledgments but cannot undo effects that happened during an
in-flight request or replace external-system fencing.

New hardened systemd coordinator and health units are in
[deploy/systemd](deploy/systemd). They are templates requiring an
operator-provisioned database, secure Zenoh/mTLS PKI, secret files,
privileged endpoint routing and supervision. Tests exercise bounded
retry semantics and a real Zenoh server starting after the client.

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for security assumptions,
commands and remaining machine-level verification work.

## Cryptographic Zenoh mTLS integration gates (v0.15)

The full-feature integration suite now constructs temporary private
certification authorities and certificates with OpenSSL, validates the
operator's TLS-only configuration with the exact CLI preflight, and
executes **real signed Zenoh RPC over mutually authenticated TLS**.
It rejects clients presenting certificates issued by a different CA
and clients trusting an unrelated server CA. Test credentials are
generated at runtime, **never committed** as long-lived secrets.

This checks actual TLS handshakes rather than configuration shape
alone. However, CI still runs on one loopback host, not on physically
separate machines; operational PKI trust, certificate rotation,
revocation, DNS failover, live PostgreSQL failover and multi-host
network partition scenarios remain unverified.

See [docs/MTLS_VERIFICATION.md](docs/MTLS_VERIFICATION.md) and
[tests/zenoh_mtls_e2e.rs](tests/zenoh_mtls_e2e.rs).

## PostgreSQL-authoritative job lease clock (v0.16)

PostgreSQL queue claims, expiry recovery and fenced job completion now use
the **database server clock** within SQL, not the coordinator host clock.
This prevents skewed active/standby coordinators from changing lease
expiration decisions. The existing `now_ms` API argument remains for
compatibility but is ignored in the PostgreSQL backend (not SQLite).
Tests cover forward/backward host-clock skew and database-side expiry.

This does not replace PostgreSQL HA, operational clock synchronization,
host-level fault injection or external-system fencing.

See [docs/LEASE_CLOCK.md](docs/LEASE_CLOCK.md).

## Production work still required

1. Certificate-backed TLS/mTLS handshakes on separate deployed hosts, plus certificate rotation and revocation.
2. PostgreSQL HA replication and external-effect fencing; real **cross-host** fault/partition certification.
3. Signed plugin supply chain, sandbox and capability isolation.
4. Secret manager integration, peer/key rotation, revocation and quotas.
5. Distributed traces, whole-host/partition fault injection, property tests and benchmarks.
6. Real L402 payment verification **only after** independent security review.

There are **no substantiated sub-millisecond latency claims** for this
prototype. Tests prove certain behavior, not all adversarial or
production network conditions.

## License

The crate declares MIT licensing in `Cargo.toml`. License/redistribution
status should be reviewed before public release.
