# Usage — OCAF Component Gate

Run `cargo test --manifest-path components/omat/ocaf-component-gate/Cargo.toml` and then `cargo run --manifest-path components/omat/ocaf-component-gate/Cargo.toml -- --repo .`.

For formal release acceptance use `cargo run --manifest-path components/omat/ocaf-component-gate/Cargo.toml -- --repo . --strict-targets`. Strict mode MUST fail until all 33 registered swarms have actual component folders, Rust Cargo packages, runnable source, tests and documentation.

Every new agent, system, swarm, OMAt tool or OMAs server must be registered under the matching category, with a `component.toml` containing `kind`, `transport` and `maturity`. Required component files include `Cargo.toml`, `src/lib.rs` or `src/main.rs`, `tests/*.rs`, `docs/architecture.md`, `docs/user.md` and `docs/certification.md`.

Do not mark a component verified or a stage complete merely because a Markdown document or Make scenario exists. Supply real CI run IDs, immutable commit SHA and production integration evidence.
