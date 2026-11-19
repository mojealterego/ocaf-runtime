# HarmonyOS Agents Builder — local JSON exchange

Scope: explicit import/export and identity validation in Code/NoCode Local mode.
Base main: c62de2b4c1ca5f1474065b22393c979bf08ead51. Source copies verified against Git blobs.
- BuilderPage.ets: ec0ef34c01d5ae3a7bf8f5cce9557ca715cbec2f
- BuilderDomain.ets: 4fe3aa6e983f8d7b8c5e6405358a613cb1a542b2
- ProjectStore.ets: d39afd8663b94955ee5b8af1a17c7f0e1391cb19
- page-store.test.mjs: 8b5d68692b92ea70a39443e1e40d7e70ba71844b

## Change
Import validates canonical DAG, tenant namespace and selected project ID before replacing in-memory source. Export validates the same identity. No automatic storage write, tool execution, network access or publication. Transfer buffer clears on identity/workspace changes or export failure. Static plan validation shares the identity and Local-mode guard. UI offers explicit text-field exchange in both modes; not file-picker or signed distribution.

## Executed deterministic tests
Linux isolated partial source checkout, Node 22.16.0.
Command: `node --experimental-strip-types --test apps/agents-builder-harmonyos/tests/page-store.test.mjs`.
RED 5/10 pass, 5 fail, exit 1.
GREEN 10/10 pass, 0 fail, 0 skip, exit 0.
Five new tests cover roundtrip including tool node, malformed/duplicate/oversize/foreign input, stale export, pending-save and unauthenticated Cloud guards, source-level UI wiring. Five existing handler tests pass. Actual domain and extracted page handlers, controlled Preferences adapter; not native GUI E2E. git diff --check PASS.

Verified final Git blobs:
- BuilderPage.ets: 888d63e39c88420b9cb45128a48ce93a1520a65d
- page-store.test.mjs: 8d2c6ec512949524c8b0bcdd2e64b1b6a12492c1

SHA-256:
- f31e721e43d95faff07ab234ac91478c1e2f91974c4e7b48a6575e77f95dae40  BuilderPage.ets
- ee1aad486e345502ee44fc99137636f35da1ea82bcf239172bb8f646b79749e4  page-store.test.mjs

SCOPED_PASS (local source-handler evidence). Full repo suite, DevEco/ArkTS build, native GUI, real Preferences durability, signed HAP, authenticated Cloud and complete platform release gate remain open. No physical device or paid CI triggered manually.
