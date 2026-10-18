#!/usr/bin/env python3
"""Run independent Codex engineers and produce a gated integration artifact.

Input is a trusted mission JSON supplied by the workflow gateway. This script
does not publish a branch or PR. It emits a patch and a receipt for a separate
read-only / publish permission boundary.
"""

import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid


class FactoryFailure(Exception):
    pass


def run(argv, *, cwd=None, env=None, input_text=None, timeout=60):
    return subprocess.run(
        argv, cwd=cwd, env=env, input=input_text, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout, check=False,
    )


def git(repo, *args, timeout=60):
    result = run(["git", *args], cwd=repo, timeout=timeout)
    if result.returncode:
        raise FactoryFailure(f"git {args[0]} failed (exit {result.returncode})")
    return result.stdout.strip()


def valid_name(value):
    return isinstance(value, str) and 0 < len(value) <= 128 and all(
        c.isascii() and (c.isalnum() or c in "-_.") for c in value
    )


def valid_scope(scope):
    if not isinstance(scope, str) or not scope or scope.startswith("/") or "\\" in scope:
        return False
    parts = scope.rstrip("/").split("/")
    return all(p not in ("", ".", "..", ".git") for p in parts)


def within(path, scope):
    scope = scope.rstrip("/")
    return path == scope or path.startswith(scope + "/")


def validate_mission(mission):
    if not isinstance(mission, dict) or not valid_name(mission.get("mission_id")):
        raise FactoryFailure("invalid mission_id")
    base_sha = mission.get("base_sha")
    if not isinstance(base_sha, str) or len(base_sha) != 40 or any(c not in "0123456789abcdef" for c in base_sha):
        raise FactoryFailure("mission requires pinned base_sha")
    workers = mission.get("workers")
    if not isinstance(workers, list) or not 2 <= len(workers) <= 8:
        raise FactoryFailure("mission requires 2 to 8 independent workers")
    ids = set()
    all_scopes = []
    for worker in workers:
        if not isinstance(worker, dict) or not valid_name(worker.get("id")) or not valid_name(worker.get("role")):
            raise FactoryFailure("invalid worker identity or role")
        if worker["id"] in ids:
            raise FactoryFailure("duplicate worker id")
        ids.add(worker["id"])
        prompt = worker.get("instructions")
        if not isinstance(prompt, str) or not prompt.strip() or len(prompt) > 32768:
            raise FactoryFailure("invalid worker instructions")
        scopes = worker.get("write_set")
        if not isinstance(scopes, list) or not scopes or len(scopes) > 64 or not all(map(valid_scope, scopes)):
            raise FactoryFailure("worker needs valid write_set paths")
        for scope in scopes:
            protected = (".gitmodules", ".github/workflows", ".github/scripts",
                         "scripts/engineering_factory_runner.py")
            if any(within(item, scope) or within(scope, item) for item in protected):
                raise FactoryFailure("protected write_set")
            if any(within(scope, prior) or within(prior, scope) for prior in all_scopes):
                raise FactoryFailure("worker write_set scopes overlap")
            all_scopes.append(scope)
    commands = mission.get("verify_commands", [["cargo", "test", "--locked", "--no-default-features"]])
    if not isinstance(commands, list) or not commands or len(commands) > 8:
        raise FactoryFailure("invalid verify_commands")
    for command in commands:
        if not isinstance(command, list) or not 2 <= len(command) <= 16 or not all(isinstance(s, str) for s in command):
            raise FactoryFailure("verify command must be an argv list")
        if command[:2] != ["cargo", "test"] or any("\n" in arg or "\0" in arg for arg in command):
            raise FactoryFailure("only cargo test verification is allowed")
        if any(arg.startswith("--config") or arg in ("--manifest-path", "--target-dir") for arg in command[2:]):
            raise FactoryFailure("unsafe cargo test option")
    return workers, commands


