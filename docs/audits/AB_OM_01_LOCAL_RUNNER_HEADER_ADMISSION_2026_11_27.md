# AB-OM-01 / stage 0.51 — local runtime HTTP header admission

Date: 2026-11-27. Baseline commit: 6063cbc431e69b5fe17e2a5c5717dabf36043225. Status TESTED_LOCAL_PARTIAL. Full stage remains open.

The loopback HTTP handler accepted duplicate Host, Origin, Authorization, Content-Type and X-OCAF-Model-Name headers based on first-value lookup. A non-ASCII Authorization caused compare_digest to raise TypeError and disconnect. Reject ambiguous headers before dispatch; require ASCII before constant-time comparison. Content-Type comma lists are also rejected conservatively.

Evidence: Python 3.13.5, isolated Linux loopback, real TCP/HTTP server (ThreadingHTTPServer), actual parse_request_json, OCAFServer and LocalHandler source section. Recording backend only; no model execution. The source section from def parse_request_json through immediately before def main was extracted from GitHub and reproduced byte-for-byte (8339 bytes, SHA-1 2381cb6f02669c0d8f58dba5dba863fbe0aeef66); original full Git blob a16e6598aec914dec45ab4ea4a459cfd748c3dc5 verified. The local module harness provided imports, MAX_REQUEST and minimal unused static-file constants; full source file was not locally compiled or imported. Test file imports the actual full module when run in the repository.

Command: python -m unittest discover -s scripts/tests -p test_local_runner_header_admission.py -v.
RED: 1 PASS, 6 FAIL, exit 1. Five duplicate-header cases accepted; invalid token disconnected. SHA-256 7cabb612e4a6d900f98bfb06ef4e9264f455868e2e433eebd16ddb8a2df79a43.
GREEN: 7 PASS, 0 FAIL, exit 0. All six negative cases reject before dispatch; valid chat and import still dispatch. SHA-256 d87d880de63cf81c9cdb36ef26a70b12cbaa26779aa6db8f7eba3dd60da2c699.
python -m py_compile on local reconstructed HTTP module: exit 0.
Modified section SHA-1 03202c1c319beebb56f53e893588b07d754639dc; complete patched Git blob 972ddb84ca17fbfa544f5965589be0a86bdf652c; test blob 0dbb03801d220464e0d2e1d29b6766bf99cbf789. Raw logs retained in temporary test environment, not committed.

Existing stage051 workflow now discovers both HTTP admission test files. No manual CI dispatch. Full repository regression, full module local import, actual llama inference, Rust build and production verification remain unperformed. No external audit required; no stage or release status promotion.
