# Priority 0.35 — bounded HMAC peer credential rotation and identity hardening

**Scope:** verified signed-RPC peer identity and scheduled HMAC rotation.
**Explicit limitation:** Zenoh TLS peer certificate SAN is **not** exposed to
the RPC authorization layer. This change does **not** bind peer HMAC identity
to a verified mTLS certificate. Such a claim requires a separate, audited
transport-to-RPC cryptographic identity channel.

## Current behavior

A configured peer's `key_b64` remains its old signing credential.
Operator may opt into a bounded rotation by setting **all three**:

```json
{
  "agent-a": {
    "key_b64": "<BASE64_32_BYTE_OLD_KEY>",
    "next_key_b64": "<BASE64_32_BYTE_NEW_KEY>",
    "activate_at_ms": 1800000000000,
    "retire_old_at_ms": 1800000060000,
    "allowed_tools": ["demo_echo"],
    "submit_jobs": false,
    "worker_jobs": false,
    "write_effects": false
  }
}
```

The timestamps are *illustrative*, not a ready-to-deploy schedule.
Replace both keys using a trusted secret manager. Both HMAC credentials
must contain different nonzero 32-byte random secrets. Time values must
describe a **positive overlap no longer than 30 minutes**.

* Before `activate_at_ms`: **only old** key accepted.
* From `activate_at_ms` through the interval before
  `retire_old_at_ms`: both old and new keys accepted, with the signed
  response using **the same key as the request**.
* From `retire_old_at_ms`: **only new** key accepted, old key rejected,
  including at the exact deadline.

The RPC sender claim is used to select exactly one peer's policy; the
claimed identity remains untrusted until the HMAC verifies. Replay
protection and rate quotas are shared per sender across the overlap;
changing a key cannot bypass the replay journal. ACL, worker, job
submission and effect-write authorizations remain independent of the
credential used to authenticate the request.

The `ocaf-coordinator` peer-file reader requires all three rotation
fields together and rejects reused credentials across peers (including
old-versus-new reuse). Incomplete configuration fails startup.
The gateway library exposes `PeerPolicy::with_key_rotation(...)`; other
application-created gateways need explicitly configured rotation.
There is no implicit compatibility fallback.

## Operator rollout

1. Provision and verify the intended Zenoh mTLS certificate and CA
   separately (see `MTLS_LIFECYCLE_034.md`). Do not treat HMAC key
   possession as proof of the TLS certificate's SAN.
2. Write one owner-only `OCAF_PEERS_FILE` containing an old and new
   credential and a short future overlap window. Deploy the same schedule
   to every standby candidate. Use synchronized, independently monitored
   UTC time on all hosts; clock rollback can invalidate cutover intent.
3. Reload/restart the coordinator service with reviewed configuration;
   this code does **not** hot-reload peer files. New-key clients start
   using the new secret no earlier than `activate_at_ms`.
4. Verify an authenticated response from new-key clients and rejection
   of old-key requests at `retire_old_at_ms`. Do not assume network
   mTLS connection establishment constitutes RPC authorization.
5. After retirement, remove the obsolete secret from the service
   configuration, rotate external secret storage, redeploy standbys
   and audit replay/authorization logs.
6. For emergency compromise, **do not schedule an overlap with a
   compromised key**. Remove/revoke it immediately by replacing the
   peer configuration and restarting *all* exposed coordinators; take
   independent steps to revoke TLS credentials and assess persistence.

The configured cutover schedule is read on process startup. If a
coordinator crashes and starts with stale peer configuration, rollback
could reauthorize an old key. Operators must distribute and verify the
same current peer policy on every host before enabling the new endpoint.
No key material is included in log/audit output.

## Verification

```bash
cargo test --locked --no-default-features --test rpc_key_rotation
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

CI also tests `ocaf-coordinator` JSON peer configuration and confirms
that old HMAC credentials are rejected at the deadline. These tests
do not validate end-to-end TLS peer certificate identity. Physical
multi-host tests, certificate revocation, verified single-primary
PostgreSQL, sandbox isolation, backup restoration and long-running
soak/pen testing remain required.

**Production readiness: NO-GO.**
