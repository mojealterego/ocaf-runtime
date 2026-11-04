# OCAF Agents Builder — Linux + HarmonyOS

Status: Linux CI **implemented, execution unverified**; HarmonyOS **design only**. No signed Linux distribution, HarmonyOS HAP or device test has been produced by this document.

## Product topology

One shared versioned typed graph / Code↔NoCode project schema, common model routing, credential policy, cloud workspace protocol and export verifier. Each native shell exposes **Code / NoCode** and **Local / Cloud** as runtime switches, not separate applications.

| Platform | Native host | Local mode | Cloud mode | Acceptance |
| --- | --- | --- | --- | --- |
| Linux x86_64 | existing Tauri 2 Rust desktop app + WebKitGTK | constrained local executor, offline project store | authenticated OCAF Cloud | ELF build + Xvfb smoke + GUI E2E + AppImage/deb/rpm validation |
| HarmonyOS NEXT | ArkTS/ArkUI Stage application (HAP) | sandboxed supported subset; no assumption of Android APK or POSIX/WASI availability | HTTPS OCAF Cloud through scoped client | DevEco SDK build + simulator/device launch + ArkUI Code/NoCode E2E + signed HAP |

## Linux implementation path

1. Reuse `apps/agents-builder-desktop` and `apps/agents-builder-web`; no duplicated editor.
2. Build using ubuntu-24.04 with WebKit2GTK 4.1 dependencies and Tauri 2. Validate ELF header and run under Xvfb. CI: `.github/workflows/agents-builder-native-linux.yml`.
3. Add real GUI automation: create/edit/reopen project in Code and NoCode, verify bidirectional conversion, local/cloud isolation, offline behavior and denied permissions.
4. Package AppImage, deb and rpm **only after** native build and GUI checks. Sign release and publish SHA-256, SBOM and provenance; distinguish unsigned CI binaries from releases.
5. Negative gates: prevent unrestricted shell/file access, injected model URL, tenant crossing, and false cloud execution receipts.

## HarmonyOS NEXT design (not implemented)

- `apps/agents-builder-harmonyos/AppScope/app.json5`: app bundle metadata, explicit supported API level and permissions.
- `apps/agents-builder-harmonyos/entry/src/main/ets/entryability/EntryAbility.ets`: Stage UIAbility lifecycle.
- `apps/agents-builder-harmonyos/entry/src/main/ets/pages/BuilderPage.ets`: ArkUI shell with Code/NoCode and Local/Cloud controls.
- `apps/agents-builder-harmonyos/entry/src/main/ets/services/BuilderGateway.ets`: typed versioned request/response transport; local/remote capability negotiation.
- `apps/agents-builder-harmonyos/entry/src/main/ets/services/SecureCredentials.ets`: platform-backed encrypted credential storage, no secrets in WebView/localStorage/logs.
- `apps/agents-builder-harmonyos/entry/src/main/ets/services/ProjectStore.ets`: atomic workspace persistence and migrations.
- `apps/agents-builder-harmonyos/entry/src/main/ets/tests/`: ArkTS unit tests and UI tests for both switches and denial paths.

### HarmonyOS architecture decisions

- Prefer native ArkUI for app shell and capability/security controls. Embed the shared editor only if supported Web component, ES module loading, local resource access and CSP are verified against a pinned HarmonyOS SDK; otherwise implement a typed ArkUI graph editor using the same IR protocol.
- Cloud mode: TLS-authenticated, tenant-scoped API with explicit consent, retry/backoff, offline queue bounds and cancellation.
- Local mode: capability-negotiated graph validation and supported execution only. **No unsupported promise** of GGUF inference, native Rust/WASI binaries or Android runtime compatibility.
- Model support: remote API keys via encrypted platform storage; GGUF only after measuring RAM, ABI, model backend availability and device performance.
- Build: pinned DevEco/ohpm/hvigor toolchain and HarmonyOS SDK in an authorized runner; reproducible HAP and package metadata; no fabricated HAP from ZIP.
- Security: least-privilege permissions, domain allowlist, integrity verification, key rotation, redaction, explicit user approval for agent publish, deny-by-default tool capabilities.

## Release gates

1. **Source contract**: shared IR schema and versioned API, negative tests.
2. **Build**: real Linux ELF / HarmonyOS HAP generated with platform toolchains.
3. **Runtime**: install/launch on Linux graphical session and HarmonyOS simulator/device.
4. **GUI E2E**: both editing modes, Local/Cloud switching, persistence, conversion, denied unauthorized actions.
5. **Distribution**: signature, checksums, SBOM, rollback and reproducible evidence per pinned commit.

Do not mark either platform IMPLEMENTED_VERIFIED until all relevant acceptance evidence exists. Linux workflow presence alone proves neither successful build nor GUI behavior.
