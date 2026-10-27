#!/usr/bin/env python3
"""Exercise a narrow candidate-to-prior rollback on an ephemeral runner.

Only invoke after independently verifying both GitHub attestations. This
checks source binding and one stable CLI behavior, not database migration or
production rollback readiness.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tarfile
import tempfile
from contextlib import contextmanager
from pathlib import Path

from scripts.verify_release_bundle import ARCHIVE, BundleError
from scripts.verify_rollback_pair import verify_pair

PROBE = "ocaf-cluster-health"
INVALID_ARG = "--ocaf-rollback-probe-invalid"


@contextmanager
def _binary(bundle: Path):
    with tempfile.TemporaryDirectory(prefix="ocaf-rollback-") as directory:
        executable = Path(directory) / PROBE
        try:
            with tarfile.open(bundle / ARCHIVE, "r:gz") as tar:
                member = tar.getmember(PROBE)
                stream = tar.extractfile(member)
                if stream is None:
                    raise BundleError("rollback probe binary missing")
                content = stream.read(128 * 1024 * 1024 + 1)
        except (OSError, EOFError, KeyError, tarfile.TarError) as error:
            raise BundleError("rollback probe binary missing or invalid") from error
        if not content or len(content) > 128 * 1024 * 1024:
            raise BundleError("rollback probe binary invalid")
        executable.write_bytes(content)
        executable.chmod(0o500)
        yield executable, content


def _probe(bundle: Path) -> dict[str, object]:
    with _binary(bundle) as (executable, content):
        try:
            result = subprocess.run([str(executable), INVALID_ARG],
                                    capture_output=True, timeout=10,
                                    env={"PATH": "/usr/bin:/bin"}, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise BundleError("rollback CLI probe did not complete") from error
        if result.returncode != 1 or b"unknown option" not in result.stderr:
            raise BundleError("rollback CLI behavior mismatch")
        return {"binary_sha256": hashlib.sha256(content).hexdigest(),
                "invalid_option_exit": result.returncode}


def database_probe(bundle: Path, expected_pending: int, dsn: str) -> dict[str, object]:
    """Check that an attested CLI reads the expected restored PG state."""
    if not 0 <= expected_pending <= 1000 or not dsn:
        raise BundleError("invalid PostgreSQL rollback probe configuration")
    with _binary(bundle) as (executable, content):
        env = {"PATH": "/usr/bin:/bin", "OCAF_CLUSTER_PG_DSN": dsn,
               "OCAF_CLUSTER_ID": "stage042_rollback",
               "OCAF_ALLOW_INSECURE_LOOPBACK": "1"}
        try:
            result = subprocess.run([str(executable), "--json", "--no-fail"],
                                    capture_output=True, timeout=15, env=env,
                                    check=False)
            if result.returncode != 0:
                raise BundleError("PostgreSQL rollback CLI failed")
            payload = json.loads(result.stdout)
        except (OSError, subprocess.TimeoutExpired, ValueError) as error:
            raise BundleError("PostgreSQL rollback CLI invalid response") from error
        snapshot = payload.get("snapshot") if isinstance(payload, dict) else None
        if (not isinstance(snapshot, dict) or
                type(snapshot.get("jobs_pending")) is not int or
                snapshot["jobs_pending"] != expected_pending):
            raise BundleError("PostgreSQL rollback state mismatch")
        return {"binary_sha256": hashlib.sha256(content).hexdigest(),
                "jobs_pending": snapshot["jobs_pending"],
                "scope": "single ephemeral PostgreSQL snapshot"}


def revoked_database_probe(bundle: Path, revoked_dsn: str) -> dict[str, object]:
    """After a successful snapshot probe, deny use of a rotated DB password."""
    if not revoked_dsn:
        raise BundleError("missing revoked PostgreSQL credential")
    password = revoked_dsn.partition("password=")[2].split(" ", 1)[0]
    if not password:
        raise BundleError("revoked PostgreSQL probe needs a password")
    with _binary(bundle) as (executable, content):
        env = {"PATH": "/usr/bin:/bin", "OCAF_CLUSTER_PG_DSN": revoked_dsn,
               "OCAF_CLUSTER_ID": "stage042_rollback",
               "OCAF_ALLOW_INSECURE_LOOPBACK": "1"}
        try:
            result = subprocess.run([str(executable), "--json", "--no-fail"],
                                    capture_output=True, timeout=15, env=env,
                                    check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise BundleError("revoked PostgreSQL probe did not complete") from error
        if (result.returncode == 0 or result.stdout or
                password.encode() in result.stderr):
            raise BundleError("revoked PostgreSQL credential was accepted or leaked")
        return {"binary_sha256": hashlib.sha256(content).hexdigest(),
                "revoked_credential_denied": True,
                "scope": "ephemeral PostgreSQL password rotation"}


def drill(repo: Path, candidate: Path, prior: Path, candidate_sha: str,
          prior_sha: str, repository: str) -> dict[str, object]:
    pair = verify_pair(repo, candidate, prior, candidate_sha, prior_sha, repository)
    candidate_result = _probe(candidate)
    prior_result = _probe(prior)
    return {**pair, "status": "EPHEMERAL_ROLLBACK_CLI_PASS",
            "candidate_probe": candidate_result, "prior_probe": prior_result,
            "behavior_verified": True,
            "behavior_scope": "single invalid-option CLI probe on ephemeral runner",
            "signature_verified": False, "production_approved": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("repo", "candidate", "prior"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("candidate_sha", "prior_sha", "repository"):
        parser.add_argument("--" + name.replace("_", "-"), required=True)
    args = parser.parse_args()
    try:
        report = drill(args.repo, args.candidate, args.prior,
                       args.candidate_sha, args.prior_sha, args.repository)
    except BundleError as error:
        print(json.dumps({"status": "FAIL", "reason": str(error)}))
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
