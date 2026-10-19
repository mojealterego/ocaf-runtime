#!/usr/bin/env python3
"""Validate a GitHub-dispatched engineering mission before spending API credits."""

import argparse
import json
import os
from pathlib import PurePosixPath
import re
import sys


ID = re.compile(r"^[a-z0-9][a-z0-9_-]{2,47}$")
SHA = re.compile(r"^[0-9a-f]{40}$")
MAX_INPUT_BYTES = 16_384
PROTECTED = (
    ".gitmodules",
    ".github/workflows",
    ".github/scripts",
    "scripts/engineering_factory_runner.py",
)


def scope(value):
    if not isinstance(value, str) or not value or len(value) > 200:
        raise ValueError("write_set entries must be nonempty paths of at most 200 characters")
    if (
        value.startswith("/")
        or "\\" in value
        or any(ord(character) < 32 or ord(character) == 127 for character in value)
        or any(part in ("", ".", "..") for part in value.rstrip("/").split("/"))
    ):
        raise ValueError("write_set contains an unsafe path")
    path = PurePosixPath(value)
    if ".git" in path.parts or any(overlaps(value, protected) for protected in PROTECTED):
        raise ValueError("write_set includes a protected factory or Git path")
    return value


def within(path, parent):
    parent = parent.rstrip("/")
    return path == parent or path.startswith(parent + "/")


def overlaps(a, b):
    return within(a.rstrip("/"), b) or within(b.rstrip("/"), a)


def nonempty_text(value, field, limit):
    if not isinstance(value, str) or not value.strip() or len(value) > limit:
        raise ValueError(f"{field} must be nonempty text of at most {limit} characters")
    return value.strip()


def validate(raw, base_sha, repository):
    if len(raw.encode("utf-8")) > MAX_INPUT_BYTES:
        raise ValueError("mission JSON exceeds 16 KiB")
    data = json.loads(raw)
    if not isinstance(data, dict):
        raise ValueError("mission must be a JSON object")
    if not isinstance(data.get("mission_id"), str) or not ID.fullmatch(data["mission_id"]):
        raise ValueError("mission_id must be 3-48 lowercase letters, digits, '_' or '-'")
    if not SHA.fullmatch(base_sha):
        raise ValueError("workflow base SHA is invalid")
    if not isinstance(repository, str) or repository.count("/") != 1:
        raise ValueError("workflow repository is invalid")
    if data.get("base_sha") not in (None, base_sha):
        raise ValueError("mission base_sha does not match the dispatched commit")
    if data.get("repo") not in (None, repository):
        raise ValueError("mission repo does not match the workflow repository")
    data["base_sha"] = base_sha
    data["repo"] = repository
    nonempty_text(data.get("objective"), "objective", 4000)
    acceptance = data.get("acceptance")
    if not isinstance(acceptance, list) or not (1 <= len(acceptance) <= 12):
        raise ValueError("acceptance must contain 1-12 criteria")
    for item in acceptance:
        nonempty_text(item, "acceptance item", 1000)
    workers = data.get("workers")
    if not isinstance(workers, list) or not (2 <= len(workers) <= 4):
        raise ValueError("workers must contain 2-4 independent implementers")
    worker_ids, all_scopes = set(), []
    for worker in workers:
        if not isinstance(worker, dict) or not isinstance(worker.get("id"), str) or not ID.fullmatch(worker["id"]):
            raise ValueError("each worker needs a valid id")
        if worker["id"] in worker_ids:
            raise ValueError("worker ids must be unique")
        worker_ids.add(worker["id"])
        nonempty_text(worker.get("role"), "worker role", 80)
        nonempty_text(worker.get("instructions"), "worker instructions", 4000)
        write_set = worker.get("write_set")
        if not isinstance(write_set, list) or not (1 <= len(write_set) <= 20):
            raise ValueError("each worker needs 1-20 write_set paths")
        for value in write_set:
            value = scope(value)
            for previous in all_scopes:
                if overlaps(value, previous):
                    raise ValueError(f"overlapping write scopes: {value!r}, {previous!r}")
            all_scopes.append(value)
    commands = data.get("verify_commands", [])
    if not isinstance(commands, list) or not (1 <= len(commands) <= 8):
        raise ValueError("verify_commands must contain 1-8 commands")
    for command in commands:
        if not isinstance(command, list) or not (2 <= len(command) <= 16) or not all(isinstance(arg, str) for arg in command):
            raise ValueError("each verify command must be an argv array of 2-16 strings")
        if command[:2] != ["cargo", "test"] or any("\n" in arg or "\0" in arg for arg in command):
            raise ValueError("only cargo test verification is allowed")
        if any(arg.startswith("--config") or arg in ("--manifest-path", "--target-dir") for arg in command[2:]):
            raise ValueError("unsafe cargo test option")
    return data


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        raw = os.environ["FACTORY_MISSION_JSON"]
        mission = validate(raw, os.environ["GITHUB_SHA"], os.environ["GITHUB_REPOSITORY"])
        if mission["mission_id"] != os.environ["FACTORY_MISSION_ID"]:
            raise ValueError("dispatch mission_id does not match mission_json.mission_id")
        with open(args.output, "w", encoding="utf-8") as handle:
            json.dump(mission, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
        print(f"Validated mission {mission['mission_id']} with {len(mission['workers'])} independent implementer slots")
    except (KeyError, ValueError, json.JSONDecodeError) as error:
        print(f"Invalid engineering mission: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
