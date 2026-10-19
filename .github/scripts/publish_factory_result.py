#!/usr/bin/env python3
"""Publish an independently reviewed factory patch as a draft pull request."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from validate_factory_mission import scope, within


def run(*args, check=True, capture=True):
    result = subprocess.run(args, text=True, capture_output=capture, check=False)
    if check and result.returncode:
        raise RuntimeError(f"{' '.join(args[:3])} failed: {result.stderr.strip()}")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mission", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--patch", required=True)
    args = parser.parse_args()
    mission = json.loads(Path(args.mission).read_text(encoding="utf-8"))
    receipt = json.loads(Path(args.receipt).read_text(encoding="utf-8"))
    patch = Path(args.patch)
    if not patch.is_file() or patch.stat().st_size == 0 or patch.stat().st_size > 4_000_000:
        raise ValueError("integrated patch must be nonempty and at most 4 MB")
    if receipt.get("mission_id") != mission["mission_id"] or receipt.get("base_sha") != mission["base_sha"]:
        raise ValueError("receipt does not match the dispatched mission and base SHA")
    workers = receipt.get("workers")
    if not isinstance(workers, list) or len(workers) != len(mission["workers"]):
        raise ValueError("receipt is missing independent worker records")
    expected_ids = {worker["id"] for worker in mission["workers"]}
    worker_ids = {worker.get("id") for worker in workers if isinstance(worker, dict)}
    instance_ids = {worker.get("instance_id") for worker in workers if isinstance(worker, dict)}
    if worker_ids != expected_ids or len(instance_ids) != len(workers) or not all(instance_ids):
        raise ValueError("receipt worker IDs or instance IDs do not prove independent workers")
    review = receipt.get("review")
    if not isinstance(review, dict) or review.get("decision") != "approved":
        raise ValueError("independent review has not approved this patch")
    if not review.get("instance_id") or review["instance_id"] in instance_ids:
        raise ValueError("independent reviewer instance is missing or reused")
    tests = receipt.get("tests")
    if not isinstance(tests, dict) or tests.get("passed") is not True:
        raise ValueError("integrated tests did not pass")
    commands = tests.get("commands")
    if not isinstance(commands, list) or len(commands) != len(mission["verify_commands"]) or not all(
        isinstance(command, dict)
        and command.get("command") == expected
        and command.get("passed") is True
        and command.get("exit_code") == 0
        for command, expected in zip(commands, mission["verify_commands"])
    ):
        raise ValueError("integrated test evidence is incomplete")
    if run("git", "rev-parse", "HEAD").stdout.strip() != mission["base_sha"]:
        raise ValueError("publish checkout is not pinned to the mission base SHA")
    scopes = [scope(item) for worker in mission["workers"] for item in worker["write_set"]]
    run("git", "apply", "--check", "--binary", str(patch))
    run("git", "apply", "--index", "--binary", str(patch))
    changed = [path.decode("utf-8") for path in run("git", "diff", "--cached", "--name-only", "-z").stdout.encode("utf-8").split(b"\0") if path]
    if not changed:
        raise ValueError("integrated patch has no changed files")
    for path in changed:
        scope(path)
        if not any(within(path, allowed) for allowed in scopes):
            raise ValueError(f"patch changed a path outside approved write sets: {path}")
    run("git", "diff", "--cached", "--check")
    run("git", "config", "user.name", "ocaf-factory[bot]")
    run("git", "config", "user.email", "ocaf-factory[bot]@users.noreply.github.com")
    branch = f"factory/{mission['mission_id']}"
    run("git", "checkout", "-b", branch)
    run("git", "commit", "-m", f"factory: {mission['mission_id']}")
    candidate_tree = run("git", "rev-parse", "HEAD^{tree}").stdout.strip()
    run("gh", "auth", "setup-git")
    remote = run("git", "ls-remote", "--exit-code", "origin", f"refs/heads/{branch}", check=False)
    if remote.returncode == 0:
        run("git", "fetch", "--no-tags", "origin", f"refs/heads/{branch}")
        remote_tree = run("git", "rev-parse", "FETCH_HEAD^{tree}").stdout.strip()
        if remote_tree != candidate_tree:
            raise ValueError(f"branch {branch} already exists with different content; refusing to overwrite")
    elif remote.returncode == 2:
        run("git", "push", "origin", f"HEAD:refs/heads/{branch}")
    else:
        raise RuntimeError(f"cannot inspect existing branch: {remote.stderr.strip()}")
    existing = json.loads(run("gh", "pr", "list", "--state", "open", "--head", branch, "--json", "number,url").stdout)
    if existing:
        pr = existing[0]
    else:
        body = Path(os.environ["RUNNER_TEMP"]) / "factory-pr-body.md"
        body.write_text(
            f"## Mission {mission['mission_id']}\n\n"
            f"{mission['objective']}\n\n"
            "## Acceptance\n\n"
            + "".join(f"- {criterion}\n" for criterion in mission["acceptance"])
            + "\nGenerated by separate engineering workers. Independent review and integrated tests are recorded in the workflow artifact. "
            "This pull request is a draft pending GitHub CI and human review.\n",
            encoding="utf-8",
        )
        url = run(
            "gh", "pr", "create", "--draft", "--base", "main", "--head", branch,
            "--title", f"Factory: {mission['mission_id']}", "--body-file", str(body),
        ).stdout.strip()
        pr = json.loads(run("gh", "pr", "view", url, "--json", "number,url").stdout)
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"pr_number={pr['number']}\n")
            handle.write(f"pr_url={pr['url']}\n")
    print(f"Draft PR: {pr['url']}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, RuntimeError, json.JSONDecodeError) as error:
        print(f"Factory publish failed: {error}", file=sys.stderr)
        raise SystemExit(1)
