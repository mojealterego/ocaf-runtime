# Operator guide

Run cargo test --manifest-path omat/ocaf-artifact-gate/Cargo.toml, then cargo run --manifest-path omat/ocaf-artifact-gate/Cargo.toml -- check-tree . from the repository root. Each agent, system, swarm, OMAt or OMAs must have its own folder and contract. Add Rust implementation, tests and required documentation before reporting readiness. Zenoh is required when the architecture uses it, with actual transport source and runtime interoperability tests. Passing this structural check is not production approval.
