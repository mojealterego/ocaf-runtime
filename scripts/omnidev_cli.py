#!/usr/bin/env python3
"""0.56 OmniDev command-line entry: operator-gated simulation only.

Pass key through OCAF_OMNIDEV_OPERATOR_KEY_HEX (>=64 hex chars).
Never prints the key; 'approve' emits a scoped HMAC after explicit patch review.
"""
from __future__ import annotations
import argparse
import dataclasses
import json
import os
import sys
from pathlib import Path
from omnidev_pipeline import DockerExecutor, OmniDevError, Repair, Stage056


def _command(value: str) -> tuple[str, ...]:
    try:
        parts = json.loads(value)
    except json.JSONDecodeError as exc:
        raise OmniDevError("command must be a JSON string array") from exc
    if (not isinstance(parts, list) or not 1 <= len(parts) <= 16
            or any(not isinstance(x, str) or not x or len(x) > 256 for x in parts)):
        raise OmniDevError("invalid command array")
    return tuple(parts)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    sub = parser.add_subparsers(dest="action", required=True)
    submit = sub.add_parser("submit")
    submit.add_argument("--repair-json", type=Path, required=True)
    for action in ("status", "approve", "execute", "reconcile"):
        cmd = sub.add_parser(action)
        cmd.add_argument("--tenant", required=True)
        cmd.add_argument("--task", required=True)
        if action in ("approve", "execute"):
            cmd.add_argument("--command-json", required=True)
            cmd.add_argument("--image", default="python:3.13-slim")
        if action == "approve":
            cmd.add_argument("--reviewed-sha", required=True)
        if action in ("execute", "reconcile"):
            cmd.add_argument("--approval", required=True)
        if action == "reconcile":
            cmd.add_argument("--evidence-sha", required=True)
            cmd.add_argument("--decision", choices=[
                "NO_EFFECT_DECLARED", "EFFECT_DECLARED", "UNKNOWN"], required=True)
    args = parser.parse_args(argv)
    try:
        key_hex = os.environ.get("OCAF_OMNIDEV_OPERATOR_KEY_HEX", "")
        key = bytes.fromhex(key_hex)
        journal = Stage056(args.db, key)
        try:
            if args.action == "submit":
                with args.repair_json.open("rb") as inp:
                    body = inp.read(65537)
                if len(body) > 65536:
                    raise OmniDevError("repair JSON exceeds size limit")
                obj = json.loads(body)
                if not isinstance(obj, dict) or set(obj) != set(
                        f.name for f in dataclasses.fields(Repair)):
                    raise OmniDevError("repair JSON has unknown or missing fields")
                result = journal.submit(args.repo, Repair(**obj))
            elif args.action == "status":
                result = journal.status(args.tenant, args.task)
            elif args.action == "reconcile":
                result = journal.reconcile(args.tenant, args.task, args.evidence_sha,
                                           args.decision, args.approval)
            elif args.action == "approve":
                status = journal.status(args.tenant, args.task)
                if status["status"] != "REVIEW_REQUIRED":
                    raise OmniDevError("not pending operator approval")
                if status["patch_sha256"] != args.reviewed_sha:
                    raise OmniDevError("operator did not approve the stored patch SHA")
                row = journal.db.execute(
                    "SELECT base FROM jobs WHERE tenant=? AND task=?",
                    (args.tenant, args.task)).fetchone()
                result = {"approval": journal.token(
                    args.tenant, args.task, row[0], args.reviewed_sha,
                    _command(args.command_json), args.image),
                    "patch_sha256": args.reviewed_sha}
            else:
                result = journal.execute(args.repo, args.tenant, args.task,
                                         args.approval, _command(args.command_json),
                                         DockerExecutor(args.image))
            print(json.dumps(result, sort_keys=True))
            return 0
        finally:
            journal.db.close()
    except (OmniDevError, OSError, ValueError, TypeError, KeyError,
            json.JSONDecodeError) as exc:
        print(json.dumps({"status": "DENIED", "reason": str(exc)}, sort_keys=True))
        return 2


if __name__ == "__main__":
    sys.exit(main())
