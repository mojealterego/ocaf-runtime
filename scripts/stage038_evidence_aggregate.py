#!/usr/bin/env python3
"""Reject forged/incomplete OCAF 0.38 CI soak evidence; never assert staging GO."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from datetime import datetime, timezone

HEX_40 = re.compile(r"[0-9a-f]{40}\Z")
HEX_64 = re.compile(r"[0-9a-f]{64}\Z")
BOUND_LIMITS = {"max_rss_kib": 1572864, "max_fds": 512, "max_processes": 64}
MAX_SHARD_BYTES = 512 * 1024

def valid_cycle(item: object, i: int, limits: dict) -> bool:
    if not isinstance(item, dict) or item.get("cycle") != i or type(item.get("returncode")) is not int:
        return False
    if item["returncode"] != 0 or item.get("timed_out") is not False or "ceiling_violation" in item:
        return False
    if not isinstance(item.get("elapsed_sec"), (int, float)) or isinstance(item.get("elapsed_sec"), bool) or item["elapsed_sec"] <= 0:
        return False
    if type(item.get("samples")) is not int or item["samples"] < 1:
        return False
    for key, maximum in (
        ("peak_observed_rss_kib", limits["max_rss_kib"]),
        ("peak_observed_open_fds_per_process", limits["max_fds"]),
        ("peak_observed_process_count", limits["max_processes"]),
    ):
        value = item.get(key)
        if type(value) is not int or value < 0 or value > maximum:
            return False
    return True

def aggregate(directory: Path, expected_commit: str,
              expected_shards: int, cycles_per_shard: int) -> dict:
    if not HEX_40.fullmatch(expected_commit):
        raise ValueError("expected commit must be a full lowercase 40-character SHA")
    if not (1 <= expected_shards <= 20 and 1 <= cycles_per_shard <= 100):
        raise ValueError("expected shard/cycle counts outside approved bounds")
    expected = {f"shard-{i}.json" for i in range(1, expected_shards + 1)}
    actual = {f.name for f in directory.iterdir() if f.name.endswith(".json")}
    if actual != expected:
        raise ValueError(f"incomplete/extra shards: expected {len(expected)}, observed {len(actual)}")
    shared_engine = None
    totals = {"cycles": 0, "peak_rss_kib": 0, "peak_fds": 0, "peak_processes": 0}
    shard_digests: list[dict[str, object]] = []
    import hashlib
    for i in range(1, expected_shards + 1):
        file = directory / f"shard-{i}.json"
        if file.is_symlink() or not file.is_file() or file.stat().st_size > MAX_SHARD_BYTES:
            raise ValueError("unsafe or oversized evidence shard")
        raw = file.read_bytes()
        try:
            data = json.loads(raw)
        except (UnicodeError, ValueError) as exc:
            raise ValueError("invalid evidence JSON") from exc
        if not isinstance(data, dict) or data.get("schema_version") != 1:
            raise ValueError("unknown evidence schema")
        if data.get("passed") is not True:
            raise ValueError(f"shard {i} failed")
        if data.get("git_commit") != expected_commit or data.get("architecture") != "x86_64":
            raise ValueError(f"shard {i} commit/architecture mismatch")
        engine = data.get("engine_sha256")
        if not isinstance(engine, str) or not HEX_64.fullmatch(engine):
            raise ValueError(f"shard {i} missing valid engine hash")
        if shared_engine is not None and engine != shared_engine:
            raise ValueError("mixed Wasmtime engines in shard set")
        shared_engine = engine
        if not isinstance(data.get("kernel"), str) or not data["kernel"]:
            raise ValueError("missing kernel provenance")
        if data.get("limits") != BOUND_LIMITS:
            raise ValueError("unexpected soak resource limits")
        if data.get("cycles_requested") != cycles_per_shard or data.get("cycles_completed") != cycles_per_shard:
            raise ValueError("missing or misreported soak cycles")
        if data.get("new_private_workspace_residue") != []:
            raise ValueError("leftover private workspaces")
        start = data.get("started_unix_secs")
        end = data.get("completed_unix_secs")
        if type(start) is not int or type(end) is not int or end < start:
            raise ValueError("invalid soak interval")
        cycles = data.get("cycles")
        if not isinstance(cycles, list) or len(cycles) != cycles_per_shard:
            raise ValueError("wrong cycle array length")
        for num, cycle in enumerate(cycles, 1):
            if not valid_cycle(cycle, num, BOUND_LIMITS):
                raise ValueError(f"invalid cycle {num} of shard {i}")
            totals["cycles"] += 1
            totals["peak_rss_kib"] = max(totals["peak_rss_kib"], cycle["peak_observed_rss_kib"])
            totals["peak_fds"] = max(totals["peak_fds"], cycle["peak_observed_open_fds_per_process"])
            totals["peak_processes"] = max(totals["peak_processes"], cycle["peak_observed_process_count"])
        shard_digests.append({"shard": i, "sha256": hashlib.sha256(raw).hexdigest(), "cycles": len(cycles)})
    if totals["cycles"] != expected_shards * cycles_per_shard:
        raise ValueError("aggregate count mismatch")
    return {
        "schema_version": 1,
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "status": "CI_EXTENDED_SOAK_PASS_NOT_STAGING_GO",
        "git_commit": expected_commit,
        "engine_sha256": shared_engine,
        "shard_count": expected_shards,
        "cycles_per_shard": cycles_per_shard,
        "cycles_total": totals["cycles"],
        "observed_peaks": totals,
        "shard_digests": shard_digests,
        "limitations": [
            "Each shard ran on an independent ephemeral GitHub x86_64 runner.",
            "Aggregate is not a 100-cycle single-host staging soak.",
            "RSS, FDs and processes sampled at 20ms; not a kernel cgroup peak bound.",
            "No signed operator evidence, deployed coordinator, cross-host chaos or independent audit."
        ],
        "stage_038_complete": False,
        "production_approved": False,
    }

def save_report(path: Path, report: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    original = os.umask(0o077)
    try:
        with path.open("x", encoding="utf-8") as fp:
            json.dump(report, fp, indent=2, sort_keys=True)
            fp.write("\n")
    finally:
        os.umask(original)

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--directory", type=Path, required=True)
    ap.add_argument("--expected-commit", required=True)
    ap.add_argument("--expected-shards", type=int, default=5)
    ap.add_argument("--cycles-per-shard", type=int, default=20)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    try:
        evidence = aggregate(args.directory, args.expected_commit,
                             args.expected_shards, args.cycles_per_shard)
        save_report(args.output, evidence)
    except (OSError, ValueError) as exc:
        print(f"NO_GO: {exc}", file=sys.stderr)
        return 2
    print(json.dumps({
        "status": evidence["status"], "cycles_total": evidence["cycles_total"],
        "stage_038_complete": False, "output": str(args.output)
    }))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
