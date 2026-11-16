# OCAF Agents Builder v1.0 — native source implementation, 2026-11-16

## Repository / branch

All changes are applied to `mojealterego/ocaf-runtime` on the existing
`main` branch. No parallel OCAF repository and no forked builder engine.

## Implemented code (source-level)

| Platform | Code/NoCode | Native host | Shared core |
| --- | --- | --- | --- |
| Cloud | Existing web views | Browser PWA | Rust `src/builder_cloud.rs` / `builder_http.rs` |
| Windows | Same web views | Tauri v2 `apps/desktop/src-tauri` | Existing JSON DAG and web editor |
| macOS | Same web views | Tauri v2 `apps/desktop/src-tauri` | Existing JSON DAG and web editor |
| Android | Same web views | Compose + WebViewAssetLoader + SAF | `ocaf-builder-mobile-ffi` compiles the existing Rust graph validator |
| iOS | Same web views | SwiftUI + WKWebView + share sheet | `ocaf-builder-mobile-ffi` via generated Swift bindings |

The two modes are editable views of the same project. Source-only Android
and iOS build definitions generate bindings against the same Rust
`src/builder_core.rs`, not independent native graph validators.

## Verification commands, to execute on target toolchains

```sh
python3 -m unittest scripts.tests.test_agents_builder_native_structure -v
node --test apps/agents-builder-web/tests/*.test.mjs
cargo test --manifest-path crates/ocaf-builder-mobile-ffi/Cargo.toml
cd apps/desktop/src-tauri && cargo tauri build
cd ../../android && gradle :app:assembleDebug
cd ../.. && bash apps/ios/build.sh iphonesimulator
```

**Do not** interpret the presence of these commands as proof of execution.
Windows, macOS, Android and iOS require independent real builds, platform
installation and Code/NoCode GUI E2E. iOS requires macOS/Xcode, Android
requires SDK/NDK/cargo-ndk, desktop requires Tauri OS dependencies.

## Explicitly open specification items

REQ-030..034 formal OCAF-DSL/LSP, REQ-050 complete 13-kind
model, REQ-052..054 runtime budgets/idempotency/approval, REQ-060
actual authenticated TLS ingress, REQ-090..092 real signed executable
WASI packaging/sandbox, native Keychain/Keystore cloud identities and
production SBOM/signing/attestation remain OPEN. Existing cloud Rust
boundary is not a deployed TLS gateway. No effects execute in these
new local shells.

GATE-02 (real 10 builds), GATE-03 (10 device GUI E2E), GATE-04..08 and
GATE-10 are **NIEPOTWIERDZONE**. Never substitute static source checks or
synthetic fixtures for binary/toolchain/device evidence. The tracked
`release/CERTIFICATE_RECORD.json` remains the fail-closed authority.
