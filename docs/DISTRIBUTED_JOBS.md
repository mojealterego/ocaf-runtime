# OCAF Runtime 0.8 — authenticated distributed jobs (experimental)

## Scope

The coordinator now exposes a durable job queue over **signed Zenoh RPC**.
Separate authenticated clients can submit jobs, while remote workers
poll for eligible jobs, execute only locally allowlisted tools and report
the result using a fencing token. A submitter can retrieve the status and
output of **its own** jobs. Requests and replies remain bound to an HMAC
peer identity.

This is *real networked pull-based job dispatch*, but it is **not**
a replicated scheduler, sandbox, certificate-enforced transport, or
exactly-once external side effect system. SQLite remains exclusively
on the single coordinator host.

## Protocol

| RPC action | Role | Behavior |
| --- | --- | --- |
| `SubmitJob { id, tool, args }` | `submit_jobs=true` and tool in ACL | Persist a one-attempt, non-retryable job with a UUID owned by the authenticated submitter |
| `GetJob { id }` | `submit_jobs=true` | Return current status and output **only to original submitter**; otherwise null |
| `ClaimJob` | `worker_jobs=true` | Recover expired leases and atomically claim the oldest pending job whose tool is in the worker's allowlist |
| `FinishJob { lease, outcome }` | `worker_jobs=true`; matching worker identity and tool ACL | Commit success/failure/uncertain only while the signed worker's lease token is active |
| `Heartbeat { tools }` | Authenticated peer with matching capability ACL | Register available tools for 15 seconds |

`RemoteJobOutcome` can be `Succeeded { output }`, `Failed` (a
definitive pre-execution rejection) or `Uncertain` (a tool might have
produced side effects, but outcome is not established).

**Remote jobs are always submitted with `retry_safe=false` and
`max_attempts=1`.** They are never automatically rerun after an
ambiguous crash or lost reply. Repeated submit attempts using the same
job UUID and payload return the existing job, but a different owner
cannot take over that identifier.

## Identity and authorization

Each peer has an independent 32-byte HMAC key. Operators must assign
roles explicitly in the private coordinator JSON file:

```json
{
  "submitter": {
    "key_b64": "<UNIQUE_BASE64_32_BYTE_KEY_A>",
    "allowed_tools": ["demo_echo"],
    "submit_jobs": true,
    "worker_jobs": false
  },
  "worker-a": {
    "key_b64": "<UNIQUE_BASE64_32_BYTE_KEY_B>",
    "allowed_tools": ["demo_echo"],
    "submit_jobs": false,
    "worker_jobs": true
  }
}
```

The coordinator refuses zero keys, reusing a key for separate peers,
unknown senders, unauthorised roles and off-ACL tool names. All
submission IDs must be UUIDs. The submitter identity is obtained only
from the verified signed request, never from user-supplied job data.
A durable `job_owners_v1` table prevents a different submitter from
reusing or reading an existing job UUID.

The coordinator process requires **three different absolute SQLite
paths**: `OCAF_REPLAY_DB_PATH`, `OCAF_COORDINATOR_DB_PATH` and
`OCAF_JOBS_DB_PATH`. No other process needs filesystem access to
the coordinator databases. The JSON file specified by `OCAF_PEERS_FILE`
must be owner-only (mode `0600` on Unix).

## Local loopback demonstration

Use three shells with **separate keys**, different node identities
and a Zenoh server/client configuration. An example loopback-only
server config exists at `examples/insecure-local/server.json5`;
a matching client config exists at
`examples/insecure-local/client.json5`.

Coordinator (with a private, configured `OCAF_PEERS_FILE`):

```bash
export OCAF_COORDINATOR_NODE_ID=coord
export OCAF_ZENOH_CONFIG=examples/insecure-local/server.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Unsafe loopback-only demonstration
export OCAF_COORDINATOR_LOCK_PATH=/absolute/private/path/leader.lock
export OCAF_COORDINATOR_MODE=active
export OCAF_PEERS_FILE=/absolute/private/path/peers.json
export OCAF_REPLAY_DB_PATH=/absolute/private/path/replay.sqlite
export OCAF_COORDINATOR_DB_PATH=/absolute/private/path/members.sqlite
export OCAF_JOBS_DB_PATH=/absolute/private/path/jobs.sqlite
cargo run --bin ocaf-coordinator
```

