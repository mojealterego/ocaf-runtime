#!/usr/bin/env python3
"""Stage 0.56: conservative static security critic for admitted OmniDev diffs.

This is a detector of *selected* risky additions, not a security proof or
semantic program analyzer. Even a clean result requires independent review and
an authenticated, isolated compile/test gate before any write or deployment.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

from omnidev_patch_gate import GateError, _read_patch, inspect_patch

RULES: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("UNSAFE_RUST", re.compile(r"\bunsafe\b", re.I)),
    ("TLS_VERIFICATION_BYPASS", re.compile(
        r"danger_accept_invalid_certs|danger_accept_invalid_hostnames|"
        r"\bverify\s*=\s*False\b|GIT_SSL_NO_VERIFY|CERT_NONE", re.I)),
    ("SHELL_EXECUTION", re.compile(
        r"\b(?:os\.system|subprocess\.(?:run|Popen|call|check_call|check_output)|"
        r"std::process::Command::new|Command::new)\s*\(", re.I)),
    ("DYNAMIC_CODE", re.compile(r"\b(?:eval|exec)\s*\(", re.I)),
    ("POTENTIAL_SECRET", re.compile(
        r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|"
        r"\b(?:API_KEY|SECRET_KEY|ACCESS_TOKEN)\s*=\s*['\"][^'\"]{8,}", re.I)),
)
HEADER = re.compile(rb"^diff --git a/([^\n]+) b/([^\n]+)$")
HUNK = re.compile(rb"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")


def inspect_added_lines(patch: bytes, paths: list[str]) -> list[tuple[str, int, str]]:
    """Extract additions from validated unified diffs without reading deletions as code."""
    found: list[str] = []
    current: str | None = None
    new_line = 0
    in_hunk = False
    added: list[tuple[str, int, str]] = []
    for line in patch.splitlines():
        if line.startswith(b"diff --git "):
            m = HEADER.fullmatch(line)
            if m is None or m[1] != m[2]:
                raise GateError("invalid diff file header for critic")
            current = m[1].decode("utf-8", "strict")
            found.append(current)
            in_hunk = False
            continue
        m = HUNK.match(line)
        if m:
            if current is None:
                raise GateError("hunk without file")
            new_line = int(m[1])
            in_hunk = True
            continue
        if not in_hunk:
            continue
        if line.startswith(b"+"):
            added.append((current, new_line, line[1:].decode("utf-8", "strict")))
            new_line += 1
        elif line.startswith(b" "):
            new_line += 1
        elif line.startswith(b"-") or line == b"\\ No newline at end of file":
            continue
        else:
            in_hunk = False
    if found != paths:
        raise GateError("critic diff files differ from admitted patch")
    return added


def criticize_patch(patch: bytes, paths: list[str]) -> dict[str, object]:
    findings: list[dict[str, str | int]] = []
    for path, line_no, added_line in inspect_added_lines(patch, paths):
        for name, pattern in RULES:
            if pattern.search(added_line):
                findings.append({"path": path, "new_line": line_no, "rule": name})
    return {
        "stage": "0.56",
        "critic_version": 1,
        "patch_sha256": hashlib.sha256(patch).hexdigest(),
        "status": "DENIED_FOR_AUTOMATION" if findings else "SECURITY_REVIEW_REQUIRED",
        "findings": findings,
        "scope": "HEURISTIC_ADDED_LINES_ONLY",
    }


def review_candidate(repo: Path, patch: Path, base: str) -> dict[str, object]:
    admission = inspect_patch(repo, patch, base)
    data = _read_patch(patch)
    critique = criticize_patch(data, admission["paths"])
    if critique["patch_sha256"] != admission["patch_sha256"]:
        raise GateError("candidate patch changed between admission and review")
    critique["base_sha"] = admission["base_sha"]
    critique["changed_lines"] = admission["changed_lines"]
    return critique


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--patch", required=True, type=Path)
    parser.add_argument("--base", required=True)
    args = parser.parse_args()
    try:
        result = review_candidate(args.repo, args.patch, args.base)
        print(json.dumps(result, sort_keys=True))
        return 4 if result["status"] == "SECURITY_REVIEW_REQUIRED" else 3
    except (GateError, OSError, UnicodeError) as exc:
        print(json.dumps({"status": "DENIED", "reason": str(exc)}, sort_keys=True))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
