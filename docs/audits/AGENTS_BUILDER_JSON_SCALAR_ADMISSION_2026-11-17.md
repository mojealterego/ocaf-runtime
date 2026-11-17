# Agents Builder — JSON scalar admission (0.51/0.52)

Base upstream main: da4d1e88b4ce8eb18e18a4c1e46b36965f5206ae. Requirements: REQ-024, REQ-042.

The shared parser now rejects non-finite numbers and isolated UTF-16
surrogates in values and keys, preserving duplicate-key and existing
size/depth checks. The PWA cache is rotated to v11.

## Evidence

Linux / Node v22.16.0, isolated exact-blob fixture, no GitHub Actions.
Prior RED: 14 pass, 2 fail. Prior GREEN: 16 pass, 0 fail.
Fresh re-verification: 16 pass, 0 fail; separate deterministic scalar corpus:
8,906 acceptances and 4,132 rejections (13,038 cases total).
Corpus persisted as strict-json-scalar-audit.test.mjs.

Command: node --test apps/agents-builder-web/tests/strict-json*.test.mjs
Final focused result: 19 pass, 0 fail, exit 0.
Node syntax checks for strict-json.mjs and sw.js: PASS.

Exact tested Git blob SHAs:
- strict-json.mjs: 36a00ba9872cdc4d527a1a59c433af83959c92f6
- strict-json.test.mjs: c59f933d5baf57b6463350fdc0c779ab2f1b3856
- strict-json-scalar-audit.test.mjs: 5c6f8b37fccd5148a2ba62bcc959429497db1290
- sw.js: 9b68f1092e98229196620c192b98073a614d0cea

## Limits

Scoped source review and local tests: PASS. The additional corpus is a
separate verification procedure, not an independent human or agent audit.
Full repository checkout, full suite, Rust parity, native builds,
actual device/GUI E2E, live Cloud and independent acceptance remain
unverified. GATE-08 and GATE-10 remain open; no product or release approval.
