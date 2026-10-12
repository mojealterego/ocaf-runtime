# Security architecture and threat boundaries

## Implemented controls

- Per-peer HMAC-SHA256 signatures over the **serialized payload bytes**.
  The unsigned sender claim is used only for key lookup. Execution follows
  successful verification using that peer's key.
- Per-peer exact tool allowlist. Unknown peers and denied tools fail closed.
- Authenticated coordinator heartbeats bind membership to the verified RPC
  sender; capabilities must be a subset of the administrator-assigned peer
  ACL. Lookups require the requester's ACL for that capability.
- Configurable fixed-window quotas on authenticated RPC requests (default 64
  messages per peer per second); exhausted quotas reject messages before
  resource discovery or tool execution.
- Remote calls are restricted to **enabled, free tools without elevated
  permissions**. Paid tools, permissioned tools and L402 authorization
  credentials are not transported through this RPC layer.
- Node IDs are strict alphanumeric, underscore and hyphen; routing paths
  cannot contain wildcard or path delimiters.
- Versioned envelopes, UUID request IDs, short validity windows, clock
  skew checks, durable SQLite replay admission (RPC CLI), bounded replay cache,
  bounded HMAC frame/body sizes and concurrency limit.
- The v0.9 CLI transport preflight enforces explicit Zenoh TLS-only/mTLS
  settings and existence/permissions of credential files; plaintext is
  rejected except opted-in loopback, with multicast/gossip disabled.
- Single-host coordinator active/passive process ownership uses an
  exclusive OS file lock that is released on process death. Local standby
  promotes only after obtaining that lock.
- Replies are signed with the appropriate peer secret and the client
  validates sender, recipient, protocol version and correlation ID.
- Trusted local jobs use SQLite WAL, idempotent UUID submission, atomic lease
  claims, worker fencing tokens, bounded retry counts and explicit uncertain
  state after unsafe or exhausted expired work. Jobs run through the
  same runtime authorization pathway. Since v0.8 a dedicated signed remote
  job API exists, with separate explicit submitter/worker roles.
- Runtime enforces JSON Schema validation, execution timeouts,
  cancellation-aware atomic budget accounting and controlled errors.

## Not guaranteed

| Threat / property | Current state | Required production control |
| --- | --- | --- |
| Passive traffic observation | CLI refuses plaintext except explicitly opted-in loopback; library users must enforce transport themselves | Real TLS/mTLS handshake tests, cert lifecycle and endpoint validation |
| Credential theft on disk/memory | Key is read from environment; no hardware protection | Secret manager, ephemeral credential issuance, rotation |
| Replays after process restart | SQLite WAL journal committed before execution in RPC server CLI | Verify durable storage, backups and local filesystem semantics |
| Unauthorized execution by authorized peer | Exact allowlist limits tools; allowed code may still perform side effects | Sandboxed tools, fine-grained capability policy and quotas |
| Duplicate execution when response is lost | Same request ID is refused even after restart, but outcome is unknown after crash | Durable results, transactional outbox and per-tool idempotency |
| Network denial of service | Bounded frames/concurrency and per-peer fixed-window quotas; not full DDoS defense | Edge controls, distributed quotas and network backpressure |
| Agent identity | Pairwise shared secret authenticates a peer possessing that key | Workload PKI, mTLS identity binding, key rotation |
| Real L402 payment authorization | Not implemented | Verified invoice/hash/macaroon/caveats, ledger, replay protection |
| Tool isolation | Tools run in the same OS process | sandbox, seccomp, capability boundaries and quotas |
| End-to-end trace/observability | Basic in-process metrics | OpenTelemetry, audit log and incident response |
| Queue failover | On-host active/passive lock and takeover; single-host SQLite | Cross-host replicated coordinator, fencing at external services |
| Agent membership / discovery | Durable SQLite membership with 15-second expiry and signed RPC | Replicated directory, endpoint attestation and active health probes |
| Remote node identity | Signed HMAC key possession, no transport certificate enforcement | Workload mTLS and certificate rotation/revocation |
| Expired worker external side effects | Journal fencing prevents stale commits, not external writes | Transactional outbox, external fencing and idempotency |
| Multi-router trust | Not independently verified | mTLS Zenoh config and authenticated router topology |

**HMAC signs; it does not encrypt**. A Zenoh config file being present
does not prove that TLS/mTLS is enabled; deployments must independently
verify the actual transport security. The demo configuration files in
`examples/insecure-local` are plaintext loopback-only.

## Durable replay journal (v0.5.0)

