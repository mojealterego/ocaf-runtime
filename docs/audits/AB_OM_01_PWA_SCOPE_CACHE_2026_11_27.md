# Stage 0.51 / AB-OM-01 — scope-isolated PWA cache v14

Date: 2026-11-27. Base commit: 74aba9fda9f1efc903205bd0602605661a644e4b. TESTED_LOCAL_PARTIAL; no stage acceptance.

The previous GGUF client fix was not accompanied by a service worker cache version change. CacheStorage is origin-wide, so the previous cache could also be shared across different application registration scopes. The new implementation names caches by encoded registration scope and version v14, restricts cleanup to versioned caches of that scope, creates install Requests with cache reload, credentials omit and redirect error, and bypasses cache handling for Authorization-bearing requests.

Legacy unscoped caches are deliberately not deleted: ownership cannot be established for other registrations on the same origin. This may leave old storage until explicitly cleaned. Existing skipWaiting/clients.claim behavior is preserved; multi-tab consistency during upgrade not verified.

Environment: isolated Linux temporary directory, Node.js v22.16.0.
Original files reconstructed byte-for-byte, verified via git hash-object:
- sw.js be1c780b601cf50d7b0bf40710a893d96321a2ac
- tests/service-worker-cache.test.mjs ac4d579902ee37960843f55378ba1fc94b3447f9

Command from apps/agents-builder-web: node --test tests/service-worker-cache.test.mjs.
Baseline 3 PASS / 0 FAIL. RED with new contract tests 0 PASS / 4 FAIL, exit 1.
GREEN 4 PASS / 0 FAIL, exit 0. node --check sw.js exit 0.
RED TAP SHA-256 fa084b38fc69496bd70d6486d8178525db9af6872f349a9a3f96e895d4af7733.
GREEN TAP SHA-256 8bfd550cfae3cb0a105cc01f08b0db773b83213330db679bee10b4981e796dd5.
Raw TAP logs not embedded. Modified blob SHA: sw.js 5a2cf573fc4597414ef4739be3034e30f9639d74; test 474a42fe5cd2e5072f749c6befb24ce897dbe765.

Tests use Node VM service worker and CacheStorage doubles and real Node Request objects. Not browser E2E. Full checkout unavailable (DNS failure in execution environment); cargo not installed. Full suite, actual model inference and real PWA upgrade not executed. No manifest/status changes or production approval.
