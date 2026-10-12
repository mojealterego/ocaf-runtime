# Security architecture and threat boundaries

## Implemented controls

- Per-peer HMAC-SHA256 signatures over the **serialized payload bytes**.
  The unsigned sender claim is used only for key lookup. Execution follows
  successful verification using that peer's key.
- Per-peer exact tool allowlist. Unknown peers and denied tools fail closed.
- Remote calls are restricted to **enabled, free tools without elevated
  permissions**. Paid tools, permissioned tools and L402 authorization
  credentials are not transported through this RPC layer.
- Node IDs are strict alphanumeric, underscore and hyphen; routing paths
  cannot contain wildcard or path delimiters.
- Versioned envelopes, UUID request IDs, short validity windows, clock
  skew checks, in-memory replay detection, bounded replay cache,
  bounded HMAC frame/body sizes and concurrency limit.
- Replies are signed with the appropriate peer secret and the client
  validates sender, recipient, protocol version and correlation ID.
- Runtime enforces JSON Schema validation, execution timeouts,
  cancellation-aware atomic budget accounting and controlled errors.

## Not guaranteed

| Threat / property | Current state | Required production control |
| --- | --- | --- |
| Passive traffic observation | Plain Zenoh TCP leaks payload contents | TLS/mTLS, encrypted overlay, private network |
| Credential theft on disk/memory | Key is read from environment; no hardware protection | Secret manager, ephemeral credential issuance, rotation |
| Replays after process restart | In-memory cache is lost | Durable idempotency store and TTL ledger |
| Unauthorized execution by authorized peer | Exact allowlist limits tools; allowed code may still perform side effects | Sandboxed tools, fine-grained capability policy and quotas |
| Duplicate execution when response is lost | Client has no exactly-once guarantee | Transactional deduplication/outbox, operation tokens |
| Network denial of service | Bounded frames/concurrency; not full network DDoS defense | Edge controls, per-peer rate limits and backpressure |
| Agent identity | Pairwise shared secret authenticates a peer possessing that key | Workload PKI, mTLS identity binding, key rotation |
| Real L402 payment authorization | Not implemented | Verified invoice/hash/macaroon/caveats, ledger, replay protection |
| Tool isolation | Tools run in the same OS process | sandbox, seccomp, capability boundaries and quotas |
| End-to-end trace/observability | Basic in-process metrics | OpenTelemetry, audit log and incident response |
| Multi-router trust | Not independently verified | mTLS Zenoh config and authenticated router topology |

**HMAC signs; it does not encrypt**. A Zenoh config file being present
does not prove that TLS/mTLS is enabled; deployments must independently
verify the actual transport security. The demo configuration files in
`examples/insecure-local` are plaintext loopback-only.

## Keys

Generate a distinct, cryptographically random 32-byte key per two-peer
relationship, provision through an appropriate secret manager, and inject
only at launch. The example CLI uses `OCAF_SHARED_KEY_B64`. Never commit
real keys or print them in logs. Rotation and automatic revocation are
not part of this implementation.

## Future security acceptance gates

1. A complete mutual TLS deployment test with unauthorized peers blocked.
2. Durable replay ledger and idempotency with simulated crashes.
3. Per-peer budgets, rate limits and structured audited denials.
4. Sandbox integration for untrusted tools and bounded CPU/memory/network.
5. Dependency and supply-chain audits, fuzzing of signed envelope parser.
6. Cross-machine test under network disruption, latency and packet loss.
7. External security assessment and documented recovery/runbook.

## Vulnerability reporting

Do not disclose exploitable credentials or private infrastructure details in
public GitHub issues. Use the repository owner's private reporting channel.
