#!/usr/bin/env python3
"""Fail-closed, offline OCAF release evidence precheck.

This tool verifies a dependency snapshot and the integrity/freshness of
operator-supplied artifacts. It does not test infrastructure or authorize
production deployment. Only an independent human change process can do that.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import sys
import tomllib
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

REQUIRED_EVIDENCE = (
    "postgres_single_primary_fencing",
    "multi_host_mtls_failover",
    "credential_rotation_revocation",
    "independent_sink_idempotency",
    "tool_sandbox_isolation",
    "backup_restore_rehearsal",
    "dependency_license_security_audit",
    "load_soak_slo",
    "independent_security_review",
)
MAX_EVIDENCE_AGE = timedelta(days=30)
MAX_ARTIFACT_BYTES = 10 * 1024 * 1024
MAX_MANIFEST_BYTES = 1024 * 1024
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def _unique_fields(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON field")
        value[key] = item
    return value


def _valid_lockfile(repo: Path) -> str | None:
    lock = repo / "Cargo.lock"
    if not lock.is_file() or lock.is_symlink():
        return "cargo_lock_missing"
    try:
        if lock.stat().st_size > MAX_ARTIFACT_BYTES:
            return "cargo_lock_invalid"
        data = tomllib.loads(lock.read_text(encoding="utf-8"))
        if type(data.get("version")) is not int or data["version"] not in (3, 4):
            return "cargo_lock_invalid"
        packages = data.get("package")
        if not isinstance(packages, list) or not packages:
            return "cargo_lock_invalid"
        for p in packages:
            if not isinstance(p, dict) or not isinstance(p.get("name"), str) or not isinstance(p.get("version"), str):
                return "cargo_lock_invalid"
            source = p.get("source", "")
            if source and not isinstance(source, str):
                return "cargo_lock_invalid"
            if source.startswith("registry+") and not SHA256.fullmatch(p.get("checksum", "")):
                return "cargo_lock_invalid"
        if not any(p["name"] == "ocaf_runtime" for p in packages):
            return "cargo_lock_invalid"
    except (OSError, UnicodeError, ValueError, TypeError, tomllib.TOMLDecodeError):
        return "cargo_lock_invalid"
    return None


def _manifest(manifest: Path) -> dict[str, Any]:
    if manifest.is_symlink() or not manifest.is_file() or manifest.stat().st_size > MAX_MANIFEST_BYTES:
        raise ValueError("invalid manifest")
    data = json.loads(manifest.read_text(encoding="utf-8"), object_pairs_hook=_unique_fields)
    if (not isinstance(data, dict) or set(data) != {"schema_version", "checks"}
            or type(data["schema_version"]) is not int or data["schema_version"] != 1):
        raise ValueError("invalid manifest schema")
    checks = data["checks"]
    if not isinstance(checks, dict) or set(checks) != set(REQUIRED_EVIDENCE):
        raise ValueError("incomplete or unknown evidence check")
    return checks


def _verify_evidence(root: Path, name: str, data: Any, now: datetime) -> str | None:
    if not isinstance(data, dict) or set(data) != {"path", "sha256", "observed_at_utc"}:
        return f"evidence_schema_invalid:{name}"
    raw_path, checksum, observed = data["path"], data["sha256"], data["observed_at_utc"]
    if not isinstance(raw_path, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", raw_path) or raw_path in (".", ".."):
        return f"evidence_path_unsafe:{name}"
    file = root / raw_path
    if file.is_symlink() or not file.is_file():
        return f"evidence_path_unsafe:{name}"
    try:
        mode = file.stat().st_mode
        size = file.stat().st_size
        if not stat.S_ISREG(mode) or not 0 < size <= MAX_ARTIFACT_BYTES:
            return f"evidence_artifact_invalid:{name}"
    except OSError:
        return f"evidence_artifact_invalid:{name}"
    if not isinstance(checksum, str) or not SHA256.fullmatch(checksum):
        return f"evidence_hash_invalid:{name}"
    try:
        actual = hashlib.sha256()
        with file.open("rb") as handle:
            for chunk in iter(lambda: handle.read(65536), b""):
                actual.update(chunk)
        if actual.hexdigest() != checksum:
            return f"evidence_hash_mismatch:{name}"
    except OSError:
        return f"evidence_artifact_invalid:{name}"
    if not isinstance(observed, str) or not observed.endswith("Z"):
        return f"evidence_timestamp_invalid:{name}"
    try:
        timestamp = datetime.fromisoformat(observed.replace("Z", "+00:00"))
    except ValueError:
        return f"evidence_timestamp_invalid:{name}"
    if timestamp.tzinfo is None:
        return f"evidence_timestamp_invalid:{name}"
    age = now - timestamp.astimezone(timezone.utc)
    if age < timedelta(0) or age > MAX_EVIDENCE_AGE:
        return f"evidence_stale:{name}"
    return None


def evaluate(repo: Path, manifest: Path | None, now: datetime | None = None) -> dict[str, Any]:
    """Produce an offline evidence inventory, never a production approval."""
    now = now or datetime.now(timezone.utc)
    if now.tzinfo is None:
        raise ValueError("now must have timezone information")
    blockers: list[str] = []
    lock_issue = _valid_lockfile(repo)
    if lock_issue:
        blockers.append(lock_issue)
    if manifest is None:
        blockers.append("evidence_manifest_missing")
    else:
        try:
            checks = _manifest(manifest)
        except (OSError, UnicodeError, ValueError, TypeError):
            blockers.append("evidence_manifest_invalid")
        else:
            for name in REQUIRED_EVIDENCE:
                issue = _verify_evidence(manifest.parent, name, checks[name], now)
                if issue:
                    blockers.append(issue)
    return {
        "schema_version": 1,
        "status": "NO_GO" if blockers else "EVIDENCE_COMPLETE_UNVERIFIED",
        "production_approved": False,
        "blockers": blockers,
        "required_evidence_checks": list(REQUIRED_EVIDENCE),
        "limitations": "Artifact hashes and dates do not prove test authenticity, host independence, operating controls, or production approval.",
    }



REQUIRED_OPERATOR_DIRECTIVE = frozenset({
    "schema_version", "git_branch", "operator_directive_date",
    "operator_authorization_source", "operator_decision",
    "administrative_release_blocked", "stage_038_administrative_status",
    "stage_039_development", "safety_acceptance_a01_a12",
    "third_party_audit_evidence", "verified_production_readiness",
    "production_deployed", "risk_waiver_notes",
})


def apply_operator_override(report: dict[str, Any], policy_path: Path,
                            *, acknowledged: bool) -> dict[str, Any]:
    """User-authorized administrative release, NOT evidence or actual deployment."""
    if acknowledged is not True:
        raise ValueError("explicit acknowledgement of outstanding risk required")
    if (policy_path.is_symlink() or not policy_path.is_file() or
            not 0 < policy_path.stat().st_size <= 8192):
        raise ValueError("operator directive missing, linked or oversized")
    try:
        directive = json.loads(policy_path.read_text(encoding="utf-8"),
                               object_pairs_hook=_unique_fields)
    except (OSError, UnicodeError, ValueError) as exc:
        raise ValueError("unreadable operator directive") from exc
    if not isinstance(directive, dict) or set(directive) != REQUIRED_OPERATOR_DIRECTIVE:
        raise ValueError("invalid operator directive schema")
    expected = {
        "schema_version": 1, "git_branch": "main",
        "operator_decision": "UNBLOCK_PRODUCTION_DEPLOYMENT",
        "administrative_release_blocked": False,
        "stage_038_administrative_status": "CLOSED_BY_OPERATOR",
        "stage_039_development": "AUTHORIZED",
        "safety_acceptance_a01_a12": "INCOMPLETE",
        "third_party_audit_evidence": "INCOMPLETE",
        "verified_production_readiness": False,
        "production_deployed": False,
    }
    if any(type(directive[k]) is not type(v) or directive[k] != v for k, v in expected.items()):
        raise ValueError("operator directive contradicts factual release state")
    for k in ("operator_directive_date", "operator_authorization_source",
              "risk_waiver_notes"):
        v = directive[k]
        if not isinstance(v, str) or not 5 <= len(v) <= 500:
            raise ValueError("directive lacks risk/authorization context")
    state = dict(report)
    state.update(status="OPERATOR_AUTHORIZED_UNVERIFIED",
                 administrative_release_unblocked=True,
                 operator_risk_acknowledged=True,
                 technical_evidence_passed=False,
                 production_approved=False,
                 deployment_executed=False)
    state["limitations"] += (
        " Operator explicitly authorized an unverified administrative release "
        "exception. All evidence blockers remain; physical host isolation, "
        "auditor qualifications and live deployment are not attested."
    )
    return state



def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--evidence", type=Path, default=None)
    parser.add_argument("--report-only", action="store_true", help="emit report without blocking development CI; NOT a production override")
    parser.add_argument("--operator-override-policy", type=Path,
                        help="operator's explicit risk waiver (not technical acceptance)")
    parser.add_argument("--ack-unverified-production-risk", action="store_true",
                        help="explicitly acknowledge unverified evidence before release")
    args = parser.parse_args(argv)
    report = evaluate(args.repo, args.evidence)
    if args.operator_override_policy is not None or args.ack_unverified_production_risk:
        if args.operator_override_policy is None:
            parser.error("--operator-override-policy required")
        try:
            report = apply_operator_override(
                report, args.operator_override_policy,
                acknowledged=args.ack_unverified_production_risk)
        except ValueError as exc:
            parser.error(str(exc))
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    if report.get("administrative_release_unblocked"):
        return 0
    return 0 if args.report_only or not report["blockers"] else 2


if __name__ == "__main__":
    raise SystemExit(main())