# 0.40 — asymmetric witness admission for PostgreSQL effects

The stage 0.40 majority admission gate now accepts Ed25519 votes from three or more independently keyed witnesses. The candidate-side `QuorumCertificateGate` receives **only raw public keys** via `witness_public_keys`; `issue_vote_ed25519` runs at a witness and needs its private key. It signs a canonical lease bound to cluster, candidate, epoch, lease ID, witness ID, issue time and expiry under the `OCAF-STAGE040-WITNESS-ED25519-V1` domain. The gate requires a strict majority of distinct witnesses signing the identical lease, then persists the accepted certificate and highest epoch in an owned private SQLite database. The persisted membership digest binds the public keys, threshold and signature scheme. Restart with a changed key, threshold, clock domain or HMAC scheme fails closed.

The previous HMAC gate remains available only with `allow_shared_secret_lab=True` for legacy local tests. A candidate with its HMAC verifier keys could forge witness votes, so this mode is not suitable for a candidate separated from the witness authorities. Neither mode proves that witness processes are deployed on independent hosts or that PostgreSQL itself fences direct SQL clients.

## Local verification

```bash
python3 -m unittest scripts.tests.test_stage040_quorum_certificate scripts.tests.test_stage040_pg_witness_effect_lab -v
```

The `stage040-pg-witness-effect-lab.yml` workflow also starts a real, disposable PostgreSQL postmaster and checks admitted writes, minority denial and stale epoch denial. The lab generates all three private keys in one process and one kernel; the gate receives only public keys, but the lab process retains the private keys. It is a cryptographic and adapter test, not an independent witness deployment. The report sets `candidate_gate_holds_only_public_witness_keys=true`, `postgresql_server_side_fencing_proved=false`, `three_host_ha_tested=false`, `stage_040_complete=false` and `production_approved=false`.

## Remaining acceptance work

Provision three durable independent witness hosts and three PostgreSQL hosts, keep private keys inside their witness domains, authenticate vote requests, and demonstrate live TCP WAL, promotion after majority, physical isolation of the stale primary, return without dual writable nodes, and the required 60-minute 8–32-client fencing soak. The local admission test does not satisfy those gates.
