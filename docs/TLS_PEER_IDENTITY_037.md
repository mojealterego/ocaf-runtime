# Priority 0.37 — verified TLS 1.3 peer certificate → RPC identity

**2026-10-13. Opt-in alternative network ingress. Production NO-GO.**

## What changed

Zenoh's query/reply API does not expose a cryptographically verified
TLS peer certificate to its RPC handler. Passing a claimed SAN in the
RPC request would be forgeable. Therefore 0.37 provides a **separate**
TLS 1.3-only, mutually authenticated TCP RPC listener. It obtains the
client's leaf DER from the **same live rustls TLS session** that delivers
the HMAC-authenticated RPC frame, after a rustls/WebPKI mandatory client
certificate handshake.

The server maps the RPC sender to a peer policy with BOTH:
- approved leaf certificate SHA-256 fingerprint (configured in 0.36);
- an explicit expected DNS Subject Alternative Name (no CN fallback).

At configuration time, the required SAN must exist in the pinned leaf;
at request time, the verified TLS session's leaf DER must match that
pin, and the frame must also pass HMAC and certificate-private-key PoP.
A correct HMAC and PoP accompanying a *different TLS certificate* fail
closed before any replay journal write or effectful operation.

**Important:** This is **not** the Zenoh TLS peer identity. Production
deployment must enable exactly one ingress. If `OCAF_RPC_INGRESS=tls-bound`,
the coordinator does **not** declare a Zenoh RPC queryable, so clients
cannot bypass transport binding via a fallback query path.

## Configuration: coordinator

Build explicitly with `cargo build --locked --features tls-bound
--bin ocaf-coordinator --bin ocaf-rpc-node`. Existing builds without
this feature refuse TLS-bound mode.

The coordinator uses the current owner-only `OCAF_PEERS_FILE` with
mandatory entries for **every** peer:

```json
{
  "agent-a": {
    "key_b64": "<32-byte-HMAC-secret-in-base64>",
    "allowed_tools": ["demo_echo"],
    "certificate_pem_path": "/etc/ocaf/pki/agent-a.pem",
    "certificate_sha256": "<lowercase-64-hex-der-fingerprint>",
    "tls_dns_san": "agent-a.test"
  }
}
```

Stage the actual identities and TLS server keys outside Git, using
the same user-level 0600 permissions and independent PKI checks
required by priority 0.34.

Set:

```text
OCAF_RPC_INGRESS=tls-bound
OCAF_TLS_RPC_BIND=127.0.0.1:9443  # replace with explicit approved address
OCAF_TLS_SERVER_CERT_FILE=/etc/ocaf/pki/coordinator.pem
OCAF_TLS_SERVER_KEY_FILE=/etc/ocaf/pki/coordinator.key
OCAF_TLS_CLIENT_CA_FILE=/etc/ocaf/pki/client-root.pem
```

The server reads these three files with the private-file secret reader.
Other coordinator parameters (`OCAF_PEERS_FILE`, leader/replay/journal
paths or PostgreSQL cluster parameters) remain mandatory according to
their existing run mode. **Do not** configure a Zenoh fallback session.

The TCP wire format is deliberately minimal: one unsigned-big-endian
32-bit length prefix, one pre-signed JSON HMAC frame, then one
status-byte and length-prefixed HMAC response. Maximum frame
65,536 bytes, one request per TLS session, 8-second connection
timeout, 64 concurrent connections. ALPN **ocaf-rpc/1** and TLS 1.3
are required. Errors are indistinguishable on the wire.

## Client

`ocaf-rpc-node` in **client** mode supports the new transport,
without opening Zenoh:

```text
OCAF_RPC_INGRESS=tls-bound
OCAF_TLS_RPC_ADDRESS=127.0.0.1:9443
OCAF_TLS_RPC_SERVER_DNS=localhost
OCAF_TLS_CLIENT_CERT_FILE=/etc/ocaf/pki/agent-a.pem
OCAF_TLS_CLIENT_KEY_FILE=/etc/ocaf/pki/agent-a.key
OCAF_TLS_SERVER_CA_FILE=/etc/ocaf/pki/server-root.pem
OCAF_RPC_CERT_KEY_FILE=/etc/ocaf/pki/agent-a.key
```

In addition, the existing `OCAF_ROLE=client`,
`OCAF_NODE_ID=agent-a`, `OCAF_PEER_ID=<coordinator-node-id>`
and `OCAF_SHARED_KEY_B64` must match the server policy.
No automatic retry of effects. For now the demo `server` and
`worker` roles deliberately reject TLS-bound ingress rather than
quietly falling back to unauthenticated Zenoh queryables. Remote job
worker transport migration requires an additional stage.

## Test matrix

```bash
cargo test --locked --features tls-bound --test tls_bound_identity
cargo clippy --locked --features tls-bound --all-targets -- -D warnings
cargo fmt --all -- --check
```

The integration test generates ephemeral real P-256 CAs and
server/client certificates. It checks:
1. approved mTLS leaf, DNS SAN, HMAC and PoP succeeds;
2. legitimate HMAC and PoP **through a different trusted leaf**
   is rejected;
3. an untrusted CA is rejected during TLS handshake;
4. a wrong server hostname is rejected during TLS handshake;
5. a peer without a strict leaf pin cannot start the ingress;
6. a wrong DNS SAN fails configuration before serving.

## Unresolved production controls

- No CRL/OCSP automated verification yet. Removing a compromised
  leaf requires coordinated immediate policy deployment and restart;
  the PKI operator must separately revoke it. **Do not claim revocation.**
- This mode uses a distinct TCP/TLS endpoint. The original Zenoh
  RPC transport remains HMAC/PoP-only unless disabled by the exclusive
  process configuration; a firewall must also restrict any independent
  legacy RPC services.
- Authenticated client certs must have proper CA chain, valid time,
  clientAuth EKU; server certs must have a valid serverAuth EKU and
  the exact expected server DNS name. rustls verifies these.
- Real multi-host certificate rotation, failover, TLS key compromise,
  CRL enforcement and penetration tests remain required.
- Private keys are currently loaded as bounded PEM bytes; key zeroization
  at drop, TPM/HSM support and cert reload are not implemented.
- TLS client mode is currently CLI/client only; migrating all worker,
  heartbeat and observer traffic must occur before claiming a universal
  transport identity guarantee.

**Production status remains NO-GO.**
