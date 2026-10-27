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
from pathlib import Path

from scripts.verify_release_bundle import ARCHIVE, BundleError
from scripts.verify_rollback_pair import verify_pair

PROBE = "ocaf-cluster-health"
INVALID_ARG = "--ocaf-rollback-probe-invalid"


def _probe(bundle: Path) -> dict[str, object]:
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
