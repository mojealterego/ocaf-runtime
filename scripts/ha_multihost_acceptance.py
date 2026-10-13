#!/usr/bin/env python3
"""Bounded, non-destructive SSH orchestration of OCAF PostgreSQL leader probes.

Must be run explicitly against three independently provisioned disposable
Linux hosts. This tests PostgreSQL lock exclusion and epoch monotonicity,
NOT production PKI, host independence, PostgreSQL failover or Zenoh RPC.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import select
import shlex
import subprocess
import sys
from typing import Any

HOST = re.compile(r"(?:[A-Za-z_][A-Za-z0-9_.-]{0,62}@)?[A-Za-z0-9][A-Za-z0-9_.-]{0,252}\Z")
PATH = re.compile(r"/[A-Za-z0-9_./-]{1,254}\Z")
CLUSTER = re.compile(r"ocaf33_[0-9a-fA-F]{32}\Z")
SSH = ("ssh", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes",
       "-o", "ConnectTimeout=8", "-T")


class AcceptanceError(ValueError):
    """A required independent observation is missing or inconsistent."""


def validate_args(hosts: list[str], cluster: str, binary: str, dsn_file: str) -> None:
    if len(hosts) != 3 or any(HOST.fullmatch(host) is None or host.startswith("-") for host in hosts):
        raise AcceptanceError("three explicit SSH destinations are required")
    if len(set(hosts)) != 3:
        raise AcceptanceError("SSH destinations must be distinct")
    if CLUSTER.fullmatch(cluster) is None:
        raise AcceptanceError("use an isolated ocaf33_ cluster and 32 hexadecimal characters")
    for value in (binary, dsn_file):
        if PATH.fullmatch(value) is None or ".." in value.split("/"):
            raise AcceptanceError("binary and DSN file must use safe absolute paths")


def ssh_command(host: str, cluster: str, binary: str, dsn_file: str, *probe_args: str) -> list[str]:
    remote_args = ["env", "OCAF_HA_PROBE_ACK=isolated-test-only",
                   f"OCAF_HA_PROBE_CLUSTER_ID={cluster}",
                   f"OCAF_CLUSTER_PG_DSN_FILE={dsn_file}",
                   "OCAF_ALLOW_INSECURE_LOOPBACK=0", binary, *probe_args]
    return [*SSH, host, " ".join(shlex.quote(arg) for arg in remote_args)]


def parse_record(raw: str, cluster: str, outcome: str) -> dict[str, Any]:
    if len(raw) > 4096:
        raise AcceptanceError("oversized remote probe record")
    try:
        value = json.loads(raw)
    except ValueError as exc:
        raise AcceptanceError("remote probe did not return JSON") from exc
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise AcceptanceError("unexpected probe schema")
    if value.get("cluster_id") != cluster or value.get("outcome") != outcome:
        raise AcceptanceError("probe cluster or outcome mismatch")
    if outcome in ("acquired", "released"):
        if type(value.get("epoch")) is not int or value["epoch"] <= 0:
            raise AcceptanceError("invalid leader epoch")
    elif value.get("epoch") is not None:
        raise AcceptanceError("denied probe must not report an epoch")
    return value


def check_sequence(cluster: str, start: dict[str, Any], denied_b: dict[str, Any],
                   denied_c: dict[str, Any], released: dict[str, Any],
                   acquired_b: dict[str, Any], acquired_c: dict[str, Any]) -> list[int]:
    a = parse_record(json.dumps(start), cluster, "acquired")["epoch"]
    parse_record(json.dumps(denied_b), cluster, "denied")
    parse_record(json.dumps(denied_c), cluster, "denied")
    end = parse_record(json.dumps(released), cluster, "released")["epoch"]
    b = parse_record(json.dumps(acquired_b), cluster, "acquired")["epoch"]
    c = parse_record(json.dumps(acquired_c), cluster, "acquired")["epoch"]
    if end != a or b <= a or c <= b:
        raise AcceptanceError("leader handover did not preserve increasing fencing epochs")
    return [a, b, c]


def invoke(host: str, cluster: str, binary: str, dsn_file: str, *arguments: str) -> dict[str, Any]:
    command = ssh_command(host, cluster, binary, dsn_file, *arguments)
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=20, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise AcceptanceError("SSH probe unavailable or timed out") from exc
    if result.returncode != 0 or len(result.stdout) > 4096:
        raise AcceptanceError("SSH probe failed or returned oversized output")
    return parse_record(result.stdout.strip(), cluster, arguments[arguments.index("--expect") + 1])


def orchestration(hosts: list[str], cluster: str, binary: str, dsn_file: str) -> dict[str, Any]:
    hold = subprocess.Popen(ssh_command(hosts[0], cluster, binary, dsn_file,
                                        "--mode", "hold", "--hold-ms", "30000"),
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                            text=True, bufsize=1)
    try:
        if hold.stdout is None:
            raise AcceptanceError("missing leader probe output")
        ready, _, _ = select.select([hold.stdout], [], [], 15)
        if not ready:
            raise AcceptanceError("initial leader did not acquire within the deadline")
        start = parse_record(hold.stdout.readline().strip(), cluster, "acquired")
        denied_b = invoke(hosts[1], cluster, binary, dsn_file, "--mode", "attempt", "--expect", "denied")
        denied_c = invoke(hosts[2], cluster, binary, dsn_file, "--mode", "attempt", "--expect", "denied")
        if hold.wait(timeout=45) != 0:
            raise AcceptanceError("leader did not complete the bounded hold")
        released = parse_record(hold.stdout.readline().strip(), cluster, "released")
        acquired_b = invoke(hosts[1], cluster, binary, dsn_file, "--mode", "attempt", "--expect", "acquired",
                            "--min-epoch", str(start["epoch"] + 1))
        acquired_c = invoke(hosts[2], cluster, binary, dsn_file, "--mode", "attempt", "--expect", "acquired",
                            "--min-epoch", str(acquired_b["epoch"] + 1))
        epochs = check_sequence(cluster, start, denied_b, denied_c, released, acquired_b, acquired_c)
        return {"schema_version": 1, "status": "LAB_PROBE_PASS_NOT_PRODUCTION_APPROVAL",
                "observed_at_utc": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
                "cluster_id": cluster, "ssh_destinations": hosts,
                "epochs": epochs, "scope": "PostgreSQL advisory exclusion and graceful handover only"}
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise AcceptanceError("bounded remote probe operation failed") from exc
    finally:
        if hold.poll() is None:
            hold.terminate()
            try:
                hold.wait(timeout=2)
            except subprocess.TimeoutExpired:
                hold.kill()
                hold.wait()
        if hold.stdout is not None:
            hold.stdout.close()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coordinator-a", required=True)
    parser.add_argument("--coordinator-b", required=True)
    parser.add_argument("--observer-c", required=True)
    parser.add_argument("--cluster-id", required=True)
    parser.add_argument("--binary", default="/usr/local/bin/ocaf-ha-probe")
    parser.add_argument("--dsn-file", default="/etc/ocaf/pg-dsn")
    parser.add_argument("--confirm-isolated-test", action="store_true", required=True)
    args = parser.parse_args(argv)
    try:
        hosts = [args.coordinator_a, args.coordinator_b, args.observer_c]
        validate_args(hosts, args.cluster_id, args.binary, args.dsn_file)
        if not args.confirm_isolated_test:
            raise AcceptanceError("isolated test authorization is required")
        result = orchestration(hosts, args.cluster_id, args.binary, args.dsn_file)
        print(json.dumps(result, sort_keys=True))
        return 0
    except AcceptanceError as exc:
        print(json.dumps({"status": "FAIL", "reason": str(exc)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
