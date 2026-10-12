# OCAF Runtime — Rust / Tokio / Zenoh

**Status: working experimental distributed RPC prototype; not production hardened.**

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
| Two-session Zenoh TCP integration test | Automated test |
| TLS/mTLS transport | Operator-configured (not programmatically enforced) |
| Durable admission (at-most-once within TTL) | Implemented with SQLite; not exactly-once execution |
| Durable result cache, cross-host coordination, agent lifecycle | Not implemented |
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
export OCAF_REPLAY_DB_PATH="$(pwd)/ocaf-replay.sqlite"
cargo run --bin ocaf-rpc-node
```

**Client terminal (with the same shared key):**

```bash
export OCAF_ROLE=client
export OCAF_NODE_ID=agent-a
export OCAF_PEER_ID=agent-b
export OCAF_ZENOH_CONFIG=examples/insecure-local/client.json5
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

The RPC CLI **requires** `OCAF_ZENOH_CONFIG` and does not silently
fall back to Zenoh network defaults. Example configuration files are
plaintext loopback-only and must not be used as production settings.

## Wire contract

- Fixed keyspace: `ocaf/v1/nodes/{target-node}/rpc`.
- `RpcRequest` includes version, UUID correlation ID, sender,
  recipient, issuance time, short TTL and `Discover` / `Execute`.
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
- Size limits: 64 KiB frame, 32 KiB body; maximum request TTL: 30 s.
- Do not send personal/private data over the demo's plaintext transport.

See [docs/SECURITY.md](docs/SECURITY.md) for threat model and limitations.

## Production work still required

1. Enforced TLS/mTLS identity and secure configuration verification.
2. Persistent result journal, recoverable distributed job execution and cross-host consensus.
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
