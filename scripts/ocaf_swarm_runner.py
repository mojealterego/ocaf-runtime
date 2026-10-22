#!/usr/bin/env python3
"""Durable local stage runner for the OCAF engineering swarm.

Make is an optional control plane. This runner is the execution gate: a group
must hold a lease, execute a real worker command, and provide commit/CI/review
evidence before a stage can become COMPLETE_VERIFIED.

The state database is SQLite so the runner is usable on a clean checkout. The
same state transitions can later be backed by PostgreSQL without changing the
worker contract. No shell is used for worker commands.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import secrets
import shlex
import sqlite3
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


ACTOR_RE = re.compile(r"^[A-Za-z0-9_-]{1,64}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
MAX_OUTPUT = 2_000_000
GROUP_ID_RE = re.compile(r"^G(?:[1-9]|[12][0-9]|3[0-3])$")
DEFAULT_ROSTER = Path(__file__).resolve().parents[1] / "docs/agents/swarm_groups_v1.json"


class SwarmError(RuntimeError):
    pass


def now_ms() -> int:
    return int(time.time() * 1000)


def validate_actor(value: str, label: str) -> str:
    if not ACTOR_RE.fullmatch(value):
        raise SwarmError(f"invalid {label}: use 1-64 ASCII letters, digits, _ or -")
    return value


def connect(path: Path) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    db = sqlite3.connect(path, timeout=30, isolation_level=None)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA journal_mode=WAL")
    db.execute("PRAGMA busy_timeout=30000")
    return db


def load_roster(path: Path = DEFAULT_ROSTER) -> list[dict[str, str]]:
    """Load and validate the fixed 33-group engineering roster."""
    raw = json.loads(path.read_text(encoding="utf-8"))
    if raw.get("version") != 1 or not isinstance(raw.get("groups"), list):
        raise SwarmError("roster must contain version 1 and a groups array")
    groups = raw["groups"]
    if len(groups) != 33:
        raise SwarmError(f"roster must contain exactly 33 groups, got {len(groups)}")
    ids: set[str] = set()
    specializations: set[str] = set()
    validated: list[dict[str, str]] = []
    for group in groups:
        if not isinstance(group, dict):
            raise SwarmError("roster contains a non-object group")
        required = ("id", "specialization", "scope", "handoff")
        if any(not isinstance(group.get(key), str) or not group[key].strip() for key in required):
            raise SwarmError("every group needs non-empty id, specialization, scope and handoff")
        group_id = group["id"]
        specialization = group["specialization"]
        if not GROUP_ID_RE.fullmatch(group_id):
            raise SwarmError(f"invalid roster group id: {group_id}")
        if group_id in ids:
            raise SwarmError(f"duplicate roster group id: {group_id}")
        if specialization in specializations:
            raise SwarmError(f"duplicate roster specialization: {specialization}")
        ids.add(group_id)
        specializations.add(specialization)
        validated.append({key: group[key] for key in required})
    expected = {f"G{i}" for i in range(1, 34)}
    if ids != expected:
        raise SwarmError("roster ids must be exactly G1 through G33")
    return validated


def seed_roster(db: sqlite3.Connection, roster: list[dict[str, str]]) -> None:
    db.execute(
        """CREATE TABLE IF NOT EXISTS swarm_groups (
           group_id TEXT PRIMARY KEY,
           specialization TEXT NOT NULL UNIQUE,
           scope TEXT NOT NULL,
           handoff TEXT NOT NULL
        )"""
    )
    for group in roster:
        db.execute(
            """INSERT INTO swarm_groups(group_id,specialization,scope,handoff)
               VALUES(?,?,?,?)
               ON CONFLICT(group_id) DO UPDATE SET specialization=excluded.specialization,
                 scope=excluded.scope,handoff=excluded.handoff""",
            (group["id"], group["specialization"], group["scope"], group["handoff"]),
        )


def roster_group(db: sqlite3.Connection, group_id: str) -> sqlite3.Row:
    row = db.execute("SELECT * FROM swarm_groups WHERE group_id=?", (group_id,)).fetchone()
    if row is None:
        raise SwarmError(f"unknown group: {group_id}; use one of G1..G33")
    return row


def init_db(db: sqlite3.Connection, roster_path: Path = DEFAULT_ROSTER) -> None:
    db.executescript(
        """
        CREATE TABLE IF NOT EXISTS stages (
            id TEXT PRIMARY KEY,
            ordinal INTEGER NOT NULL,
            title TEXT NOT NULL,
            acceptance TEXT NOT NULL,
            negative TEXT NOT NULL,
            dependencies TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN
              ('READY','ASSIGNED','OPEN','COMPLETE_VERIFIED','BLOCKED')),
            assigned_group TEXT,
            specialization TEXT,
            lease_token TEXT,
            lease_until_ms INTEGER,
            attempt INTEGER NOT NULL DEFAULT 0,
            base_sha TEXT,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS stages_claim_order
          ON stages(status, ordinal, lease_until_ms);
        CREATE TABLE IF NOT EXISTS evidence (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stage_id TEXT NOT NULL REFERENCES stages(id),
            group_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            output_sha256 TEXT NOT NULL,
            output_json TEXT NOT NULL,
            stdout TEXT NOT NULL,
            stderr TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            stage_id TEXT,
            group_id TEXT,
            event TEXT NOT NULL,
            detail TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );
        """
    )
    seed_roster(db, load_roster(roster_path))


def load_manifest(db: sqlite3.Connection, manifest: Path, ready_ids: set[str]) -> int:
    raw = json.loads(manifest.read_text(encoding="utf-8"))
    rows = raw.get("main")
    if not isinstance(rows, list):
        raise SwarmError("manifest must contain a main array")
    inserted = 0
    for ordinal, row in enumerate(rows):
        if not isinstance(row, dict) or not isinstance(row.get("id"), str):
            raise SwarmError("manifest contains an invalid stage row")
        stage_id = row["id"]
        deps = row.get("dependencies", [])
        if not isinstance(deps, list) or any(not isinstance(x, str) for x in deps):
            raise SwarmError(f"invalid dependencies for {stage_id}")
        # The manifest is planning input, not authorization to execute. Only
        # explicitly promoted ids start READY; every other stage is BLOCKED
        # until its predecessor is verified.
        status = "READY" if stage_id in ready_ids else "BLOCKED"
        db.execute(
            """INSERT INTO stages
               (id,ordinal,title,acceptance,negative,dependencies,status,updated_at_ms)
               VALUES(?,?,?,?,?,?,?,?)
               ON CONFLICT(id) DO UPDATE SET
                 ordinal=excluded.ordinal,title=excluded.title,
                 acceptance=excluded.acceptance,negative=excluded.negative,
                 dependencies=excluded.dependencies""",
            (
                stage_id,
                ordinal,
                str(row.get("title", "")),
                str(row.get("acceptance", "")),
                str(row.get("negative", "")),
                json.dumps(deps, separators=(",", ":")),
                status,
                now_ms(),
            ),
        )
        inserted += 1
    return inserted


def _dependencies_complete(db: sqlite3.Connection, deps: list[str]) -> bool:
    if not deps:
        return True
    marks = ",".join("?" for _ in deps)
    found = db.execute(
        f"SELECT id FROM stages WHERE id IN ({marks}) AND status='COMPLETE_VERIFIED'", deps
    ).fetchall()
    return len(found) == len(set(deps))


def _expire_leases(db: sqlite3.Connection, at_ms: int) -> None:
    rows = db.execute(
        "SELECT id,assigned_group FROM stages WHERE status='ASSIGNED' AND lease_until_ms <= ?",
        (at_ms,),
    ).fetchall()
    for row in rows:
        db.execute(
            """UPDATE stages SET status='READY',assigned_group=NULL,specialization=NULL,
               lease_token=NULL,lease_until_ms=NULL,base_sha=NULL,updated_at_ms=?
               WHERE id=? AND status='ASSIGNED'""",
            (at_ms, row["id"]),
        )
        db.execute(
            "INSERT INTO events(stage_id,group_id,event,detail,created_at_ms) VALUES(?,?,?,?,?)",
            (row["id"], row["assigned_group"], "lease_expired", "returned to READY", at_ms),
        )


def _promote_eligible(db: sqlite3.Connection, at_ms: int) -> None:
    """Make only dependency-satisfied stages claimable after a verified handoff."""
    rows = db.execute("SELECT id,dependencies FROM stages WHERE status='BLOCKED'").fetchall()
    for row in rows:
        if _dependencies_complete(db, json.loads(row["dependencies"])):
            db.execute(
                "UPDATE stages SET status='READY',updated_at_ms=? WHERE id=? AND status='BLOCKED'",
                (at_ms, row["id"]),
            )
            db.execute(
                "INSERT INTO events(stage_id,event,detail,created_at_ms) VALUES(?,?,?,?)",
                (row["id"], "promoted", "dependencies verified", at_ms),
            )


HISTORICAL_UNDEFINED_TITLE = "Historyczny etap bez zachowanej definicji"


def _next_sequential_ready(db: sqlite3.Connection) -> sqlite3.Row | None:
    """Return the first active stage in ordinal order.

    Historical rows without a preserved definition are excluded from the
    execution queue.  A later READY stage must never jump over an earlier
    READY or OPEN stage, even when the later row has no dependencies.
    """
    rows = db.execute(
        "SELECT * FROM stages WHERE status IN ('READY','OPEN') ORDER BY ordinal"
    ).fetchall()
    for row in rows:
        if row["title"] == HISTORICAL_UNDEFINED_TITLE:
            continue
        return row
    return None


def claim_next(
    db: sqlite3.Connection,
    group: str,
    specialization: str,
    base_sha: str,
    stage_id: str | None = None,
    lease_ms: int = 900_000,
) -> dict[str, Any] | None:
    validate_actor(group, "group")
    if not specialization.strip() or not SHA_RE.fullmatch(base_sha):
        raise SwarmError("specialization and a 40-character base SHA are required")
    if not 10_000 <= lease_ms <= 86_400_000:
        raise SwarmError("lease must be between 10 seconds and 24 hours")
    at_ms = now_ms()
    db.execute("BEGIN IMMEDIATE")
    try:
        group_row = roster_group(db, group)
        _expire_leases(db, at_ms)
        active = db.execute(
            "SELECT id FROM stages WHERE status='ASSIGNED' AND assigned_group=? LIMIT 1",
            (group,),
        ).fetchone()
        if active is not None:
            db.execute("COMMIT")
            return None
        query = """SELECT * FROM stages
                   WHERE (status='READY' OR (status='OPEN' AND assigned_group=?))"""
        params: list[Any] = [group]
        if stage_id is not None:
            if not stage_id.strip():
                raise SwarmError("stage_id cannot be empty")
            query += " AND id=?"
            params.append(stage_id)
        query += " ORDER BY CASE WHEN status='OPEN' THEN 0 ELSE 1 END, ordinal"
        candidates = db.execute(query, params).fetchall()
        selected = None
        # An OPEN lease is a resume of work already owned by this group.
        queue_head = _next_sequential_ready(db)
        if queue_head is not None and queue_head["status"] == "OPEN":
            # Only the owning group may resume an OPEN stage.  While another
            # group owns it, the strict queue is blocked at this ordinal.
            if queue_head["assigned_group"] == group:
                selected = queue_head
        elif queue_head is not None:
            selected = queue_head
            deps = json.loads(selected["dependencies"])
            if not _dependencies_complete(db, deps):
                selected = None
        if stage_id is not None and selected is not None and selected["id"] != stage_id:
            raise SwarmError(
                f"strict ordinal queue requires stage {selected['id']} before {stage_id}"
            )
        if stage_id is not None and selected is None:
            requested = db.execute("SELECT * FROM stages WHERE id=?", (stage_id,)).fetchone()
            if requested is not None and requested["status"] == "READY":
                raise SwarmError("requested stage is not the next sequential READY stage")
        if selected is None:
            db.execute("COMMIT")
            return None
        token = secrets.token_urlsafe(32)
        attempt = int(selected["attempt"]) + 1
        updated = db.execute(
            """UPDATE stages SET status='ASSIGNED',assigned_group=?,specialization=?,
               lease_token=?,lease_until_ms=?,attempt=?,base_sha=?,updated_at_ms=?
               WHERE id=? AND (status='READY' OR (status='OPEN' AND assigned_group=?))""",
            (
                group,
                group_row["specialization"],
                token,
                at_ms + lease_ms,
                attempt,
                base_sha,
                at_ms,
                selected["id"],
                group,
            ),
        ).rowcount
        if updated != 1:
            raise SwarmError("stage claim lost a concurrent race")
        db.execute(
            "INSERT INTO events(stage_id,group_id,event,detail,created_at_ms) VALUES(?,?,?,?,?)",
            (selected["id"], group, "claimed", f"attempt={attempt}", at_ms),
        )
        db.execute("COMMIT")
        return {
            "stage_id": selected["id"],
            "title": selected["title"],
            "acceptance": selected["acceptance"],
            "negative": selected["negative"],
            "dependencies": json.loads(selected["dependencies"]),
            "group": group,
            "specialization": group_row["specialization"],
            "attempt": attempt,
            "lease_token": token,
            "base_sha": base_sha,
            "lease_until_ms": at_ms + lease_ms,
        }
    except Exception:
        db.execute("ROLLBACK")
        raise


def valid_completion(payload: Any) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "worker output must be a JSON object"
    if payload.get("status") != "COMPLETE_VERIFIED":
        return False, "worker did not report COMPLETE_VERIFIED"
    evidence = payload.get("evidence")
    if not isinstance(evidence, dict):
        return False, "worker evidence object is missing"
    commit = evidence.get("commit_sha")
    if not isinstance(commit, str) or not SHA_RE.fullmatch(commit):
        return False, "evidence.commit_sha must be a 40-character Git SHA"
    ci_runs = evidence.get("ci_runs")
    if not isinstance(ci_runs, list) or not ci_runs:
        return False, "evidence.ci_runs must contain at least one run"
    for run in ci_runs:
        if isinstance(run, str):
            if not run.strip():
                return False, "evidence.ci_runs contains an empty run"
        elif isinstance(run, dict):
            run_id = run.get("run_id") or run.get("url")
            if not isinstance(run_id, str) or not run_id.strip():
                return False, "each CI run needs run_id or url"
            if "commit_sha" in run and run["commit_sha"] != commit:
                return False, "CI run commit_sha does not match evidence.commit_sha"
        else:
            return False, "evidence.ci_runs entries must be strings or objects"
    if not isinstance(evidence.get("negative_tests"), list) or not evidence["negative_tests"]:
        return False, "evidence.negative_tests must contain at least one test"
    if not isinstance(evidence.get("independent_review"), str) or not evidence["independent_review"].strip():
        return False, "evidence.independent_review is missing"
    return True, "verified evidence accepted"


def record_worker_result(
    db: sqlite3.Connection,
    claim: dict[str, Any],
    stdout: str,
    stderr: str,
    payload: Any,
    exit_code: int,
) -> dict[str, Any]:
    stdout = stdout[:MAX_OUTPUT]
    stderr = stderr[:MAX_OUTPUT]
    raw = json.dumps(payload, sort_keys=True, separators=(",", ":")) if payload is not None else "null"
    digest = hashlib.sha256((stdout + "\n" + stderr + "\n" + raw).encode()).hexdigest()
    verified, reason = valid_completion(payload)
    status = "COMPLETE_VERIFIED" if verified and exit_code == 0 else "OPEN"
    at_ms = now_ms()
    db.execute("BEGIN IMMEDIATE")
    try:
        row = db.execute(
            "SELECT * FROM stages WHERE id=? AND status='ASSIGNED' AND assigned_group=? AND lease_token=?",
            (claim["stage_id"], claim["group"], claim["lease_token"]),
        ).fetchone()
        if row is None:
            raise SwarmError("lease is no longer valid; refusing to record worker result")
        db.execute(
            """INSERT INTO evidence(stage_id,group_id,attempt,output_sha256,output_json,stdout,stderr,created_at_ms)
               VALUES(?,?,?,?,?,?,?,?)""",
            (claim["stage_id"], claim["group"], claim["attempt"], digest, raw, stdout, stderr, at_ms),
        )
        detail = reason if status == "OPEN" else "verified commit, CI, negative tests and review"
        if exit_code != 0:
            detail = f"worker exit code {exit_code}; {detail}"
        db.execute(
            """UPDATE stages SET status=?,lease_token=NULL,lease_until_ms=NULL,updated_at_ms=?
               WHERE id=? AND status='ASSIGNED' AND lease_token=?""",
            (status, at_ms, claim["stage_id"], claim["lease_token"]),
        )
        db.execute(
            "INSERT INTO events(stage_id,group_id,event,detail,created_at_ms) VALUES(?,?,?,?,?)",
            (claim["stage_id"], claim["group"], "completed" if status == "COMPLETE_VERIFIED" else "opened", detail, at_ms),
        )
        if status == "COMPLETE_VERIFIED":
            _promote_eligible(db, at_ms)
        db.execute("COMMIT")
    except Exception:
        db.execute("ROLLBACK")
        raise
    return {"stage_id": claim["stage_id"], "status": status, "reason": reason, "evidence_sha256": digest}


def run_worker(db: sqlite3.Connection, claim: dict[str, Any], command: str) -> dict[str, Any]:
    argv = shlex.split(command)
    if not argv:
        raise SwarmError("worker command is empty")
    env = os.environ.copy()
    for key, value in {
        "OCAF_GROUP": claim["group"],
        "OCAF_SPECIALIZATION": claim["specialization"],
        "OCAF_STAGE_ID": claim["stage_id"],
        "OCAF_STAGE_TITLE": claim["title"],
        "OCAF_STAGE_ACCEPTANCE": claim["acceptance"],
        "OCAF_STAGE_NEGATIVE": claim["negative"],
        "OCAF_BASE_SHA": claim["base_sha"],
        "OCAF_LEASE_TOKEN": claim["lease_token"],
    }.items():
        env[key] = value
    try:
        proc = subprocess.run(argv, capture_output=True, text=True, env=env, timeout=3_600, check=False)
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else (exc.stdout or b"").decode(errors="replace")
        stderr = exc.stderr if isinstance(exc.stderr, str) else (exc.stderr or b"").decode(errors="replace")
        return record_worker_result(db, claim, stdout, stderr + "\nworker timeout", None, 124)
    try:
        payload = json.loads(proc.stdout) if proc.stdout.strip() else None
    except json.JSONDecodeError:
        payload = None
    return record_worker_result(db, claim, proc.stdout, proc.stderr, payload, proc.returncode)


def load_claim(
    db: sqlite3.Connection, group: str, stage_id: str, lease_token: str
) -> dict[str, Any]:
    """Load an existing lease for an external worker handoff."""
    validate_actor(group, "group")
    if not stage_id.strip() or not lease_token.strip():
        raise SwarmError("stage_id and lease_token are required")
    row = db.execute(
        """SELECT s.*, g.specialization AS roster_specialization
           FROM stages s JOIN swarm_groups g ON g.group_id=s.assigned_group
           WHERE s.id=? AND s.assigned_group=? AND s.lease_token=?
             AND s.status='ASSIGNED'""",
        (stage_id, group, lease_token),
    ).fetchone()
    if row is None:
        raise SwarmError("active lease not found for group, stage and token")
    return {
        "stage_id": row["id"],
        "title": row["title"],
        "acceptance": row["acceptance"],
        "negative": row["negative"],
        "dependencies": json.loads(row["dependencies"]),
        "group": row["assigned_group"],
        "specialization": row["roster_specialization"],
        "attempt": row["attempt"],
        "lease_token": row["lease_token"],
        "base_sha": row["base_sha"],
        "lease_until_ms": row["lease_until_ms"],
    }


def complete_external(
    db: sqlite3.Connection,
    group: str,
    stage_id: str,
    lease_token: str,
    result_path: Path,
    stdout_path: Path | None,
    stderr_path: Path | None,
    exit_code: int,
) -> dict[str, Any]:
    claim = load_claim(db, group, stage_id, lease_token)
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    stdout = stdout_path.read_text(encoding="utf-8") if stdout_path else json.dumps(payload)
    stderr = stderr_path.read_text(encoding="utf-8") if stderr_path else ""
    return record_worker_result(db, claim, stdout, stderr, payload, exit_code)


def status(db: sqlite3.Connection) -> list[dict[str, Any]]:
    return [dict(row) for row in db.execute("SELECT * FROM stages ORDER BY ordinal")]


def group_status(db: sqlite3.Connection) -> list[dict[str, Any]]:
    """Return all 33 groups, including idle groups with no current lease."""
    rows = db.execute(
        "SELECT * FROM swarm_groups ORDER BY CAST(SUBSTR(group_id, 2) AS INTEGER)"
    ).fetchall()
    result = []
    for group in rows:
        stage = db.execute(
            """SELECT id,title,status,attempt,lease_until_ms FROM stages
               WHERE assigned_group=? AND status IN ('ASSIGNED','OPEN')
               ORDER BY CASE status WHEN 'ASSIGNED' THEN 0 ELSE 1 END, ordinal LIMIT 1""",
            (group["group_id"],),
        ).fetchone()
        result.append({
            "group_id": group["group_id"],
            "specialization": group["specialization"],
            "scope": group["scope"],
            "handoff": group["handoff"],
            "status": "WORKING" if stage and stage["status"] == "ASSIGNED" else (
                "OPEN" if stage else "IDLE"
            ),
            "stage_id": stage["id"] if stage else None,
            "stage_title": stage["title"] if stage else None,
            "attempt": stage["attempt"] if stage else None,
            "lease_until_ms": stage["lease_until_ms"] if stage else None,
        })
    return result


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", type=Path, default=Path(".ocaf/swarm.sqlite"))
    sub = p.add_subparsers(dest="command", required=True)
    init = sub.add_parser("init")
    init.add_argument("--manifest", type=Path, required=True)
    init.add_argument("--ready-id", action="append", default=[])
    init.add_argument("--roster", type=Path, default=DEFAULT_ROSTER)
    claim = sub.add_parser("claim")
    claim.add_argument("--group", required=True)
    claim.add_argument("--specialization", required=True)
    claim.add_argument("--base-sha", required=True)
    claim.add_argument("--stage-id")
    run = sub.add_parser("run")
    run.add_argument("--group", required=True)
    run.add_argument("--specialization", required=True)
    run.add_argument("--base-sha", required=True)
    run.add_argument("--stage-id")
    run.add_argument("--worker-cmd", required=True)
    complete = sub.add_parser("complete")
    complete.add_argument("--group", required=True)
    complete.add_argument("--stage-id", required=True)
    complete.add_argument("--lease-token", required=True)
    complete.add_argument("--result-file", type=Path, required=True)
    complete.add_argument("--stdout-file", type=Path)
    complete.add_argument("--stderr-file", type=Path)
    complete.add_argument("--exit-code", type=int, default=0)
    sub.add_parser("status")
    groups = sub.add_parser("groups")
    groups.add_argument("--roster", type=Path, default=DEFAULT_ROSTER)
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    db = connect(args.db)
    init_db(db, getattr(args, "roster", DEFAULT_ROSTER))
    if args.command == "init":
        count = load_manifest(db, args.manifest, set(args.ready_id))
        print(json.dumps({"stages_loaded": count, "db": str(args.db)}))
        return 0
    if args.command == "status":
        print(json.dumps(status(db), indent=2))
        return 0
    if args.command == "groups":
        print(json.dumps(group_status(db), indent=2))
        return 0
    if args.command == "complete":
        result = complete_external(
            db,
            args.group,
            args.stage_id,
            args.lease_token,
            args.result_file,
            args.stdout_file,
            args.stderr_file,
            args.exit_code,
        )
        print(json.dumps(result))
        return 0 if result["status"] == "COMPLETE_VERIFIED" else 2
    claim = claim_next(db, args.group, args.specialization, args.base_sha, args.stage_id)
    if claim is None:
        print(json.dumps({"status": "WAITING_NO_ELIGIBLE_STAGE", "group": args.group}))
        return 0
    if args.command == "claim":
        print(json.dumps(claim, indent=2))
        return 0
    result = run_worker(db, claim, args.worker_cmd)
    print(json.dumps(result))
    return 0 if result["status"] == "COMPLETE_VERIFIED" else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, SwarmError, sqlite3.Error) as exc:
        print(f"ocaf-swarm-runner: {exc}", file=sys.stderr)
        raise SystemExit(1)