The RPC server CLI requires `OCAF_REPLAY_DB_PATH` to point to an absolute,
writable SQLite database path. Startup fails if the database cannot be
initialized; it never falls back to the volatile in-memory implementation.
The runtime records each signed request's recipient, sender, UUID and
expiration before invoking a tool. Claims use an IMMEDIATE SQLite transaction
and a unique composite key. Journal mode is WAL with synchronous FULL.

The `RpcGateway::new` library constructor still uses an in-memory cache
for explicitly embedded deployments and tests; use
`RpcGateway::with_sqlite_replay` for crash-resistant admission checks.

Guarantee: at most one **admitted attempt** per (node, peer, UUID) within
the authenticated request's validity window on a single shared SQLite
database. This does **not** guarantee that the underlying tool ran,
completed, or committed its own external side effects. A crash can leave
the result unknown and the same UUID blocked. The journal does not store
tool inputs, outputs, or session secrets.

The SQLite file is **not** a distributed consensus store and must not be
placed on unreliable shared network filesystems. Use a proper distributed
database and transactional processing protocol before deploying active
replicas across separate hosts. Keep the database and WAL sidecars private,
exclude them from backups that leak customer metadata, and size capacity
for a maximum of 4,096 live journal entries (full => fail closed).

## Coordinator trust boundaries

The experimental coordinator uses a separate persistent local SQLite
membership database and a signed, replay-checked Zenoh RPC request. Agents
can register only their own sender identity and authorized tool names.
Requester capability lookups are individually permission-checked and
rotate fairly across live heartbeat entries. A heartbeat is a claim of
availability, **not proof of a healthy tool executor**. The coordinator
is still a single host. Capability lookup responses do not implement
job-lease transfer, but the separate signed job RPC does provide fenced
remote assignment on the coordinator's single-host SQLite queue. See
[COORDINATOR.md](COORDINATOR.md) and
[DISTRIBUTED_JOBS.md](DISTRIBUTED_JOBS.md).

## Job queue boundaries

The experimental `JobStore` and `JobWorker` modules persist local
work, results, attempts, leases and recovery status in SQLite. They
are exposed over RPC **only after peer HMAC verification and explicit
job role and tool ACL checks**. Submitter ownership is persisted so an
unrelated peer cannot read another submitter's results or take over its
UUID. Remote jobs are single-attempt and non-retryable by construction.
Only a trusted local publisher can decide that an operation is retry-safe; marking
an operation retry-safe is not a generic guarantee of idempotency.
A stale lease token can never commit a result to this journal, but the
stale worker might still mutate an external system. See
[docs/JOBS.md](JOBS.md) for operational constraints and recovery policy.

## Distributed worker boundaries (v0.8)

The remote worker pulls work with its coordinator-specific signing key and
executes it only through the existing, deny-by-default runtime. Work is
restricted to enabled free tools with no special permissions. A 30-second
coordinator-issued lease and worker identity are checked before result
storage; malformed or forged reports fail closed. In ambiguous failures,
non-idempotent jobs become `uncertain` and require operator reconciliation.
The coordinator is a single point of failure, no network confidentiality
is provided without independently configured transport security, and
remote workers are not sandboxed or attested.

## Active/passive lock boundary (v0.9)

The coordinator CLI requires `OCAF_COORDINATOR_LOCK_PATH`. A single
active process holds an OS-exclusive lock; a local standby waits without
opening its Zenoh session or SQLite stores until that lock releases.
This protects multiple **processes on one host**, not split-brain across
machines or shared network storage. The leader lock inode must never be
unlinked, and local storage must have a private parent directory. See
[HA.md](HA.md).

## Keys

Generate a distinct, cryptographically random 32-byte key per two-peer
relationship, provision through an appropriate secret manager, and inject
only at launch. The example CLI uses `OCAF_SHARED_KEY_B64`. Never commit
real keys or print them in logs. Rotation and automatic revocation are
not part of this implementation.

## Future security acceptance gates

1. A complete mutual TLS deployment test with unauthorized peers blocked.
2. Extend the admission ledger into a transactional result/outbox journal and simulate process crashes.
3. Cluster job coordination, external effect fencing, per-tenant budgets and structured audited denials.
4. Sandbox integration for untrusted tools and bounded CPU/memory/network.
5. Dependency and supply-chain audits, fuzzing of signed envelope parser.
6. Cross-machine test under network disruption, latency and packet loss.
7. External security assessment and documented recovery/runbook.

## Vulnerability reporting

Do not disclose exploitable credentials or private infrastructure details in
public GitHub issues. Use the repository owner's private reporting channel.
