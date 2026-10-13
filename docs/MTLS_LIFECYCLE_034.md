# OCAF 0.34 — mTLS certificate lifecycle security preflight

**2026-10-13 | Operational verification, not automated key rotation. Production: NO-GO.**

## Deliverables

- `scripts/mtls_lifecycle.py` is a read-only OpenSSL-based operational gate.
  It validates a candidate server or client leaf against a specified trust
  anchor, appropriate X.509 extended key usage, exact expected DNS/IP SAN,
  current validity and a configurable minimum future validity window.
- Its public-key match check ensures the staged certificate corresponds to
  the operator-supplied private key. It rejects key files accessible to
  other users, final-component symlinks, oversized material and writable
  CA/certificate files.
- `--previous-cert` rejects a no-op rotation by requiring a changed SHA-256
  leaf certificate fingerprint. The JSON receipt contains public identities,
  fingerprints and scope; no private key material is printed.
- Structural Zenoh config preflight now opens certificate/key files with
  `O_NOFOLLOW` on Unix, rejects group/world-writable material and requires
  the private-key file to be owned by the running user with owner-only mode.
  The Zenoh configuration reader also refuses final-component symlinks.
- CI exercises real ephemeral OpenSSL P-256 PKI positive/negative test cases
  before supply-chain artifacts are created.

This gate reads explicit certificate paths. **It does not parse the live Zenoh
config, bind a TLS certificate SAN to an HMAC peer ID, monitor running TLS
connections, dynamically reload certificates, issue keys, revoke certificates,
or automatically restart services.** A trusted operator must ensure the
provided paths and identity are exactly those of the actual deployment.

The file preflight verifies metadata before invoking OpenSSL, which then
reopens files. Untrusted actors with write access to ancestor directories
could race replacements. All certificate parent directories must be protected
by operators. The first PEM certificate in the CA file is checked for
expiration; use independently reviewed anchors when rotating a CA bundle.

## Invocation — on the matching host and service account

An illustrative server check:

```bash
python3 scripts/mtls_lifecycle.py \
  --role server \
  --cert /etc/ocaf/pki/coordinator.pem \
  --key /etc/ocaf/pki/coordinator.key \
  --ca /etc/ocaf/pki/root-ca.pem \
  --expected-dns coordinator.internal.example \
  --min-valid-seconds 604800
```

A client/worker check:

```bash
python3 scripts/mtls_lifecycle.py \
  --role client \
  --cert /etc/ocaf/pki/agent.pem \
  --key /etc/ocaf/pki/agent.key \
  --ca /etc/ocaf/pki/root-ca.pem \
  --expected-dns agent.internal.example \
  --min-valid-seconds 604800
```

**These are examples; the mentioned paths and hostnames are not provisioned.**
Exit status `0`: validated; `2`: fail-closed. Use `--expected-ip`
instead of `--expected-dns` if the certificate has an explicit IP SAN.
To verify a staged replacement, add
`--previous-cert /etc/ocaf/pki/coordinator.pem` to the same command and
point `--cert` and `--key` at the **candidate**, not the active credential.

## Controlled rollout / rollback

1. Stage a candidate certificate with the authorized CA and owner-only
   private key in a service-controlled, non-writable-by-other-users tree.
   Validate both its identity and signing chain using this preflight,
   check the receipt against the approved change request and separately
   assess revocation through the external PKI.
2. If changing the CA, first establish a reviewed trust overlap on **both**
   communication ends and validate old/new client and server handshakes.
   A changed trust root must not silently authorize new RPC peers; update
   HMAC peer policy, permissions and independent workload identities.
3. Switch deployment references atomically under maintenance controls,
   restart/recreate Zenoh sessions using the operator's service manager
   (hot reload has not been implemented), repeat the preflight against
   **active files**, then run signed RPC/mTLS and effect fencing tests.
4. Verify expiry monitoring, heartbeat recovery, SAN/hostname checks and
   rejection of unauthorized clients. Remove retired trust only after
   migration and documented rollback criteria. Revoke compromised
   certificates in the external system and independently rotate HMAC keys.
5. Record operator and PKI attestations, source commit SHA, active leaf and
   trust-anchor fingerprints, actual failed/successful handshakes, and
   downtime/restart observations. Maintain a separate revocation and
   emergency rollback runbook.

## Local verification

```bash
python3 -m unittest discover -s scripts/tests -p test_mtls_lifecycle.py -v
cargo test --locked --test transport_policy
cargo test --locked --test zenoh_mtls_e2e
```

CI tests validate ephemeral private PKI; they do **not** prove real-host
rotation, external revocation, peer-to-certificate identity binding,
PostgreSQL single-primary fencing or trusted execution sandboxes.

**Remaining signoff: NO-GO** pending multi-host PKI rotation, revocation,
live failover, workload identity binding, isolated effect execution,
backup/restore and independent security review.
