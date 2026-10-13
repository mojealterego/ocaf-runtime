# OCAF 0.15 — Real Zenoh mutual-TLS handshake verification

**Scope:** cryptographic transport interoperability and negative trust
tests for the Rust Zenoh 1.10 client/coordinator on a single CI host.
The goal is to detect TLS/mTLS deployment regressions before rollout.
This is not equivalent to a production multi-host PKI audit.

## What is actually tested

`tests/zenoh_mtls_e2e.rs` provisions a **temporary two-day private
test PKI**, generated at test execution time with OpenSSL. No private
keys, certificate passwords or real service credentials are committed.
All private key paths are temporary and used solely by the test process.

1. A trusted CA signs a Zenoh listener certificate containing
   `IP:127.0.0.1` in its subject-alternative names and a separate
   client certificate authorized for client authentication.
2. Both configurations explicitly opt into `protocols: ["tls"]`,
   `enable_mtls: true`, `verify_name_on_connect: true`,
   `close_link_on_expiration: true`, disabled multicast/gossip scouting,
   and absolute certificate paths. The *same* operator preflight used
   by OCAF's CLI validates the configurations.
3. A real Zenoh listener and client session exchange a signed
   `RpcAction::Discover` over the resulting mTLS link.
4. With the same listener still active, a rogue client using a
   certificate signed by a distinct CA must fail connection.
5. A client trusting only the distinct CA must reject the server's
   certificate chain, even though the JSON5 preflight accepts the
   structurally valid configuration.
6. Even when the CA is trusted, a server certificate whose IP
   subject-alternative name does not match the actual 127.0.0.1
   endpoint must fail the real Zenoh connection handshake.

If an assertion fails, CI returns a failed status. The separate
`tests/transport_policy.rs` suite continues to exercise the
configuration policy's fail-closed behavior for insecure protocols,
missing credentials and unsafe files.

### Run tests

```bash
cargo test --test zenoh_mtls_e2e
cargo test
cargo clippy --all-targets -- -D warnings
```

The test requires a Unix host and OpenSSL CLI with `req` and `x509`.
It uses loopback TCP and random ephemeral listening ports. The
repository's GitHub Actions Ubuntu job runs it with the full default
Zenoh feature set; the test is intentionally excluded from
`--no-default-features` builds and non-Unix platforms.

## Deployment implications

Passing this test demonstrates that trusted certificates enable a real
signed RPC request over a Zenoh mutual TLS session, while two common
trust-anchor failures are rejected. It **does not validate production
certificates, multi-host routing, certificate revocation mechanisms or
the operational certificate rotation process**. The generated CA is
only a test credential and must never become a deployment trust root.

Production administrators still need to verify:
- certificate expiry, SAN/hostname trust and renewal on real hosts;
- certificate revocation and safe CA/key rotation without downtime;
- firewall rules, DNS/VIP failover, link encryption and authorized peers;
- PostgreSQL TLS trust and its independent replication/failover setup;
- mutual isolation of coordinator databases for different trust domains;
- network partitions, host shutdown and external-effect fencing.

See [DEPLOYMENT.md](DEPLOYMENT.md), [SECURITY.md](SECURITY.md),
and [POSTGRES_CLUSTER.md](POSTGRES_CLUSTER.md). External side effects
remain potentially ambiguous after losing a reply; at-most-once request
admission is not exactly-once effect execution.


## Priority 0.34 — independent certificate lifecycle preflight

In addition to the existing ephemeral Zenoh handshake tests, operators
can run `scripts/mtls_lifecycle.py` to check **actual staged or active**
server/client certificates for correct CA trust, key correspondence,
EKU, exact SAN and sufficient future validity. When a previous leaf is
provided, rotation candidates must have different SHA-256 fingerprints.
This is a read-only *operator* control, not hot PEM reload, automatic
CA issuance, CRL/OCSP revocation, or HMAC peer identity binding.

The static Zenoh policy now also rejects final-component symlinks,
weak permissions and non-user-owned private keys in the TLS file paths.
See [MTLS_LIFECYCLE_034.md](MTLS_LIFECYCLE_034.md) for explicit commands,
limitations, certificate staging and controlled restart procedures.
Real multi-host rotation and revocation remain unverified.
