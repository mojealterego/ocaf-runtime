#!/usr/bin/env python3
"""Offline preflight for a candidate-to-prior release rollback rehearsal.

This validates two distinct source-bound bundles and ancestry. CI must also
verify each bundle's GitHub attestation before executing either binary.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from scripts.verify_release_bundle import BundleError, GIT_SHA, verify_bundle


def verify_pair(repo: Path, candidate: Path, prior: Path,
                candidate_sha: str, prior_sha: str, repository: str) -> dict:
    if not GIT_SHA.fullmatch(candidate_sha) or not GIT_SHA.fullmatch(prior_sha):
        raise BundleError("invalid commit SHA")
    if candidate_sha == prior_sha:
        raise BundleError("rollback target must differ from candidate")
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "merge-base", "--is-ancestor", prior_sha, candidate_sha],
            check=False, capture_output=True, timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise BundleError("cannot verify source ancestry") from error
    if result.returncode != 0:
        raise BundleError("prior commit is not a known ancestor of candidate")
    old = verify_bundle(prior, expected_sha=prior_sha, expected_repository=repository)
    new = verify_bundle(candidate, expected_sha=candidate_sha, expected_repository=repository)
    if old["sha256"]["ocaf_runtime-linux-x86_64.tar.gz"] == new["sha256"]["ocaf_runtime-linux-x86_64.tar.gz"]:
        raise BundleError("rollback target binary archive is identical to candidate")
    return {"status": "PAIR_VERIFIED_UNATTESTED", "candidate_sha": candidate_sha,
            "prior_sha": prior_sha, "signature_verified": False,
            "behavior_verified": False, "production_approved": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--prior", type=Path, required=True)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--prior-sha", required=True)
    parser.add_argument("--repository", required=True)
    args = parser.parse_args()
    try:
        report = verify_pair(args.repo, args.candidate, args.prior,
                             args.candidate_sha, args.prior_sha, args.repository)
    except BundleError as error:
        print(json.dumps({"status": "FAIL", "reason": str(error)}))
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
