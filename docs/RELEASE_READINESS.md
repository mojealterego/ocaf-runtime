# OCAF 0.27 — offline release evidence precheck

## Scope

The repository contains `scripts/release_readiness.py`, a **fail-closed
release evidence inventory** for operators. It does not deploy or test an
actual multi-host cluster. Its successful outcome is
`EVIDENCE_COMPLETE_UNVERIFIED` — **never production certification**.
Independent human review and deployment/change authorization remain mandatory.

`Cargo.lock` is a required checked-in dependency snapshot for a release
candidate; it was previously ignored by `.gitignore`. Dependency pinning,
third-party advisory/license scanning and signed release provenance remain
independent release requirements. A well-formed lockfile is not an audit.

## Evidence contract

The private JSON manifest must contain exactly two top-level keys:
`schema_version` (integer `1`) and `checks` (a mapping with **exactly**
these keys):

1. `postgres_single_primary_fencing`
2. `multi_host_mtls_failover`
3. `credential_rotation_revocation`
4. `independent_sink_idempotency`
5. `tool_sandbox_isolation`
6. `backup_restore_rehearsal`
7. `dependency_license_security_audit`
8. `load_soak_slo`
9. `independent_security_review`

Each check has exactly `path`, `sha256` and `observed_at_utc`. `path` is a
single regular-file basename in the same private directory as the manifest,
without symbolic links, traversal or nested directories. `sha256` is the
lowercase SHA-256 digest of its nonempty, at-most-10-MiB contents.
`observed_at_utc` is an ISO-8601 UTC timestamp ending in `Z`. Evidence
older than 30 days, in the future, malformed or tampered is rejected.
Duplicate JSON object keys, extra/missing checks and oversized manifests
are also rejected. The checks are an inventory — test authenticity, host
independence and approval still require external verification.

Store evidence **outside the repository** and outside publicly accessible
CI artifacts; the verifier has no need for credentials, personal data,
private keys or raw customer payloads. Use redacted, operator-approved
logs and attestations. Protect the manifest directory against untrusted
writers and review artifacts separately.

## Run

From the repository root, using Python 3.11+:

```bash
python3 -m unittest discover -s scripts/tests -v
python3 scripts/release_readiness.py --repo .
python3 scripts/release_readiness.py --repo . --evidence /var/lib/ocaf/evidence/manifest.json
```

Exit codes: `0` means the *offline inventory* contains the expected files
and passes structural, freshness and digest checks, `2` means `NO_GO`.
Neither outcome constitutes production approval; the report always includes
`production_approved: false`. The development-only `--report-only` mode
returns 0 despite missing evidence so ordinary Rust CI can report current
release blockers without claiming that a release passed. **Do not use
`--report-only` in an actual release approval gate.**

At the time this check was introduced, `Cargo.lock` was not committed and
all physical infrastructure evidence was unverified. The expected default
result is `NO_GO`. Before a release, generate and review `Cargo.lock`,
commit it on `main`, run pinned Rust builds with `--locked`, check advisories
and licenses, and review the operational evidence with independent personnel.

## External signoff remains required

Evidence from two coordinator hosts and an independently fenced PostgreSQL
primary must be evaluated by real multi-host failure drills, PKI rollover,
network partitions, backup-and-restore, external sink reconciliation,
least-privilege execution, and signed-build provenance. A fabricated log
with a matching digest is still fabricated; neither this script nor GitHub
Actions can certify production HA merely by checking files.