# OCAF 0.21 — independent durable effect receiver

## Delivery

`ocaf-effect-sink` is a **separate executable**, with an independent
SQLite database and its own Zenoh queryable:

`ocaf/v1/sinks/<sink-id>/effects`

This is the first opt-in destination outside the coordinator's PostgreSQL
transaction, not a general adapter for arbitrary HTTP/payment side
effects. The receiver stores immutable JSON events. It does not execute
shell commands, send emails or write to third-party systems.

A request is authenticated with HMAC-SHA256 over a bounded signed frame.
Controller and worker use **distinct 32-byte secrets** and distinct
identities. Nonces and the effect journal are durable in SQLite with
`synchronous=FULL`, WAL and a database write lock. The database is a
local file — back it up and restrict filesystem access.

## Authorization and fencing protocol

1. The trusted controller verifies its `PgLeader` session and sends a
   signed `ActivateEpoch {epoch,expires_at_ms}`. The receiver durably
   stores the highest observed epoch and a **max. 15-second** activation
   window. The controller cannot roll the epoch back. An isolated
   receiver ceases accepting writes once its authorization expires.
2. The trusted controller must separately authorize an exact job ID,
   specific worker identity, epoch, expiration and SHA-256 hash of the
   JSON effect. A `SinkPermit` is signed with the controller secret.
   `ZenohSinkClient::issue_permit` obtains the generation through
   `PgLeader::fencing_epoch` but the caller must independently enforce
   original job ACLs before issuing this capability.
3. The worker sends an authenticated `Apply` request carrying that
   controller-signed permit and an exact JSON body. Neither merely
   knowing the worker secret nor holding a stale lease can fabricate
   a new permitted payload.
4. Under one SQLite IMMEDIATE transaction the receiver checks the
   *current* active epoch, permit expiry, payload digest, worker ID,
   durable replay nonce and unique job ID, and commits the event.
   Fresh requests repeating an identical job payload receive
   `applied=false` and the original timestamp; conflicting payloads
   return `conflict`.
5. The sink sends a signed response bound to the caller's request ID,
   sink ID and identity. There is no blind automatic retry of writes.

## Executable configuration

Build with `cargo build --release --bin ocaf-effect-sink`.

Set:

```sh
OCAF_SINK_ID=external-sink-a
OCAF_SINK_DB_PATH=/var/lib/ocaf-sink/events.sqlite
OCAF_SINK_CONTROLLER_ID=controller-a
OCAF_SINK_WORKER_ID=worker-a
OCAF_SINK_CONTROLLER_KEY_B64=<unique-base64-secret-32-bytes>
OCAF_SINK_WORKER_KEY_B64=<different-base64-secret-32-bytes>
OCAF_ZENOH_CONFIG=/etc/ocaf/sink-zenoh.json5
```

Run `ocaf-effect-sink` under a dedicated unprivileged service identity.
The SQLite directory must not be group/world writable; the database
file is owner-readable/writable only. Apply systemd sandboxing and
suitable operational backups. The Zenoh configuration is subject to
the existing `validate_zenoh_config` preflight, including TLS **and
mTLS** for remote endpoints. Plain TCP is only allowed when
`OCAF_ALLOW_INSECURE_LOOPBACK=1` and every endpoint is loopback.
Never deploy plain TCP outside a development host.

An integrator uses `ZenohSinkClient` to activate an epoch and deliver
permitted writes; the sink server is intentionally separate from
`ocaf-coordinator` and `ocaf-rpc-node`. The default job worker
does **not** automatically deliver arbitrary tools to this sink.

## Verification and limitations

- `cargo test --no-default-features --test independent_sink` exercises
  durable restart/replay, monotonic fencing, signature tampering,
  authorization expiry and idempotency.
- `cargo test --test sink_zenoh_e2e` exercises the real Zenoh queryable
  with distinct controller/worker sessions and PostgreSQL leader
  succession. CI runs these against a PostgreSQL 16 service.
- For a physical multi-host validation, run the sink, worker and
  coordinator on **separate hosts**, with explicit TLS/mTLS, collect
  packet-loss and partition evidence and validate database recovery.

**Trust model and residual risk:** This sink is not a distributed
consensus system. Between losing the old PostgreSQL leader and receiving
a higher-epoch activation (or expiration of the 15-second local epoch
window), an isolated sink may still accept valid *previously issued*
old-epoch permits. Stale deliveries are rejected **after** new-epoch
activation or expiration. A compromised controller key can issue
new permits; rotate secrets and isolate that key. An external API must
implement its own atomic fencing/idempotency adapter; an ACK from the
journal is not proof the third-party effect occurred. Clock correctness
and a single authoritative PostgreSQL leader remain operational
assumptions. Do not claim end-to-end exactly-once execution or
split-brain immunity.