Worker shell (unique key from its coordinator peer entry):

```bash
export OCAF_ROLE=worker
export OCAF_NODE_ID=worker-a
export OCAF_PEER_ID=coord
export OCAF_SHARED_KEY_B64="<WORKER_32_BYTE_KEY_B64>"
export OCAF_ZENOH_CONFIG=examples/insecure-local/client.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Unsafe loopback-only demonstration
cargo run --bin ocaf-rpc-node
```

The demonstration worker advertises `demo_echo` every five seconds,
polls for available work and executes through `OcafRuntime` with
input/output JSON Schema validation and a 250 ms manifest timeout.

Submitter shell (different key assigned to `submitter`):

```bash
export OCAF_ROLE=client
export OCAF_NODE_ID=submitter
export OCAF_PEER_ID=coord
export OCAF_SHARED_KEY_B64="<SUBMITTER_32_BYTE_KEY_B64>"
export OCAF_ZENOH_CONFIG=examples/insecure-local/client.json5
export OCAF_ALLOW_INSECURE_LOOPBACK=1 # Unsafe loopback-only demonstration
export OCAF_ACTION=submit
export OCAF_MESSAGE="one durable remote job"
export OCAF_JOB_ID="$(cat /proc/sys/kernel/random/uuid)" # Linux demo
cargo run --bin ocaf-rpc-node

export OCAF_ACTION=status
cargo run --bin ocaf-rpc-node
```

Store the UUID safely before submitting. Reuse the *same* UUID for an
idempotent repeat submission; creating a new UUID is a new job and may
cause duplicate external effects. Client queries must use the same
authenticated identity as the original submitter.

## Failure and timing semantics

- Remote job leases are **30 seconds**. The built-in worker permits a
  tool only if its manifest timeout plus 4 seconds is less than the lease.
- The coordinator uses its own reception clock to issue and validate
  leases, not the worker's declared timestamp.
- Lease UUIDs and worker identity are checked by SQLite before result
  commit; stale reports cannot mutate the journal after expiration.
- The remote worker does not retry after an ambiguous failure. It
  reports `uncertain`; if a report is lost, the coordinator records
  `uncertain` on a later claim/recovery pass.
- An allowed worker can still lie about job outcomes. HMAC proves
  possession of its key, not that the external action happened. Workload
  attestation and sandboxing are not implemented.
- Sending job arguments or results over **plaintext TCP exposes them**.
  These sample files are intended for loopback tests only. Deploying
  across hosts requires separately verified TLS/mTLS or an encrypted
  private overlay; this application does **not enforce TLS itself**.
- The coordinator supports active/passive **process** takeover on the
  same host, but the machine/disk remains a **single point of failure**.
  Database replication, multi-region fault tolerance, independent worker
  recovery, global quotas and production-grade audit logs remain open.

## Verification

`tests/distributed_jobs_rpc.rs` exercises signed submission, owner-only
result access, UUID collision resistance, role enforcement, strict
claim ACL, lease-token fencing and crash expiry.

`tests/distributed_jobs_zenoh_e2e.rs` exercises independent Zenoh
coordinator, worker and submitter sessions over loopback TCP, executing a
real registered echo tool and inspecting the persisted result. This
does **not** establish a production multi-host or mTLS deployment.

As of v0.9 both CLI programs validate Zenoh security settings before
opening a session. The coordinator must hold an exclusive local leader
lock. Read [HA.md](HA.md) for required environment variables and limits.

## Next production gates

1. Enforced TLS/mTLS with identity binding and credential rotation.
2. Replicated coordinator database or a proper distributed job broker.
3. Sandboxed workers and per-tenant job size, storage and resource quotas.
4. Durable audit events, traces, retention policies and operator controls.
5. Network failure injection, crash-point fuzzing and cross-host deployment
   tests before any production claim.
