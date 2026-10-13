# OCAF 0.33 — non-destructive three-host PostgreSQL HA acceptance probe

Date: 2026-10-13. **Scope: staging/lab PostgreSQL leadership only. Production NO-GO.**

## Purpose and limits

OCAF already tests PostgreSQL epoch fencing and Zenoh mTLS in GitHub Actions
on one Linux runner. Priority 0.33 adds a separately installable binary
`ocaf-ha-probe` and bounded three-host SSH orchestrator. This allows
operators to gather direct test evidence from three explicitly configured
machines: host A holds a dedicated test-namespace PostgreSQL advisory lock,
B and C must be denied, then A releases, B and C acquire the same lock
in turn with strictly increasing epoch values.

**The harness cannot prove the machines are physically independent**:
a DNS alias could point to the same host. Independently verify inventory,
hardware, administrative boundaries and DB topology. The probe
tests graceful handover, not actual coordinator crash/power loss, Zenoh RPC,
PostgreSQL replication, restoration, receiver-side fencing, mTLS rotation,
revocation, backup durability, or sandbox isolation. It creates/updates
the **dedicated test cluster's leader epoch row** through the existing
PgLeader API; do not point it at production. It does not kill a host,
terminate a database backend, or execute external tool effects.

The test binary is behind Cargo feature `ha-probe`; it is **not**
included in standard release archives. Running it requires a separate
operator build and explicit staging environment confirmation.

## Prerequisites

1. Provision three separate staging Linux hosts with current reviewed OCAF
   source/binary. Provision isolated PostgreSQL (not live production), a
   dedicated least-privilege testing role with epoch table permission and
   TLS certificate trust. Keep replicas and PostgreSQL HA external.
2. Install the opt-in probe binary on each host:

   ```bash
   cargo build --locked --release --no-default-features --features ha-probe --bin ocaf-ha-probe
   sudo install -m 0755 target/release/ocaf-ha-probe /usr/local/bin/ocaf-ha-probe
   ```

3. Configure `/etc/ocaf/pg-dsn` separately on all three hosts, readable
   only by their probe service user, using production-grade TLS and
   validated server identity; **do not use test credentials on production**.
   The binary uses `OCAF_CLUSTER_PG_DSN_FILE` through the existing
   restricted private-file reader. Do not put DSN/passwords in command lines.
4. Configure SSH keys and **pre-pinned known_hosts fingerprints**,
   non-interactive public-key access for distinct test operator identities,
   and verify physical inventory independently. The SSH harness forces
   `BatchMode=yes`, `StrictHostKeyChecking=yes` and non-loopback PostgreSQL TLS.
5. Use a random, dedicated cluster identifier each run. Example:

   ```bash
   python3 -c 'import secrets; print("ocaf33_" + secrets.token_hex(16))'
   ```

## Three-host test

Run only after explicit staging approval and with a reviewed new
cluster identifier:

```bash
python3 scripts/ha_multihost_acceptance.py \
  --coordinator-a ocaf@node-a.example.net \
  --coordinator-b ocaf@node-b.example.net \
  --observer-c ocaf@node-c.example.net \
  --cluster-id ocaf33_0123456789abcdef0123456789abcdef \
  --confirm-isolated-test
```

Replace the sample SSH destinations and sample cluster ID with verified
staging-only hosts and a freshly generated ID. Host aliases in this
example are **not** provisioned destinations. The command requires the
operator to run it explicitly; it is not scheduled in GitHub CI.
The orchestrator captures JSON observations, checks exact test cluster
identity, denies concurrent acquisition at B and C, then requires the
test leader on A to release and verifies monotonic epoch acquisition
first on B, then on C.

The SSH result may be `LAB_PROBE_PASS_NOT_PRODUCTION_APPROVAL` only if
all assertions pass. It includes test-time epoch values and the names
of SSH destinations but **no DSN or cryptographic credentials**.
Capture shell exit code, trusted inventory, signed operator attestations,
server logs, the precise OCAF release digest and timestamps. The JSON
report is not cryptographically signed by this tool.

## Automated preconditions

GitHub CI now additionally executes an **opt-in subprocess smoke test**
on its disposable PostgreSQL 16 instance:
- while child A holds the advisory lock, child B must receive denial;
- when child A explicitly releases, child B must obtain a larger epoch;
- the probe rejects missing operator acknowledgement and forbidden cluster
  names before attempting a database connection;
- Python regression tests reject unsafe SSH destinations and invalid
  probe records/epoch order.

This CI test validates the probe mechanics **only on one runner**.
It must not be counted as evidence of three physical hosts.

## Remaining acceptance (mandatory NO-GO)

- [ ] Three independent machines and trust domains documented and audited.
- [ ] Actual coordinator promotion after loss of host/power, with stale
      writes/effect-sink acceptance rejected in independent logs.
- [ ] Single-writer PostgreSQL HA under quorum loss, with external fencing.
- [ ] Zenoh mTLS certificate/SAN/CA rotation and negative revoked client tests.
- [ ] Controlled partition, healing, and compromised worker/tool sandbox testing.
- [ ] Restored PostgreSQL and SQLite journals; reconcile uncertain jobs.
- [ ] Approved soak SLOs and independent security review.

See `docs/FINAL_ACCEPTANCE.md` and `docs/CHAOS_ENGINEERING.md`
for the existing production-signoff checklist.
