# Toolchain recovery

The current execution host has no `cargo`, `rustc`, or `rustup`. The repository
now pins the expected CI toolchain in `rust-toolchain.toml` (`1.99.0`, minimal,
`rustfmt` and `clippy`).

## Reproducible check

Run on a host with network access or a preloaded Rust toolchain:

```sh
rustup toolchain install 1.99.0 --profile minimal
rustup component add rustfmt clippy --toolchain 1.99.0
rustup run 1.99.0 cargo test --locked --workspace --no-default-features --lib --tests
rustup run 1.99.0 cargo fmt --all -- --check
rustup run 1.99.0 cargo clippy --locked --all-targets --no-default-features -- -D warnings
```

The commands are intentionally not reported as passed until their output is
recorded with the toolchain version, commit, result and artifact digest.
