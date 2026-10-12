# OCAF Runtime — Rust / Tokio / Zenoh

**Status: experimental signed Zenoh RPC, shared PostgreSQL job queue and cross-host advisory leadership primitives; full coordinator HA and replicated storage are not implemented.**

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
| PostgreSQL cross-host session advisory leadership | Implemented as separate library primitive; NOT yet service-integrated |
| Shared PostgreSQL remote job queue | Experimental, role-restricted and transactional; opt-in only |
| Cross-host coordinator failover / replicated consensus | Not implemented |
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

## PostgreSQL distributed coordination foundations (v0.10)

The `cluster::leadership::PgLeader` API supplies **cross-host leader
exclusion** using a PostgreSQL session advisory lock. Independent hosts
using one database and one cluster key cannot simultaneously acquire it.
A dead PostgreSQL session releases ownership. The same release introduces
`cluster::jobs::PgJobQueue`, which uses PostgreSQL transactions and
`FOR UPDATE SKIP LOCKED` for atomic job claims and persists owners, lease
tokens and results. The signed RPC gateway can opt into this queue using
`with_postgres_job_db` and the coordinator CLI can enable it explicitly
with `OCAF_EXPERIMENTAL_PG_JOBS=1` and `OCAF_PG_JOBS_DSN`.

**This is not yet a production multi-host cluster.** The coordinator CLI
still uses local SQLite for replay prevention and membership, and it
does **not** activate PostgreSQL leader election in its serving path.
Until those are migrated and serving is fenced on leadership loss, using
multiple simultaneous coordinator hosts is unsupported. PostgreSQL
failover/replication must also be provided and tested externally.

CI uses a real PostgreSQL 16 service for cross-connection regression
tests. See [docs/POSTGRES_CLUSTER.md](docs/POSTGRES_CLUSTER.md) for the
deployment boundary and the remaining cross-host safety requirements.

## Production work still required

1. Certificate-backed TLS/mTLS handshake tests on deployed hosts, plus key rotation and revocation.
2. Replicated job broker/coordinator, external-effect fencing and real **cross-host** high availability.
3. Signed plugin supply chain, sandbox and capability isolation.
4. Secret manager integration, peer/key rotation, revocation and quotas.
5. Distributed traces, fault injection, property tests and benchmarks.
6. Real L402 payment verification **only after** independent security review.

There are **no substantiated sub-millisecond latency claims** for this
prototype. Tests prove certain behavior, not all adversarial or
production network conditions.

## License

The crate declares MIT licensing in `Cargo.toml`. License/redistribution
status should be reviewed before public release.
