# OCAF 0.27 / 0.42.3 — offline release evidence precheck

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

The private JSON manifest must contain exactly three top-level keys:
`schema_version` (integer `3`), `source_commit` (the full 40-character
lowercase SHA-1 of the checked-out candidate commit), and `checks` (a mapping
with **exactly** these keys):

1. `postgres_single_primary_fencing`
2. `multi_host_mtls_failover`
3. `credential_rotation_revocation`
4. `independent_sink_idempotency`
5. `tool_sandbox_isolation`
6. `backup_restore_rehearsal`
7. `stateful_release_rollback`
8. `dependency_license_security_audit`
9. `load_soak_slo`
10. `independent_security_review`

Versions 1 and 2 must be regenerated for version 3. Version 2 added the
stateful rollback check and evidence scopes; version 3 binds every check to
the candidate commit, a tracked workflow, an identified run and artifact,
and declared host identities. The operator directive file is a separate
contract and retains its own version number.

Each check has exactly `path`, `sha256`, `observed_at_utc`, `scope`,
`source_commit`, `workflow`, `workflow_sha256`, `run_id`, `run_attempt`,
`artifact_id` and `hosts`. The check's `source_commit` must equal the
manifest's candidate commit and repository `HEAD`. `Cargo.lock` and the
workflow must be regular files tracked by that commit and unchanged in the
working tree; tracked source changes also block the inventory. `workflow`
is a single `.github/workflows/*.yml` or `*.yaml` path. Its SHA-256 must equal
`workflow_sha256`, and its committed text must begin with exactly one
`# release-evidence-scope: <scope>` line matching the
check. A workflow declared `ephemeral_lab` cannot satisfy a real-host check.
The marker is a reviewed source declaration, not a physical-host attestation.

`run_id` is a nonzero decimal Actions run ID string, `run_attempt` is an
integer from 1 to 1000, and `artifact_id` is an opaque artifact identifier.
Each artifact ID, evidence path and digest must be unique across checks.
These identifiers let a reviewer retrieve the exact run and artifact; this
offline tool does not contact GitHub to verify them.

`path` is a single regular-file basename in the same private directory as
the manifest, without symbolic links, traversal or nested directories.
`sha256` is the lowercase SHA-256 digest of its nonempty, at-most-10-MiB
contents. `observed_at_utc` is an ISO-8601 UTC timestamp ending in `Z`.
Evidence older than 30 days, in the future, malformed or tampered is rejected.
The `scope` value must match the expected proof type: `independent_real_hosts`
for fencing, multi-host mTLS, credential rotation, independent sink, backup
restore and soak; `real_guest_host` for sandbox isolation; `dependency_audit`
for the dependency check; and `independent_human_review` for the security
review. `stateful_release_rollback` requires `stateful_real_host`, including
restoration of application state and denial of revoked credentials. The
ephemeral 0.42.4 CLI rehearsal cannot fill this check.

For `independent_real_hosts`, `hosts` must contain at least two entries with
distinct host IDs and Linux boot UUIDs. `stateful_real_host` requires one
`durable_real_host`; `real_guest_host` requires one
`durable_real_guest_host`. Each entry has exactly `host_id`, `boot_id` and
`host_class`. The two non-host checks use an empty list. Entries labeled
`ephemeral_ci` or lab hosts, missing identities, and reused host or boot IDs
are rejected for real-host checks. A statement of durable hosting remains a
declaration until an independent operator examines the machines and run.

For example, one entry in a v3 manifest looks like this (the remaining nine
checks are also required):

```json
{
  "schema_version": 3,
  "source_commit": "0123456789abcdef0123456789abcdef01234567",
  "checks": {
    "stateful_release_rollback": {
      "path": "stateful-rollback.json",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "observed_at_utc": "2026-10-28T12:00:00Z",
      "scope": "stateful_real_host",
      "source_commit": "0123456789abcdef0123456789abcdef01234567",
      "workflow": ".github/workflows/stateful-rollback.yml",
      "workflow_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "run_id": "123456789",
      "run_attempt": 1,
      "artifact_id": "operator-archive-2026-10-28",
      "hosts": [{
        "host_id": "ocaf-host-01",
        "boot_id": "01234567-89ab-cdef-0123-456789abcdef",
        "host_class": "durable_real_host"
      }]
    }
  }
}
```

The values above are illustrative; they do not identify a completed
stateful rollback. Do not relabel an ephemeral CI run or workflow as durable
host evidence.

Duplicate JSON object keys, extra/missing checks and oversized manifests
are also rejected. The checks are an inventory — test authenticity, host
independence and approval still require external verification. The manifest's
scope, run, artifact and host fields are operator assertions.

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
python3 scripts/release_readiness.py --repo . --evidence /var/lib/ocaf/evidence/manifest.json --require-production-approval
```

Without an operator override, exit `0` means the *offline inventory* passes
its structural, freshness and digest checks; exit `2` means a blocker or a
denied production-approval request. An explicit administrative override may
also return `0` while blockers remain, so always inspect the report status.
Neither outcome constitutes production approval; the report always includes
`production_approved: false`. The development-only `--report-only` mode
returns 0 despite missing evidence so ordinary Rust CI can report current
release blockers without claiming that a release passed. **Do not use
`--report-only` in an actual release approval gate.**
`--require-production-approval` exits 2 even for a structurally complete
manifest and even if an administrative operator override exists: this offline
inventory has no mechanism to independently certify real hosts or issue
production approval. An external change process must make that decision.

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
