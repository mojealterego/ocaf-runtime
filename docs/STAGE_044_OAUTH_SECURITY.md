# OCAF Runtime — 0.44 OAuth 2.1, per-user authorization and Secret Broker

**Source implementation in main — NOT production accepted.**

Target: `mojealterego/ocaf-runtime`, branch `main`, Rust package `ocaf_runtime`, existing OMAs/OMAt architecture. This document does **not** close 0.43, enable OSTc, modify historical stage IDs, or assert that a provider-specific MCP adapter exists. This implementation is now being introduced into main as a code-level increment; deployment and production GO remain unauthorized pending live acceptance.

## 1. Scope, provenance, standards

Stage 0.44 in the canonical ROADMAP requires authorization code + PKCE, tenant-scoped tokens, per-user endpoint isolation, a broker resistant to SSRF and confused-deputy escalation, managed OAuth/step-up and hybrid PQC TLS compatibility/downgrade evidence. The stage is historically named OAuth and must retain its ID. The PDF11–PDF15 references in the ROADMAP are **requirements references**, not evidence of existing implementation. Source texts identified from the user-provided corpus support the requirements but do not prove deployment. The underlying original PDF11–PDF15 documents have not all been individually authenticated against their references.

Primary normative references:

- OAuth 2.1 IETF Internet-Draft (September 2026; **not yet an RFC**): https://datatracker.ietf.org/doc/draft-ietf-oauth-v2-1/
- OAuth Security BCP, RFC 9700: https://www.rfc-editor.org/rfc/rfc9700
- PKCE, RFC 7636: https://www.rfc-editor.org/rfc/rfc7636
- Resource Indicators, RFC 8707: https://www.rfc-editor.org/rfc/rfc8707
- Protected Resource Metadata, RFC 9728: https://www.rfc-editor.org/rfc/rfc9728
- Step-up Authentication Challenge, RFC 9470: https://www.rfc-editor.org/rfc/rfc9470
- Mutual-TLS OAuth, RFC 8705: https://www.rfc-editor.org/rfc/rfc8705
- Authorization Server Metadata, RFC 8414: https://www.rfc-editor.org/rfc/rfc8414
- OAuth 2.0 for Browser-Based Applications, RFC 10017: https://www.rfc-editor.org/rfc/rfc10017

**Implementation contract:** Only the `authorization_code` grant is admitted. PKCE `S256` is mandatory. `plain`, implicit grant, password grant, wildcard redirect URI, client-selected audience, token in URL query, cross-tenant tokens and silent step-up are denied. The token endpoint follows the September 2026 OAuth 2.1 draft by not requiring `redirect_uri` in code exchange; the exact URI is bound at authorization time.

## 2. Trust boundaries and deployment topology

```text
Browser / OAuth client (untrusted)
   | TLS 1.3, CSRF state, pre-registered exact redirect
   v
[External authenticated identity + consent gateway] <-- real IdP / MFA / operator session
   | verified subject {tenant,user,auth_time,assurance}, explicit consent
   v
[OAuthService: src/stage044_oauth.rs]
   | SQLite IMMEDIATE transactions; 90s one-use code; PKCE S256
   | opaque access (300s), rotated refresh (24h), family replay revocation
   v
[OAuthOmatGate: src/stage044_omat_authz.rs]
   | operator-owned mTLS peer -> tenant/agent/user mapping
   | exact resource + scope + assurance check
   v
[OMAs -> OMAt policy and executor boundary] --no automatic OSTc access-->
   |
   +--> [SecretBroker: src/stage044_secret_broker.rs]
           | AES-256-GCM per {tenant,user,route}; fixed public IP + TLS host
           | TLS 1.3, hostname verification, no redirects, strict I/O limits
           v
         Explicitly approved upstream SaaS endpoint
```

**Critical distinction:** The three Rust modules are an authorization/broker **core**. They are not a complete public HTTP authorization server, login UI, consent UI, OIDC federation, client registration service, KMS, external provider OAuth refresh service or deployment on real hosts. An authenticated gateway must supply `AuthenticatedSubject` only after verifying its own session, CSRF token, user consent and step-up. Do not expose `authorize` or `provision_secret` as unauthenticated HTTP or agent tools.

OMAs/OMAt is the native protocol. Per-user MCP endpoints mentioned in source PDFs must be treated as **external compatibility adapters** requiring explicit design approval and additional wire-level tests. This package uses logical per-user authorization routes; it does not claim physical per-user listener isolation.

## 3. Rust modules and contracts

