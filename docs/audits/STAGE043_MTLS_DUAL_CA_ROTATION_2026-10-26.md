# OCAF 0.43 — TLS/mTLS dual-CA rotation and ingress continuity

**Status:** IMPLEMENTED / E2E_PENDING (do not mark PRODUCTION_APPROVED without a successful run and operator evidence).

## Scope and safety boundaries

- The rotation is **blue/green**, using two or more independently started Zenoh/OMAt processes and distinct TLS listeners. It is **not** a claim of in-place hot reload of a rustls certificate.
- The old and new server identities are signed by **different private test CAs**. The overlapping listener trusts a concatenated `old-ca.pem + new-ca.pem` bundle and is verified with both old and new client certificates.
- A final listener trusts **only** the new CA. The old client certificate must fail to connect to it; a client with the new-only trust store must reject the old server identity. Old listeners are retired only after all submitted requests have received responses.
- The configured CA is an **exclusive** TLS trust boundary in vendored Zenoh: public WebPKI roots must not be silently added when a private CA is supplied.
- OMAt's signed ingress uses a shared SQLite WAL replay ledger. Two listeners can record unique signed nonces against the same file; a rejected tool-policy decision still proves authentication and durable ingress, **not execution**.
- SIGTERM triggers a graceful queryable shutdown that completes the currently processed request before returning. It does **not** drain arbitrary queued traffic by itself; routing and in-flight draining are the operator/orchestrator's responsibility.

## Reproduction

```bash
cargo build --locked --features zenoh --bin ocaf-omat-zenoh --bin ocaf-omas-zenoh-client
bash scripts/stage043_zenoh_ca_rotation_e2e.sh
cargo test --locked --features zenoh --test stage043_omat_zenoh
```

The E2E script generates short-lived CA and identity keys in a private temporary directory, verifies both PKI chains with OpenSSL, launches the old/overlap/new-only listeners on separate loopback ports, exercises old and new mTLS client identities, sends concurrent signed requests, checks explicit OMAt policy responses, verifies retired identities are rejected, and asserts that **all 20 expected authenticated ingress nonces** are present in the SQLite ledger. It removes temporary secrets on exit.

The local OpenSSL certificate-chain checks passed before the GitHub commit. This does **not** prove the Zenoh E2E passed. CI must run the complete script, and the resulting log and commit SHA must be inspected before claiming a pass.

## Limitations / required production acceptance

1. All accepted traffic in this drill reaches a deliberately **fail-closed OMAt tool policy**; no OSTc executor is enabled. The test proves authenticated ingress continuity, not lossless completion of arbitrary external side effects.
2. In-flight requests are awaited **before** old-listener shutdown. A sudden kernel crash or an uncoordinated SIGKILL can still interrupt an in-flight reply; end-to-end exactly-once execution requires a separate durable execution receipt/outbox.
3. Test certificates are ephemeral and must not be deployed. Production issuance, key storage, revocation, tenant-to-certificate binding, endpoint discovery and routing/drain controls require target-environment evidence.
4. Rotating a listener between ports requires a gateway or a client endpoint migration plan; the script itself does not deploy a production load balancer.
5. Do not treat an OpenSSL chain-verification result as a substitute for an actual mTLS handshake, nor a green CI run as independent multi-host production acceptance.

**CI:** Stage 0.43 OMAs/OMAt Security, step `Two-PKI Zenoh TLS/mTLS certificate overlap, withdrawal and in-flight ingress E2E`.
