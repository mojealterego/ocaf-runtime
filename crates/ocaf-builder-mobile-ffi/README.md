# OCAF Agents Builder — Rust UniFFI offline core

A non-duplicating native-mobile binding to the **existing**
`src/builder_core.rs` graph validator and inert plan compiler.
It is restricted to tenant namespace `local`, never authenticates Cloud,
and cannot execute tools or publish an agent.

```sh
cd crates/ocaf-builder-mobile-ffi
cargo test
cargo build --release
cargo run --features bindgen --bin uniffi-bindgen -- generate --library target/release/libocaf_builder_mobile_ffi.so --language kotlin --out-dir out/kotlin
cargo run --features bindgen --bin uniffi-bindgen -- generate --library target/release/libocaf_builder_mobile_ffi.so --language swift --out-dir out/swift
```

On macOS the host library is a .dylib; on Windows it is a .dll.
Mobile targets require actual cross-compilation, Kotlin/Swift binding
generation, linking, and device tests. This crate is not integrated into the
native projects until those per-platform build stages are added and verified.
