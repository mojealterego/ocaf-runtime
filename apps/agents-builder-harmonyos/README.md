# OCAF Agents Builder — HarmonyOS NEXT

**Status: architecture preparation only. No HAP, simulator execution or device verification.**

## Platform boundary

HarmonyOS NEXT uses ArkTS/ArkUI Stage model and a native HAP. Android APKs are **not** the distribution target. Shared OCAF Agents Builder graph/DSL semantics must be transported over a versioned, capability-negotiated interface; the Linux/Windows/macOS Tauri host is not reused as an executable.

## Product modes

- Code / NoCode: one native application, two views of the same typed project schema. Round-trip conversion must reject unsupported constructs with an explicit lossiness report.
- Local / Cloud: an explicit switch. Cloud requires tenant-bound authentication and scoped HTTPS requests. Local requires a supported sandbox/executor and offline persistence; no assumed native GGUF/WASI support.

## Module boundaries (implementation pending SDK validation)

- `entryability/EntryAbility.ets`: lifecycle, app permissions and routing.
- `pages/BuilderPage.ets`: ArkUI Code/NoCode and Local/Cloud controls.
- `services/BuilderGateway.ets`: typed API and capability negotiation.
- `services/ProjectStore.ets`: atomic versioned project persistence.
- `services/SecureCredentials.ets`: platform keystore-backed secrets.
- `tests/`: contract, permission denial, persistence and GUI tests.

Before implementing ArkTS API calls, pin the actual DevEco Studio/HarmonyOS SDK release and validate its APIs against vendor documentation. Do not invent SDK imports, HAP build commands, simulator results, device capabilities or signing receipts.

## Acceptance

1. Reproducible signed or clearly marked unsigned HAP built with the real SDK.
2. Launch on actual HarmonyOS NEXT simulator/device, with archived logs.
3. Code and NoCode edit the same project; project round-trips and survives restart.
4. Local/Cloud switch isolates storage, credentials and tenant sessions.
5. Unauthorized filesystem/network/tool access fails closed.
6. Export has verifiable signature and provenance.

See `docs/roadmap/AGENTS_BUILDER_LINUX_HARMONYOS.md`.
