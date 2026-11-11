# Task CAS / fencing — formal verification procedure

The model in `formal/TaskCasFencing.tla` is an abstract state machine for
`src/task_cas.rs`. It does not model PostgreSQL transactions, SQLite
durability, the TRS-SEAL HMAC, network partition, or an external leader
authority. It cannot prove those mechanisms safe.

## Run

Install a trusted distribution of the TLA+ tools (`tla2tools.jar`) and run:

```sh
java -cp tla2tools.jar tlc2.TLC -config formal/TaskCasFencing.cfg formal/TaskCasFencing.tla
```

The finite model uses two tasks, epoch limit 2, revision limit 6.
Invariants: TypeOK, ReceiptConsistency, TerminalIrreversible,
NoDuplicateRevision. A passing bounded run is **not** an unbounded proof.

## Rust integration gates

```sh
cargo test --lib task_cas -- --nocapture
cargo test --lib trs_seal -- --nocapture
cargo test --all-targets --no-default-features
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Run these in the repository with the pinned toolchain and dependencies.
Record exact toolchain, commit SHA, exit code and complete logs. The model
and commands were committed without an observed TLC or Cargo run.

## Security acceptance still open

- Integrate trusted leader election and durable epoch authority; public
  `advance_fencing_epoch` is a storage primitive, not an authorization API.
- Bind replay admission to an authorized tenant/actor context.
- Enforce authenticated sequence allocation and persistent nonce lifecycle.
- Validate SQLite crash/restart and concurrent-writer behavior on the
  production filesystem; consider PostgreSQL HA for distributed nodes.
- Add tests that show old-leader effects cannot execute after fencing,
  including crash between admission and side-effect dispatch.
- Use an outbox/effect ledger to make external side effects idempotent.
- Do not claim TRS mathematical constructs provide proven cryptography.