| Module | Enforced contract | Persisted state |
| --- | --- | --- |
| `stage044_oauth` | Exact client/redirect/resource; consent; PKCE S256; step-up; one-time code; access scope/tenant/user/audience; refresh rotation/replay; user revocation | `oauth_codes`, `oauth_tokens`, `oauth_families` |
| `stage044_omat_authz` | Verified peer tenant/agent -> operator-bound user; OAuth token check before existing OMAt policy | Operator-provisioned binding in process |
| `stage044_secret_broker` | AES-256-GCM with tenant/user/route AAD; pinned public IPv4, TLS 1.3 hostname verification, bounded POST and response; no redirects | `oauth_broker_secrets` |
| `tests/stage044_integration.rs` | Real SQLite positive/negative cross-tenant/user and OMAt fail-closed behavior | Temporary SQLite |
| `scripts/stage044_pqc_tls_probe.sh` | Real hybrid TLS probe and classical-only downgrade rejection | Run log digests; external host required |
| `.github/workflows/stage044-oauth.yml` | Compile, unit/integration, fmt, clippy on Linux | GitHub Actions run artifacts/logs (after commit) |

SQLite file and WAL directory must be pre-provisioned, private (0700 directory, 0600 file), with final-component symlinks rejected. `PRAGMA synchronous=FULL` and `IMMEDIATE` transactions are used. A single SQLite file is not a multi-host HA authority; HA replication/fencing remains dependent on stages 0.40/0.41. Tokens are opaque, random, 256-bit, and only SHA-256 digests are stored. PKCE challenge is a canonical 43-character base64url-encoded SHA-256 digest. `state` is echoed but must be generated, stored and verified by the external OAuth client; a minimum length check alone does not prevent CSRF.

### Step-up rule

Each client has an operator-controlled `step_up_scopes` subset. Requests for those scopes require `assurance_level >= 2` and authentication no older than 300 seconds. Such grants do **not** receive refresh tokens. Refresh of ordinary grants reduces the assurance level to 1. This prevents a long-lived refresh token from indefinitely extending a privileged authentication event. A production identity gateway must map real MFA/ACR/AMR evidence into this assurance level, with challenge behavior compatible with RFC 9470.

The loopback protected-resource gateway also requires an explicit `OCAF_OIDC_MIN_ASSURANCE` value. Set `2` for a privileged/write-scope gateway; a signed token with stale MFA or a nonmatching ACR then receives 403 even if its scope and audience are valid. `1` is suitable only for an instance restricted to read-only scopes. This is a code-level boundary; the deployed reverse proxy, actual IdP MFA flow and OMAt policy still need matching live acceptance.

### Broker rules

`BrokerRoute` is registered by the operator and fixes the upstream hostname, public IPv4, HTTPS port 443, HTTP path, tool identity, resource, scopes and minimum assurance. The LLM cannot choose or modify these fields. There is no DNS lookup during execution, preventing DNS-rebinding through untrusted requests. TLS certificate validation still uses the configured hostname. Non-2xx status, redirects, chunked or ambiguous responses, unbounded payloads and reflected plaintext secret are rejected. Operator pinning must be updated when the provider's actual public address changes; a stale pin fails closed.

The broker's encryption key is loaded from a private operator-owned file via `open_from_key_file`; it is not generated or committed with this package. Local AES-GCM encryption is **not** evidence of HSM/KMS isolation. Memory wiping via `Vec::fill(0)` is best-effort and is not a guarantee against compiler optimizations or memory snapshots. The module includes operator-triggered atomic SQLite re-encryption (`rotate_master_key`); production still needs KMS/envelope encryption, multi-process rotation coordination and independent auditing.

## 4. Threat model and adversarial acceptance

| Threat | Required denial/proof | State |
| --- | --- | --- |
| Authorization-code interception/injection | S256 verifier mismatch denied, code single-use, 90s expiry | Rust logic + unit tests authored; **not executed** |
| PKCE downgrade | `plain` and missing/invalid challenge denied | Rust logic + unit tests authored; **not executed** |
| Redirect substitution | Exact pre-registered HTTPS URI at authorization | Rust logic + unit tests authored; **not executed** |
| CSRF / login session fixation | Real gateway session + state binding + consent UI | **OPEN: external gateway** |
| Confused deputy | Peer->user binding; token tenant/user/audience/scope checks | Rust logic + integration tests authored; **not executed** |
| Refresh replay/race | `BEGIN IMMEDIATE`; one winner; family revocation | Rust logic + concurrency tests authored; **not executed** |
| Privilege persistence after MFA | No refresh for step-up; refresh downgrades assurance | Rust logic + unit tests authored; **not executed** |
| SSRF / DNS rebinding | No caller URL, public pinned IPv4, TLS hostname, no redirects | Rust logic + negative tests authored; **not executed** |
| Vault ciphertext swap | AES-GCM tag and tenant/user/route AAD mismatch denied | Rust logic + unit tests authored; **not executed** |
| Tenant DB access by another OS principal | File mode checks; dedicated service UID, MAC policy | Partly implemented; **host evidence OPEN** |
| Secret exposure to LLM | Broker-only use; never return raw credentials | Architectural boundary; **full E2E OPEN** |
| TLS PQC downgrade | Real X25519MLKEM768 handshake; X25519-only must fail | Probe script authored; **live evidence OPEN** |
| TLS cert rotation, revocation, failure | Real TLS/mTLS negative and recovery tests | **OPEN** |
| Multi-host split-brain / rollback | Shared authoritative store, fencing and recovery | **OPEN: stages 0.40/0.41** |
| Tool execution bypass | OMAt retains its own policy and OSTc deny | Integration contract authored; **live executor OPEN** |

