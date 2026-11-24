# OCAF — Pinned HTTPS source acquisition for Meta Agent Architect

Date: 2026-11-24. Branch: main. Baseline SHA: f7194ffcee168dac8adfbbe86fca24f5f62e06d8.

## Implemented code

- src/architect_https_source_fetch.rs: production Rust HTTPS/1.1 GET transport for operator-controlled pinned host, public IPv4 address and path, with TLS 1.3 minimum, certificate and hostname validation, direct connect to pinned IP (no DNS rebinding), no redirects, no caller-chosen HTTP method or arbitrary headers.
- Reuses the original fail-closed public IPv4 classifier from stage044_secret_broker rather than introducing a separate special-IP policy.
- Bounded response: 1..1,048,576 bytes body, 8 KiB header cap, 128 KiB chunk-framing overhead cap, maximum 4096 chunks, 1..15 second transport budget. No compressed content, no non-text media types, no redirect or pipelined HTTP body.
- Strict HTTP parsing: Content-Length with exact byte agreement or Transfer-Encoding chunked only, never both; duplicate length/encoding fields, malformed chunks, response splitting, trailers, redirects, binary content and unexpected bytes are rejected.
- SourceRecord.content_sha256 is the SHA-256 of actually retrieved and decoded response BODY, not an address-derived hash or a fabricated research statement.
- src/bin/ocaf-architect-fetch.rs: real one-shot operator CLI. It fetches a pinned route and writes the validated body only into a new local file (no overwrite), with mode 0600 on Unix; reports canonical source URL, retrieval timestamp and SHA-256 in JSON. No credential, secret, agent command or arbitrary LLM-controlled URL is accepted.
- The CLI is NOT registered as an OMAt tool while OSTc is disabled. Its operator-owned execution boundary must not be accessible as an agent capability without a separately reviewed OMAt authorization integration.

## Deterministic verification

Pinned Rust 1.99.0, Cargo.lock via --locked, real test subprocess. Cargo artifacts were cleaned with cargo clean before the full default-feature test run to recover disk capacity; no source files were removed.

| Verification | Observed result |
| --- | --- |
| RED compile test | E0432: missing HTTPS source module |
| Parser + route unit tests | 7 passed, 0 failed |
| Real CLI invalid-configuration tests | 2 passed, 0 failed |
| Chunked RED | 1 failing valid-chunked response |
| Chunked GREEN | Valid chunk decoding and hash PASS; malformed chunk/CL+TE rejection PASS |
| Full cargo test --locked --no-default-features | 779 passed, 0 failed |
| Full cargo test --locked (default Zenoh) | 820 passed, 0 failed |
| cargo clippy for changed binary and tests | Exit 0; seven preexisting unrelated repository warnings, none in changed modules |
| rustfmt --check and git diff --check | Exit 0 |

## Actual external HTTPS smoke test

Operator route configured for example.com with pinned public IPv4 104.20.23.154, port 443, path /, body limit 16384 bytes and 8-second time limit. This was a real direct TLS connection, not a simulated source.

- TLS certificate chain verified by OpenSSL and hostname SNI.
- HTTP/1.1 response: 200 OK, Transfer-Encoding: chunked.
- Resulting text/html response body length: 577 bytes.
- SHA-256 computed by the fetcher: 25ddf2c883e0d1958ea971d279a7e4f0fd446724ee3db7db19dadabd4a62e484.
- Independent sha256sum of the saved file: exactly the same digest.
- SourceRecord retrieval UNIX timestamp: 1795485766.
- Output was stored in an isolated temporary directory, checked and removed. The fixture domain is not a claim of news or verified competitive research.

## Tested source SHA-256

- src/architect_https_source_fetch.rs: 2a3d9ed0cd3055ff5f9a580bff5316ad826ec4777738afff685785960d6397da
- src/bin/ocaf-architect-fetch.rs: 97840488832d0622e32e4d2e45810e92f5a1133878afef1600a557be722d1422
- src/stage044_secret_broker.rs: 3564694f8a68bcb39ea894370bc1675d2da304c506c2c3055f5edfc290ceb70d
- tests/architect_https_source_fetch.rs: bef0574f112e8df344f70d968f13a2e2f9913e3b8ce72e302d888d36f52edbcd
- tests/architect_fetch_cli.rs: 92fea0bb68dcce71b9ede7cecca89a5564f8879e0d3c63db545cb12bb50b1b9b

## Open contract and security boundaries

This operator-only fetcher is not a search engine, a multi-source crawler, a semantic research agent or an OMAt-authorized executor. Source URLs may only be set in trusted operator configuration. Agent-controlled input must not become egress policy. Public DNS resolution and rotating provider IP pin updates must be handled separately by an authorized control plane.

Stored SourceRecords alone are not cryptographically attested by the network peer; independently signed researcher and verifier receipts must bind them through the existing ResearchEvidenceGate. Real Deep Research, model/OMAs/OMAt execution, search discovery, recurring study completion, external audit anchoring, OSTc/PQC and production release authorization remain open.

Local implementation and direct external HTTPS smoke: PASS within scoped contract. Full AB-OM-06 and AB-OM-07 research products: OPEN_UNVERIFIED. Production deployment NOT_PERFORMED. GATE-09 and GATE-10 NOT_ACCEPTED. production_approved=false.
