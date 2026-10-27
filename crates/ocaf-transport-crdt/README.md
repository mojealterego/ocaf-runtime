# OCAF transport/CRDT workspace boundary

This package currently owns the existing Zenoh transport configuration preflight. It validates bounded operator configuration, denies plaintext outside explicit numeric loopback development, and requires an mTLS-only configuration for remote links. The root `ocaf_runtime::transport_policy` module re-exports the same public API, so existing CLI callers and tests keep their import path.

The package name follows the six-crate workspace plan. CRDT adapters, convergence tests, shared-memory transport benchmarks and edge interoperability are **not implemented in this package yet**. The preflight also does not replace Zenoh's actual TLS certificate validation or an end-to-end connection test.

Validate this extracted boundary with `cargo test --locked --workspace --no-default-features --lib` and `cargo test --locked -p ocaf_runtime --no-default-features --test transport_policy --test stage042_transport_workspace_compat`.
