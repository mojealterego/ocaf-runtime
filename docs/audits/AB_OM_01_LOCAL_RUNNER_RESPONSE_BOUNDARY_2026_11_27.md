# AB-OM-01 / stage 0.51 — local GGUF runner response boundary

Date: 2026-11-27. Base: c686e02a79e5cfdab40e1ed124ab3824e994a3c1. Status TESTED_LOCAL_PARTIAL, not end-to-end model execution or release approval.

Original runner-client.mjs Git blob 5bb211f30e74407660776d16712631b5145e5764 and existing strict-json.mjs blob 36a00ba9872cdc4d527a1a59c433af83959c92f6 reproduced and verified with git hash-object in isolated Linux directory. Node v22.16.0. Test settings dependency models.mjs was a fail-fast stub, because this suite exercises chat and import only, not settings/start. Native Node Response, File and ReadableStream; injected loopback transport, no actual model or server.

Command: node --test tests/runner-response-boundary.test.mjs
RED 1 PASS / 5 FAIL, exit 1; log SHA-256 9477abed387aaef25537155c326b9f66ef3649b79c8a2e89b058bb7b518cdbe9.
GREEN 6 PASS / 0 FAIL, exit 0; log SHA-256 424b32d2a2f74b34a236cbf9c2568a28a36609c8e1338dcea316c5227a7dc927.
node --check runner-client.mjs exit 0.
Tested blobs: runner-client.mjs 1707f8532fd7ec500bf66c3217ed0a0970949301; tests/runner-response-boundary.test.mjs 091ce44ebf061445a7ef6a99d025bdb50857f672.

Change: bounded streaming read (512 KiB normal, 8 KiB import), strict UTF-8 and unique-key JSON, reject non-object receipts, cancel oversized streams, release reader locks. Preserve valid responses and structured HTTP errors. This does not verify uploaded model content against an independently trusted digest. Raw TAP logs not included in this commit.

Full repository regression, real GGUF execution, native browser E2E and Rust build not executed. PWA release cache invalidation remains a separate integration task. AB-OM-01 remains OPEN_UNVERIFIED. No external audit required.
