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

## NoCode graph editing and canonical serialization (2026-11-17)

The existing BuilderDomain now reconstructs object fields in fixed Rust
struct order independently of JSON input key order and rejects nesting
over 64. Tool insertion and updates adjust the declarative budget and
reconcile shared grants; rename rewrites all incident edges; removal of
a linear type-preserving Tool/Transform reconnects predecessor/successor.
Ambiguous removal fails without modifying the source object. Grants and
costs remain static declarations, not execution authorizations.

The ArkUI source wires the operations to the existing shared Code/NoCode
project. Local save locks concurrent UI edits and workspace switching;
load rejects a mismatched stored project ID. AppScope label/icon resources
are included. Five source-handler tests use a controlled Preferences adapter, checking
single-flight save, flush failure, project identity and Cloud denial.
These UI/storage/resource changes have not been validated
with the DevEco SDK or on hardware; single-process UI exclusion does not
establish cross-process CAS, durable rollback or full persistence safety.

Actual Node.js 22.16.0: RED 10 failures on the exact original domain;
GREEN 24/24 tests (10 new graph + 5 handler + 9 existing), including 32 deterministic
edit sequences. TypeScript --strict --noEmit --target es2022 --module
esnext on an exact .ts copy of BuilderDomain succeeded. Git blob SHA:
domain 4fe3aa6e983f8d7b8c5e6405358a613cb1a542b2;
new test 3d00a29257c0147b0993bc8c5a1eb0e9a9bc81e9;
existing test 588db0207ce401885be1973ca7ee4544a7cd138f.

    node --experimental-strip-types --test apps/agents-builder-harmonyos/tests/*.test.mjs

Native HAP/GUI, authenticated Cloud, device persistence and signing are
unverified or unimplemented. GATE-10 remains NOT_ACCEPTED.

Handler test Git blob: 8b5d68692b92ea70a39443e1e40d7e70ba71844b. Handler source blob ec0ef34c01d5ae3a7bf8f5cce9557ca715cbec2f. Real Preferences durability is not established by this adapter.