def codex_env():
    # The CLI needs the model credential. Pass only runtime essentials and no
    # GitHub token or unrelated CI secret into the Codex process.
    allowed = ("PATH", "HOME", "USER", "LANG", "LC_ALL", "TMPDIR", "CODEX_HOME",
               "OPENAI_API_KEY", "SSL_CERT_FILE", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY")
    return {key: os.environ[key] for key in allowed if key in os.environ}


def codex_command(codex_bin, tree, sandbox, message_path):
    # Codex receives OPENAI_API_KEY for API auth. Its model-executed shell
    # commands inherit no ambient variables, including that credential.
    return [codex_bin, "exec", "-C", str(tree), "-s", sandbox,
            "-c", "shell_environment_policy.inherit=none",
            "-c", "approval_policy=never", "--ephemeral",
            "-o", str(message_path)]


def unprivileged_env():
    env = codex_env()
    env.pop("OPENAI_API_KEY", None)
    env.pop("CODEX_HOME", None)
    return env


def changed_paths(repo):
    git(repo, "add", "-A")
    names = run(["git", "diff", "--cached", "--name-only", "--no-renames",
                 "--diff-filter=ACMRD", "-z"], cwd=repo)
    if names.returncode:
        raise FactoryFailure("could not enumerate worker changes")
    return [name for name in names.stdout.split("\0") if name]


def check_patch(repo, scopes):
    paths = changed_paths(repo)
    if not paths:
        raise FactoryFailure("worker produced no tracked changes")
    for path in paths:
        if not any(within(path, scope) for scope in scopes):
            raise FactoryFailure("worker changed a file outside write_set")
    modes = git(repo, "ls-files", "--stage")
    for line in modes.splitlines():
        mode, _, name = line.partition(" ")
        if mode == "120000" and name.split("\t")[-1] in paths:
            raise FactoryFailure("symlink changes are not permitted")
    patch = run(["git", "diff", "--cached", "--binary"], cwd=repo)
    if patch.returncode or not patch.stdout or len(patch.stdout.encode()) > 2_000_000:
        raise FactoryFailure("worker patch is empty or exceeds 2 MB")
    return paths, patch.stdout


def execute(mission_path, output_dir, repo, codex_bin):
    mission = json.loads(mission_path.read_text(encoding="utf-8"))
    workers, commands = validate_mission(mission)
    if git(repo, "status", "--porcelain"):
        raise FactoryFailure("repository checkout must be clean")
    base_sha = git(repo, "rev-parse", "HEAD")
    if base_sha != mission["base_sha"]:
        raise FactoryFailure("mission base_sha does not match checkout HEAD")
    output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ocaf-factory-") as temporary:
        scratch = Path(temporary)
        trees = [scratch / f"worker-{i}" for i in range(len(workers))]
        integration = scratch / "integration"
        review_tree = scratch / "review"
        # Each worker is a separate OS process and a separate Git worktree.
        # Worktree creation occurs in parent to avoid Git metadata lock races.
        for tree in trees:
            git(repo, "worktree", "add", "--detach", str(tree), base_sha)
        results = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=len(workers)) as pool:
            futures = [pool.submit(run_worker_in_tree, tree, worker, codex_bin)
                       for tree, worker in zip(trees, workers)]
            for future in futures:
                results.append(future.result())
        overlap_ms = max(
            max(0, min(a["ended_at_ms"], b["ended_at_ms"]) - max(a["started_at_ms"], b["started_at_ms"]))
            for index, a in enumerate(results) for b in results[index + 1:]
        )
        if overlap_ms == 0:
            raise FactoryFailure("worker execution did not overlap")
        git(repo, "worktree", "add", "--detach", str(integration), base_sha)
        for result in results:
            applied = run(["git", "apply", "--index", "-"], cwd=integration, input_text=result["patch"])
            if applied.returncode:
                raise FactoryFailure(f"integration conflict for worker {result['id']}")
        patch_result = run(["git", "diff", "--cached", "--binary"], cwd=integration)
        if patch_result.returncode or not patch_result.stdout:
            raise FactoryFailure("integrated patch is empty")
        patch_path = output_dir / "integrated.patch"
        patch_path.write_text(patch_result.stdout, encoding="utf-8")
        tests = []
        for command in commands:
            check = run(command, cwd=integration, env=unprivileged_env(), timeout=1800)
            tests.append({"command": command, "passed": check.returncode == 0, "exit_code": check.returncode})
            if check.returncode:
                raise FactoryFailure(f"verification failed (exit {check.returncode})")
        # Review reads the same integrated patch in a fresh, independent tree.
        git(repo, "worktree", "add", "--detach", str(review_tree), base_sha)
        review_apply = run(["git", "apply", "--index", "-"], cwd=review_tree, input_text=patch_result.stdout)
        if review_apply.returncode:
            raise FactoryFailure("review worktree could not load integrated patch")
        review = independent_review_in_tree(review_tree, codex_bin, scratch)
        receipt = {"mission_id": mission["mission_id"], "base_sha": base_sha,
                   "workers": [{k: v for k, v in result.items() if k != "patch"} for result in results],
                   "concurrency": {"overlap_ms": overlap_ms},
                   "tests": {"passed": True, "commands": tests}, "review": review}
        (output_dir / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        if review["decision"] != "approved":
            raise FactoryFailure("independent reviewer requested changes")
        return receipt


def run_worker_in_tree(tree, worker, codex_bin):
    message_path = tree.parent / f"{worker['id']}.last-message"
    prompt = (
        f"You are independent engineering worker {worker['id']} ({worker['role']}).\n"
        f"Mission instructions: {worker['instructions']}\n"
        f"Edit only these paths: {', '.join(worker['write_set'])}. "
        "Implement the task and run focused local checks. Do not commit, push, publish, "
        "or edit other workers' files.\n"
    )
    command = codex_command(codex_bin, tree, "workspace-write", message_path) + ["-"]
    instance_id = str(uuid.uuid4())
    started_at_ms = time.time_ns() // 1_000_000
    process = run(command, cwd=tree, env=codex_env(), input_text=prompt, timeout=3600)
    ended_at_ms = time.time_ns() // 1_000_000
    if process.returncode:
        raise FactoryFailure(f"worker {worker['id']} process failed (exit {process.returncode})")
    paths, patch = check_patch(tree, worker["write_set"])
    return {"id": worker["id"], "role": worker["role"], "instance_id": instance_id,
            "paths": paths, "patch": patch, "started_at_ms": started_at_ms,
            "ended_at_ms": ended_at_ms, "exit_code": process.returncode}


def independent_review_in_tree(tree, codex_bin, scratch):
    schema = scratch / "review-schema.json"
    schema.write_text(json.dumps({"type": "object", "additionalProperties": False,
        "properties": {"decision": {"type": "string", "enum": ["approved", "changes_requested"]},
                       "findings": {"type": "array", "items": {"type": "string"}}},
        "required": ["decision", "findings"]}), encoding="utf-8")
    result_path = scratch / "review.json"
    prompt = ("Review the integrated diff against the base for correctness, security, "
              "scope, and whether workers' changes fit together. Inspect source and tests. "
              "Return changes_requested for material defects. You are a separate reviewer "
              "with read-only access; do not edit files.\n")
    command = codex_command(codex_bin, tree, "read-only", result_path) + [
        "--output-schema", str(schema), "-"]
    instance_id = str(uuid.uuid4())
    started_at_ms = time.time_ns() // 1_000_000
    process = run(command, cwd=tree, env=codex_env(), input_text=prompt, timeout=1800)
    ended_at_ms = time.time_ns() // 1_000_000
    if process.returncode:
        raise FactoryFailure(f"independent review process failed (exit {process.returncode})")
    review = json.loads(result_path.read_text(encoding="utf-8"))
    if review.get("decision") not in ("approved", "changes_requested") or not isinstance(review.get("findings"), list):
        raise FactoryFailure("invalid review result")
    review["instance_id"] = instance_id
    review["started_at_ms"] = started_at_ms
    review["ended_at_ms"] = ended_at_ms
    review["exit_code"] = process.returncode
    return review


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mission-file", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    try:
        receipt = execute(args.mission_file, args.output_dir, repo, shutil.which("codex") or "codex")
        print(json.dumps({"mission_id": receipt["mission_id"], "decision": receipt["review"]["decision"]}))
    except (FactoryFailure, OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"engineering factory failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