## 5. Mandatory E2E release matrix (real services, not mocks)

Each run must record commit SHA, UTC timestamp, run ID, test host/OS/kernel, Rust/OpenSSL versions, IdP issuer, certificate fingerprints (not private keys), tenant and user pseudonyms, negative-test outcome, and SHA-256 of sanitized logs.

1. Real browser + IdP: login, MFA, consent, `state` and PKCE S256; verify exact redirect; reject CSRF, missing PKCE, `plain`, injected code, duplicate code, expired code and wrong redirect.
2. Real token endpoint: `grant_type=authorization_code`, resource indicator, token scope; introspection/validation at OMAs and OMAt; reject wrong tenant, user, audience, scope and agent peer identity.
3. Refresh: two concurrent exchanges for one code and one refresh token; exactly one success; replay revokes the family; restart and retry do not reissue.
4. Per-user endpoint: two different users within the same tenant plus a second tenant; confirm no session, token, cookie, callback or tool route crossover.
5. Secret broker: operator-provisioned SaaS sandbox credentials, live HTTPS request, no secret in agent-visible output/logs, negative loopback/RFC1918/metadata IP, redirect, DNS change, invalid CA and oversized response.
6. Step-up: scope elevation requires fresh MFA; old session/old refresh token cannot obtain write scope; downgraded assurance rejected.
7. PQC TLS: hybrid `X25519MLKEM768` positive with verified peer certificate, classical-only client negative, TLS 1.2 negative, certificate mismatch negative, rotation/reconnect positive. This is **compatibility/downgrade testing**; ML-DSA/ZKP belongs to stage 0.45.
8. Faults: OMAs/OMAt crash/restart, database locked, corrupted ciphertext, secret unavailable, clock skew, cert expiry, network partition, delayed response and idempotent recovery.
9. Operations: metrics by decision category without token contents; immutable audit events, secret rotation/revocation, key rotation, resource caps, operator rollback, independent security review and sign-off.

**No `GO` decision is permitted while any mandatory E2E row is OPEN.** A passing unit-test workflow cannot change this status.

## 6. Build and deployment procedure (when Rust/hosts are available)

```bash
# On a real Linux build host with Rust 1.99.0 and repository dependencies:
cargo check --locked --no-default-features --lib --tests
cargo test --locked --no-default-features stage044_oauth::tests
cargo test --locked --no-default-features stage044_secret_broker::tests
cargo test --locked --no-default-features --test stage044_integration
cargo clippy --locked --no-default-features --lib --test stage044_integration -- -D warnings
rustfmt --check --edition 2021 src/stage044_oauth.rs src/stage044_secret_broker.rs src/stage044_omat_authz.rs tests/stage044_integration.rs

# Separately: a REAL test endpoint with an enforced hybrid TLS policy.
# OCAF_PQC_HOST, OCAF_PQC_PORT and OCAF_PQC_CA_FILE are mandatory.
bash scripts/stage044_pqc_tls_probe.sh
```

Provision the SQLite file under a dedicated UID, private parent directory and volume with backups, and an independent private AES key file (64 hex characters representing 32 random bytes). Never commit real client secrets, bearer tokens, TLS private keys or KMS material. The code intentionally does not create a database or a master key from unauthenticated requests. Use an external gateway with secure session cookies (`Secure`, `HttpOnly`, `SameSite` as appropriate), explicit CSRF protection, session rotation on login and consent, IdP verification and rate limiting. Run behind an HTTPS ingress with pinned issuer and trusted proxy policy. Deny all authorization routes until the gateway is integrated and its identity contract has real E2E evidence.

## 7. Acceptance status and blockers

- **Auth core:** authored, not locally compiled or executed in the current environment.
- **Encrypted broker and SSRF route policy:** authored, not locally compiled or executed.
- **OMAs/OMAt binding:** authored, not locally compiled or executed.
- **Public HTTP OAuth 2.1 endpoints, real IdP and user consent:** not implemented in this package.
- **Upstream managed provider OAuth refresh and external KMS/HSM integration:** not implemented. Local AES-GCM master-key re-encryption is implemented but untested.
- **Per-user external MCP adapter:** not implemented; native OMAs/OMAt preserved.
- **Hybrid PQC TLS:** probe supplied; no real server evidence.
- **Production E2E, external audit, operator GO:** not performed.

This is a security-oriented implementation increment. The canonical Roadmap must continue to show 0.44 as **NOT ACCEPTED** until the full deployment and acceptance matrix is complete. When changing stage status later, reconcile `README.md`, `ROADMAP.md`, `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, `docs/roadmap/canonical_v11_inventory.json` and `docs/roadmap/STAGE_CHANGELOG.md` atomically; do not renumber historical stages.
