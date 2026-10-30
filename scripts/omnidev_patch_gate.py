#!/usr/bin/env python3
"""Stage 0.56: fail-closed, read-only admission for an OmniDev candidate diff.

This gate does NOT run untrusted code, review semantics, sign artifacts, or
authorize a protected-branch write. It validates a candidate against a clean,
exact Git base and a narrowly scoped file policy before external sandbox tests.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
from pathlib import Path

MAX_PATCH_BYTES = 512 * 1024
MAX_FILES = 16
MAX_CHANGED_LINES = 1000
SHA_RE = re.compile(r"[0-9a-f]{40}\Z")
PATH_RE = re.compile(r"[A-Za-z0-9_./-]+\Z")
ALLOWED_PREFIXES = ("src/", "tests/", "scripts/", "docs/")
REJECTED_MARKERS = (b"GIT binary patch", b"Binary files ", b"rename from ", b"rename to ",
                    b"copy from ", b"copy to ", b"Subproject commit ")


class GateError(ValueError):
    """A candidate fails the pre-execution policy."""


def _git(repo: Path, args: list[str], data: bytes | None = None) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *args], input=data, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, timeout=10, check=False,
            env={**os.environ, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
                 "GIT_NO_REPLACE_OBJECTS": "1", "GIT_OPTIONAL_LOCKS": "0"},
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise GateError("git unavailable or timed out") from exc
    if result.returncode:
        raise GateError("git validation failed: " + result.stderr.decode("utf-8", "replace")[:300])
    return result.stdout


def _read_patch(path: Path) -> bytes:
    try:
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_PATCH_BYTES:
                raise GateError("invalid patch file type or size")
            with os.fdopen(fd, "rb", closefd=False) as stream:
                data = stream.read(MAX_PATCH_BYTES + 1)
        finally:
            os.close(fd)
    except OSError as exc:
        raise GateError("cannot open regular patch file") from exc
    if not data or len(data) > MAX_PATCH_BYTES:
        raise GateError("empty patch or patch size exceeds limit")
    if b"\x00" in data or b"\r" in data:
        raise GateError("binary or noncanonical patch encoding")
    try:
        data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise GateError("patch must be UTF-8") from exc
    return data


def _validate_path(repo: Path, name: str) -> None:
    parts = name.split("/")
    if (not PATH_RE.fullmatch(name) or name.startswith("/") or "//" in name
            or any(p in ("", ".", "..") or p.startswith(".") for p in parts)
            or not name.startswith(ALLOWED_PREFIXES)):
        raise GateError("patch path is outside approved source roots")
    candidate = repo
    for part in parts:
        candidate = candidate / part
        if candidate.is_symlink():
            raise GateError("patch path traverses a symlink")


def inspect_patch(repo: Path, patch: Path, base_sha: str) -> dict[str, object]:
    if not SHA_RE.fullmatch(base_sha):
        raise GateError("invalid base SHA")
    repo = repo.resolve(strict=True)
    top = Path(os.fsdecode(_git(repo, ["rev-parse", "--show-toplevel"]).strip())).resolve()
    if top != repo:
        raise GateError("repo must be the exact Git worktree root")
    actual = _git(repo, ["rev-parse", "HEAD"]).decode().strip()
    if actual != base_sha:
        raise GateError("base SHA differs from current HEAD")
    if _git(repo, ["status", "--porcelain", "--untracked-files=no"]):
        raise GateError("tracked worktree or index is dirty")
    data = _read_patch(patch)
    if any(marker in data for marker in REJECTED_MARKERS):
        raise GateError("binary, rename, copy or submodule patch forbidden")
    for line in data.splitlines():
        if line.startswith((b"old mode ", b"new mode ", b"deleted file mode ")):
            raise GateError("file mode change forbidden")
        if line.startswith(b"new file mode ") and line != b"new file mode 100644":
            raise GateError("new executable or symlink file forbidden")
    stats = _git(repo, ["apply", "--numstat", "-z", "-"], data)
    paths: list[str] = []
    total = 0
    for entry in stats.split(b"\x00"):
        if not entry:
            continue
        fields = entry.split(b"\t", 2)
        if len(fields) != 3 or not all(f.isdigit() for f in fields[:2]):
            raise GateError("unsupported diff stat or binary patch")
        name = fields[2].decode("utf-8", "strict")
        _validate_path(repo, name)
        if name in paths:
            raise GateError("duplicate patch path")
        paths.append(name)
        total += int(fields[0]) + int(fields[1])
    headers = sum(1 for line in data.splitlines() if line.startswith(b"diff --git "))
    if not paths or len(paths) > MAX_FILES or headers != len(paths):
        raise GateError("empty, malformed or too many file changes")
    if total > MAX_CHANGED_LINES:
        raise GateError("changed-line budget exceeded")
    _git(repo, ["apply", "--check", "--cached", "--whitespace=error", "-"], data)
    _git(repo, ["apply", "--check", "--whitespace=error", "-"], data)
    return {"status": "ADMISSIBLE_FOR_SANDBOX_TESTS", "base_sha": base_sha,
            "patch_sha256": hashlib.sha256(data).hexdigest(), "paths": paths,
            "changed_lines": total}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--patch", type=Path, required=True)
    parser.add_argument("--base", required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(inspect_patch(args.repo, args.patch, args.base), sort_keys=True))
        return 0
    except (GateError, OSError) as exc:
        print(json.dumps({"status": "DENIED", "reason": str(exc)}, sort_keys=True))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
