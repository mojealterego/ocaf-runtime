# AB-OM-03 / AB-OM-14 — public catalog boundary hardening, 2026-11-27

Status: TESTED_LOCAL_PARTIAL. Neither requirement is accepted in full. No production approval.

Base commit: `066fdda9aaab7feacc38d6149a6039124b60960d`. Node.js v22.16.0, isolated Linux temporary directory. Original Git blobs verified with git hash-object: agent-catalog.mjs f435e4cd65e3c6d6deb9ffb36bc12d2968d7cffa; strict-json.mjs 36a00ba9872cdc4d527a1a59c433af83959c92f6.

Command from apps/agents-builder-web: `node --test tests/catalog-boundary.test.mjs`.

RED: 1 PASS, 2 FAIL, exit 1. Missing expected exception for array-valued source_agent_registry_git_blob; missing expected rejection for application/jsonp.
GREEN: 3 PASS, 0 FAIL, exit 0. Tests cover both Git digest fields, invalid types and length, misleading media types and valid JSON with charset/case variation.

Tested post-fix blobs: agent-catalog.mjs 8f457d36fd7dfe1fc24eead10ae1816a04dd19e3; catalog-boundary.test.mjs adc58ff4505f24fce786a3171505c29182a4f3a1.

Local TAP log SHA-256: RED 9195324fada3e60b8f4f8ba002ba991803f71eff1a4f32af8c0adb5627cc2b0b; GREEN b9b5ede7839255a133276e515260f1e442abb91ccf20dee4a7e2fd82b034cb85. Logs not embedded; summary is not a substitute for rerunning tests.

Scope: strict scalar Git digest admission (not authenticity verification) and media-type boundary; comma-containing Content-Type is deliberately rejected as ambiguous. Fetch tests use a deterministic injected transport and native Node Response; no live network or browser E2E claim. Full project suite and Rust build not executed in this session. No change to canonical acceptance statuses; external audit not required.
