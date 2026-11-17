# OCAF Agents Builder — HarmonyOS NEXT

**Status: HarmonyOS Stage application source implemented; domain typed/behavioral tests executed in Node 22, native SDK build and device GUI E2E still NOT VERIFIED.**

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


## Source implementation (2026-11-17)

The Stage application contains native EntryAbility.ets, ArkUI BuilderPage.ets, Preferences-backed ProjectStore.ets and typed BuilderDomain.ets. Local validation checks resource limits, project ownership namespace, typed ports, directed cycles, tool permission grants and cost bounds. The Code/NoCode editors share a canonical JSON DAG and support an explicitly restricted round-trip. No runtime effect is executed.

Command for domain regression (Node.js 22+):

    node --experimental-strip-types --test apps/agents-builder-harmonyos/tests/domain.test.mjs

The UI exposes Code/NoCode and Local/Cloud switches. Local permits creating, validating, saving and reopening projects; it awaits Preferences.flush before reporting success. Cloud explicitly refuses save/load because authenticated HTTPS ingress, tenant binding and signed export are not integrated. Local tenant identifiers are only namespaces, not authentication.

Native HAP compilation, simulator/device GUI E2E, signatures, secure credentials and live Cloud service remain UNVERIFIED/NOT IMPLEMENTED; static source and Node tests are not equivalent to target-platform build evidence.
