#!/usr/bin/env python3
"""OCAF OmniDev 0.56: deterministic codegen -> review -> isolated test -> receipt.

This implements a bounded, explicitly approved repair cycle. Never runs changed
code outside Docker; never merges or pushes to protected main.
"""
from __future__ import annotations
import ast
import dataclasses
import difflib
import hashlib
import hmac
import io
import json
import os
import re
import secrets
import sqlite3
import subprocess
import tarfile
import tempfile
from pathlib import Path, PurePosixPath
from omnidev_patch_gate import GateError, inspect_patch, _validate_path
from omnidev_security_critic import criticize_patch

ID = re.compile(r"[A-Za-z0-9_.:-]{1,100}\Z")
SHA = re.compile(r"[0-9a-f]{40}\Z")
LIMIT = 64 * 1024 * 1024


class OmniDevError(ValueError):
    """Fail-closed denial of an unverified code operation."""


@dataclasses.dataclass(frozen=True)
class Repair:
    tenant: str
    task: str
    base: str
    path: str
    old: str
    new: str


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def call(argv: list[str], cwd: Path | None = None, data: bytes | None = None) -> bytes:
    try:
        p = subprocess.run(argv, cwd=cwd, input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=25, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise OmniDevError("process unavailable or timed out") from exc
    if p.returncode != 0:
        raise OmniDevError("process failed: " + p.stderr[:120].decode(errors="replace"))
    return p.stdout


def generate(repo: Path, repair: Repair) -> bytes:
    """Exact anchored edit, syntax validation, real unified Git patch."""
    if (not ID.fullmatch(repair.tenant) or not ID.fullmatch(repair.task)
            or not SHA.fullmatch(repair.base)):
        raise OmniDevError("invalid tenant/task/base")
    _validate_path(repo, repair.path)
    if not repair.path.endswith((".rs", ".py")):
        raise OmniDevError("only Rust/Python source edits are supported")
    if (not repair.old or repair.old == repair.new
            or any("\r" in s or "\x00" in s or len(s.encode()) > 8192
                   for s in (repair.old, repair.new))):
        raise OmniDevError("invalid replacement")
    if call(["git", "-C", str(repo), "rev-parse", "HEAD"]).decode().strip() != repair.base:
        raise OmniDevError("stale base")
    data = call(["git", "-C", str(repo), "show", repair.base + ":" + repair.path])
    if len(data) > 1024 * 1024:
        raise OmniDevError("source exceeds limit")
    try:
        before = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise OmniDevError("invalid UTF-8") from exc
    if before.count(repair.old) != 1:
        raise OmniDevError("edit is ambiguous or missing")
    after = before.replace(repair.old, repair.new, 1)
    if not after.endswith("\n"):
        raise OmniDevError("noncanonical source terminator")
    if repair.path.endswith(".py"):
        try:
            ast.parse(after, filename=repair.path)
        except SyntaxError as exc:
            raise OmniDevError("invalid generated Python syntax") from exc
    else:
        from omnidev_tree_sitter import parse_rust, SyntaxIndexError
        try:
            parse_rust(after.encode())
        except SyntaxIndexError as exc:
            raise OmniDevError("invalid generated Rust CST") from exc
    hunk = "".join(difflib.unified_diff(
        before.splitlines(keepends=True), after.splitlines(keepends=True),
        fromfile="a/" + repair.path, tofile="b/" + repair.path))
    if not hunk:
        raise OmniDevError("no changes")
    return ("diff --git a/" + repair.path + " b/" + repair.path
            + "\n" + hunk).encode()


def _safe_archive(repo: Path, base: str, destination: Path) -> None:
    archive = call(["git", "-C", str(repo), "archive", "--format=tar", base])
    if len(archive) > LIMIT:
        raise OmniDevError("archive too large")
    size = 0
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as tar:
        members = tar.getmembers()
        if len(members) > 20_000:
            raise OmniDevError("too many source files")
        for member in members:
            name = PurePosixPath(member.name)
            if (name.is_absolute() or ".." in name.parts
                    or not (member.isfile() or member.isdir())):
                raise OmniDevError("archive contains unsafe link or path")
            size += member.size
            if size > LIMIT:
                raise OmniDevError("archive expanded beyond limit")
            target = destination.joinpath(*name.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            f = tar.extractfile(member)
            if f is None:
                raise OmniDevError("archived data missing")
            target.write_bytes(f.read(LIMIT + 1))
            target.chmod(0o644)


class DockerExecutor:
    """Fail-closed runtime adapter: no network, privileges or host credentials."""

    def __init__(self, image: str = "python:3.13-slim"):
        if not re.fullmatch(r"[a-zA-Z0-9._/:@-]{3,160}", image):
            raise OmniDevError("invalid image")
        self.image = image

    def arguments(self, workspace: Path, name: str, command: tuple[str, ...]) -> list[str]:
        if (not ID.fullmatch(name) or not 1 <= len(command) <= 16
                or any(not x or len(x) > 256 or "\x00" in x for x in command)):
            raise OmniDevError("invalid command")
        return ["docker", "run", "--rm", "--name", name,
                "--network", "none", "--cap-drop", "ALL",
                "--security-opt", "no-new-privileges",
                "--pids-limit", "128", "--memory", "512m", "--cpus", "1",
                "--read-only", "--tmpfs", "/tmp:rw,noexec,nosuid,size=64m",
                "--user", "65534:65534",
                "--mount", "type=bind,src=" + str(workspace) + ",dst=/workspace,readonly",
                "--workdir", "/workspace", "-e", "PYTHONDONTWRITEBYTECODE=1",
                self.image, *command]

    def run(self, repo: Path, base: str, patch: bytes, command: tuple[str, ...]) -> dict:
        if not SHA.fullmatch(base) or len(patch) > 512 * 1024:
            raise OmniDevError("invalid sandbox source")
        with tempfile.TemporaryDirectory(prefix="ocaf-stage056-") as temp:
            root = Path(temp)
            workspace = root / "workspace"
            workspace.mkdir()
            _safe_archive(repo, base, workspace)
            call(["git", "apply", "--check", "--whitespace=error", "-"],
                 cwd=workspace, data=patch)
            call(["git", "apply", "--whitespace=error", "-"],
                 cwd=workspace, data=patch)
            root.chmod(0o755)
            name = "ocaf-stage056-" + secrets.token_hex(8)
            try:
                p = subprocess.run(self.arguments(workspace, name, command),
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   timeout=120, check=False)
                return {"exit_code": p.returncode, "stdout_sha256": digest(p.stdout),
                        "stdout_excerpt": p.stdout[:8192].decode(errors="replace"),
                        "sandbox": "DOCKER_NO_NETWORK_ROOTLESS_CAPDROP_READONLY"}
            except (OSError, subprocess.TimeoutExpired) as exc:
                raise OmniDevError("sandbox unavailable or uncertain") from exc
            finally:
                try:
                    subprocess.run(["docker", "rm", "-f", name],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   timeout=10, check=False)
                except (OSError, subprocess.TimeoutExpired):
                    pass


class Stage056:
    """SQLite checkpoints and HMAC-chained audit; no automatic retry after RUNNING."""

    def __init__(self, db: Path, operator_key: bytes):
        if len(operator_key) < 32:
            raise OmniDevError("operator key must have >=32 bytes")
        self.key = operator_key
        self.db = sqlite3.connect(db, isolation_level=None, timeout=10)
        self.db.execute("CREATE TABLE IF NOT EXISTS jobs(tenant TEXT, task TEXT, base TEXT,"
                        " patch BLOB, sha TEXT, status TEXT, result TEXT,"
                        " PRIMARY KEY(tenant,task))")
        self.db.execute("CREATE TABLE IF NOT EXISTS events(serial INTEGER PRIMARY KEY,"
                        " tenant TEXT, task TEXT, event TEXT, mac TEXT)")

    def _event(self, tenant: str, task: str, event: str) -> None:
        row = self.db.execute("SELECT mac FROM events ORDER BY serial DESC LIMIT 1").fetchone()
        prev = row[0] if row else "0" * 64
        mac = hmac.new(self.key, canonical([prev, tenant, task, event]), "sha256").hexdigest()
        self.db.execute("INSERT INTO events(tenant,task,event,mac) VALUES(?,?,?,?)",
                        (tenant, task, event, mac))

    def verify(self) -> bool:
        prev = "0" * 64
        for tenant, task, event, mac in self.db.execute(
                "SELECT tenant,task,event,mac FROM events ORDER BY serial"):
            expected = hmac.new(self.key, canonical([prev, tenant, task, event]), "sha256").hexdigest()
            if not hmac.compare_digest(mac, expected):
                return False
            prev = mac
        return True

    def submit(self, repo: Path, repair: Repair) -> dict:
        if not self.verify():
            raise OmniDevError("corrupt audit chain")
        patch = generate(repo, repair)
        with tempfile.TemporaryDirectory() as tmp:
            name = Path(tmp) / "candidate.diff"
            name.write_bytes(patch)
            admission = inspect_patch(repo, name, repair.base)
        critic = criticize_patch(patch, admission["paths"])
        if critic["patch_sha256"] != admission["patch_sha256"] or critic["findings"]:
            raise OmniDevError("security critic rejects candidate")
        self.db.execute("BEGIN IMMEDIATE")
        try:
            prior = self.db.execute("SELECT base,sha,status FROM jobs WHERE tenant=? AND task=?",
                                    (repair.tenant, repair.task)).fetchone()
            if prior:
                if prior[:2] != (repair.base, digest(patch)):
                    raise OmniDevError("idempotency conflict")
                status = prior[2]
            else:
                status = "REVIEW_REQUIRED"
                self.db.execute("INSERT INTO jobs VALUES(?,?,?,?,?,?,NULL)",
                                (repair.tenant, repair.task, repair.base, patch, digest(patch), status))
                self._event(repair.tenant, repair.task, status + ":" + digest(patch))
            self.db.execute("COMMIT")
        except BaseException:
            self.db.execute("ROLLBACK")
            raise
        return {"tenant": repair.tenant, "task": repair.task, "base": repair.base,
                "patch_sha256": digest(patch), "status": status,
                "security": critic["status"], "admission": admission["status"]}

    def token(self, tenant: str, task: str, base: str, sha: str,
              command: tuple[str, ...], image: str) -> str:
        if not ID.fullmatch(tenant) or not ID.fullmatch(task):
            raise OmniDevError("invalid operator token identity")
        return hmac.new(self.key, canonical(
            [tenant, task, base, sha, list(command), image]), "sha256").hexdigest()

    def status(self, tenant: str, task: str) -> dict:
        if not ID.fullmatch(tenant) or not ID.fullmatch(task):
            raise OmniDevError("invalid tenant")
        row = self.db.execute("SELECT status,sha,result FROM jobs WHERE tenant=? AND task=?",
                              (tenant, task)).fetchone()
        if row is None:
            raise OmniDevError("unknown scoped task")
        return {"status": row[0], "patch_sha256": row[1],
                "result": json.loads(row[2]) if row[2] else None}

    def execute(self, repo: Path, tenant: str, task: str, approval: str,
                command: tuple[str, ...], executor: DockerExecutor) -> dict:
        if not self.verify():
            raise OmniDevError("tampered audit chain")
        self.db.execute("BEGIN IMMEDIATE")
        try:
            row = self.db.execute("SELECT base,patch,sha,status FROM jobs "
                                  "WHERE tenant=? AND task=?", (tenant, task)).fetchone()
            if row is None:
                raise OmniDevError("no authorized tenant task")
            base, patch, sha, current = row
            if digest(patch) != sha:
                raise OmniDevError("stored patch fingerprint mismatch")
            valid = self.token(tenant, task, base, sha, command, executor.image)
            if not hmac.compare_digest(approval, valid):
                raise OmniDevError("operator approval mismatch")
            if current == "TESTED_PASS":
                self.db.execute("COMMIT")
                return self.status(tenant, task)
            if current != "REVIEW_REQUIRED":
                raise OmniDevError("no automatic retry for uncertain/failed operation")
            if call(["git", "-C", str(repo), "rev-parse", "HEAD"]).decode().strip() != base:
                raise OmniDevError("Git base changed before execution")
            self.db.execute("UPDATE jobs SET status='RUNNING' WHERE tenant=? AND task=?",
                            (tenant, task))
            self._event(tenant, task, "RUNNING:" + sha)
            self.db.execute("COMMIT")
        except BaseException:
            self.db.execute("ROLLBACK")
            raise
        try:
            result = executor.run(repo, base, patch, command)
            state = "TESTED_PASS" if result["exit_code"] == 0 else "TESTED_FAIL"
        except Exception as exc:
            state = "UNCERTAIN"
            result = {"error": type(exc).__name__, "sandbox": "UNVERIFIED"}
        self.db.execute("BEGIN IMMEDIATE")
        try:
            self.db.execute("UPDATE jobs SET status=?,result=? WHERE tenant=? AND task=? "
                            "AND status='RUNNING'",
                            (state, json.dumps(result, sort_keys=True), tenant, task))
            self._event(tenant, task, state + ":" + digest(canonical(result)))
            self.db.execute("COMMIT")
        except BaseException:
            self.db.execute("ROLLBACK")
            raise
        return self.status(tenant, task)
