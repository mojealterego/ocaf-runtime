# OCAF Runtime — Rust / Tokio / optional Zenoh

**Status: experimental.** This repository provides an in-process tool
registry and execution core. Zenoh currently opens a session only.
It does **not** yet expose a working distributed tool RPC protocol,
dynamic code loading, Lightning payments, or production L402 verification.

## Components

| Module | Function |
| --- | --- |
| `registry` | Register tools, reject duplicates, compile local-only JSON Schemas |
| `executor` | Async tool executor interface and execution context |
| `runtime` | Input/output validation, authorizer gate, timeout, atomic budget escrow |
| `auth` | Pluggable verification contract, **deny-by-default** implementation |
| `metrics` | Success/failure counters and total elapsed microseconds |
| `transport` | Optional Zenoh session opening and a local echo demo |

## Security boundaries

- **Paid tools and permissioned tools are rejected by default**, even if
  non-empty macaroon, preimage, invoice and nonce strings are supplied.
- Only a production implementation of `AuthVerifier` that validates
  cryptographic signatures, caveats, payment preimages/payment hashes,
  invoice bindings, identity, replay and revocation rules may authorize
  sensitive calls. **No such provider is included.**
- Free tools without permissions can run without credentials.
  They are therefore **public within the invoking process**. Do not
  expose them remotely without authentication or network access controls.
- JSON Schema input and output checks are enabled; external `$ref`
  references are rejected. Registration compiles validators in memory.
- Budgets are reserved atomically and refunded on failure, timeout or
  task cancellation; a successful tool commits the charge.
  This is **local resource accounting, not real Lightning settlement**.
- The runtime does not yet enforce authenticated agent identities,
  durable deduplication, exactly-once execution, sandboxed tools, tool
  provenance, trusted remote transport, or per-agent rate limits.

## Build & test

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --no-default-features
cargo test
cargo run
```

By default, Cargo compiles the optional Zenoh transport. For core-only
development, use `--no-default-features`. The demo uses a free local
echo tool and does not simulate a paid L402 transaction.

## Example

```rust
use ocaf_runtime::runtime::{OcafRuntime, ToolCallRequest};
// Implement ToolExecutor, declare a valid ToolManifest, then:
// runtime.register(manifest, executor).await?;
// runtime.execute(ToolCallRequest { ... }).await?;
```

The example is illustrative; see `src/main.rs` and
`tests/runtime_regressions.rs` for executable examples.

## Roadmap / acceptance criteria

1. **Core correctness:** CI-green compile, formatting, lint, regression
   tests, schema validation, permission and budget invariants.
2. **Zenoh RPC:** explicit wire protocol, tool discovery, request/reply,
   connection security, cancellation semantics and integration tests.
3. **L402:** Lightning-node integration, valid invoices, signed macaroons,
   payment-hash/preimage verification, replay/caveat/revocation tests.
4. **Dynamic tools:** signed manifests, sandboxed execution and resource
   quotas; avoid unsandboxed loading of untrusted code.
5. **Operability:** tracing/OTel, latency distribution benchmarks with
   hardware/network profiles, fuzzing, dependency audit and SLOs.
6. **Production release:** documented threat model, deployment hardening,
   versioned API and reproducible release artifacts.

**No sub-millisecond latency claims are made without reproducible
benchmarks**. Rust/Tokio/Zenoh alone do not establish that bound.

## Licensing

The package declares `MIT` in Cargo.toml. A standalone `LICENSE`
file and explicit redistribution policy should be confirmed by the
repository owner before publishing artifacts.
