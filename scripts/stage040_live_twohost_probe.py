#!/usr/bin/env python3
"""Operator-run 0.40.3 A→B live WAL challenge on distinct Linux kernels.

This is the smallest direct TCP replication proof. It never claims quorum,
failover, or completion of stage 0.40. Existing three-host HA gates are separate.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import secrets
import stat
import time

from stage040_interhost_live import (
    BOOT, InterhostPgError, _ssh, lsn_position, safe_nonce,
    validate_host, validate_receiver,
)


def validate_inventory(inventory: dict, known_hosts: Path) -> dict:
    if (not isinstance(inventory, dict)
            or set(inventory) != {"schema_version", "database", "postgres_port",
                                      "postgres_socket_dir", "nodes"}
            or type(inventory["schema_version"]) is not int
            or inventory["schema_version"] != 1
            or inventory["database"] != "ocaf_stage040_lab"
            or type(inventory["postgres_port"]) is not int
            or not 1024 <= inventory["postgres_port"] <= 65535
            or inventory["postgres_socket_dir"] not in ("/var/run/postgresql", "/tmp")
            or not isinstance(inventory["nodes"], dict)
            or set(inventory["nodes"]) != {"a", "b"}):
        raise InterhostPgError("dedicated two-host PostgreSQL lab inventory required")
    hosts = {role: validate_host(row) for role, row in inventory["nodes"].items()}
    if (hosts["a"]["role"] != "a" or hosts["b"]["role"] != "b"
            or hosts["a"]["host"] == hosts["b"]["host"]
            or hosts["a"]["replication_address"] == hosts["b"]["replication_address"]):
        raise InterhostPgError("two distinct source and standby endpoints required")
    if (not isinstance(known_hosts, Path) or not known_hosts.is_absolute()
            or known_hosts.is_symlink()):
        raise InterhostPgError("absolute pinned known_hosts file required")
    try:
        st = known_hosts.stat()
        if (not stat.S_ISREG(st.st_mode) or st.st_uid != os.geteuid()
                or st.st_mode & 0o077 or not 60 < st.st_size <= 131072):
            raise InterhostPgError("private pinned known_hosts file required")
        pins = known_hosts.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as exc:
        raise InterhostPgError("unreadable known_hosts pin") from exc
    for node in hosts.values():
        label = node["host"] if node["ssh_port"] == 22 else \
            f'[{node["host"]}]:{node["ssh_port"]}'
        if not any(line.startswith(label + " ") for line in pins):
            raise InterhostPgError("missing SSH host key pin for " + label)
    return hosts


def run(inventory: dict, known_hosts: Path, report: Path, *, ssh=None,
        nonce: str | None = None, timeout_seconds: float = 45) -> dict:
    hosts = validate_inventory(inventory, known_hosts)
    if (not isinstance(report, Path) or not report.is_absolute()
            or report.exists() or report.is_symlink() or not report.parent.is_dir()):
        raise InterhostPgError("fresh absolute report destination required")
    probe = _ssh if ssh is None else ssh
    port = inventory["postgres_port"]
    sock = inventory["postgres_socket_dir"]

    def remote(role: str, argv: list[str]) -> str:
        return probe(hosts[role], known_hosts, argv, timeout=20)

    def sql(role: str, query: str) -> str:
        return remote(role, ["sudo", "-n", "-u", "postgres", "--", "psql",
                             "-X", "-A", "-t", "-q", "-v", "ON_ERROR_STOP=1",
                             "-h", sock, "-p", str(port), "-U", "postgres",
                             "-d", "ocaf_stage040_lab", "-c", query])

    def snapshot() -> tuple[dict, dict]:
        boots = {role: remote(role, ["cat", "/proc/sys/kernel/random/boot_id"])
                 for role in ("a", "b")}
        if (boots["a"] == boots["b"] or any(BOOT.fullmatch(x) is None for x in boots.values())):
            raise InterhostPgError("two distinct live kernel boot IDs required")
        if sql("a", "SELECT pg_is_in_recovery()") != "f" or \
                sql("b", "SELECT pg_is_in_recovery()") != "t":
            raise InterhostPgError("primary and standby roles changed")
        clients = sql("a", "SELECT client_addr::text FROM pg_stat_replication "
                            "WHERE state='streaming'").splitlines()
        if clients != [hosts["b"]["replication_address"]]:
            raise InterhostPgError("primary does not observe exact B streaming TCP peer")
        try:
            receiver = json.loads(sql("b", "SELECT row_to_json(receiver)::text "
                                           "FROM pg_stat_wal_receiver AS receiver"))
        except (TypeError, ValueError) as exc:
            raise InterhostPgError("standby receiver observation absent") from exc
        receiver = validate_receiver(receiver, hosts["a"]["replication_address"], port)
        return boots, receiver

    before_boots, before_receiver = snapshot()
    sql("a", "CREATE TABLE IF NOT EXISTS ocaf40_live_probe("
             "nonce TEXT PRIMARY KEY,origin_boot_id TEXT NOT NULL,"
             "inserted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp())")
    challenge = safe_nonce(secrets.token_hex(16) if nonce is None else nonce)
    sql("a", f"INSERT INTO ocaf40_live_probe(nonce,origin_boot_id) "
             f"VALUES ('{challenge}','{before_boots['a']}')")
    source_lsn = sql("a", "SELECT pg_current_wal_lsn()")
    source_position = lsn_position(source_lsn)
    if source_position == 0:
        raise InterhostPgError("primary WAL challenge position is zero")
    deadline = time.monotonic() + timeout_seconds
    observed_lsn = ""
    while time.monotonic() < deadline:
        observed_lsn = sql("b", "SELECT pg_last_wal_replay_lsn()")
        if observed_lsn and lsn_position(observed_lsn) >= source_position:
            row = sql("b", "SELECT nonce || ':' || origin_boot_id "
                           "FROM ocaf40_live_probe WHERE nonce='" + challenge + "'")
            if row == challenge + ":" + before_boots["a"]:
                break
        time.sleep(.5)
    else:
        raise InterhostPgError("standby did not replay fresh primary SQL challenge")
    after_boots, after_receiver = snapshot()
    if after_boots != before_boots:
        raise InterhostPgError("host kernel restarted during live WAL observation")
    output = {
        "schema_version": 1,
        "status": "TWO_KERNEL_PG_WAL_STREAMING_OBSERVED_LAB_ONLY",
        "observed_at_utc": datetime.now(timezone.utc).isoformat(),
        "boot_ids_before": before_boots, "boot_ids_after": after_boots,
        "primary_replication_address": hosts["a"]["replication_address"],
        "standby_replication_address": hosts["b"]["replication_address"],
        "standby_receiver_before": before_receiver,
        "standby_receiver_after": after_receiver,
        "fresh_nonce": challenge, "primary_wal_lsn": source_lsn,
        "standby_replay_lsn": observed_lsn,
        "direct_tcp_stream_observed": True,
        "third_host_witness_tested": False,
        "fencing_tested": False,
        "stage_040_complete": False,
        "production_approved": False,
    }
    fd = os.open(report, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as stream:
        json.dump(output, stream, sort_keys=True, indent=2)
        stream.write("\n")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", required=True, type=Path)
    parser.add_argument("--known-hosts", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if (not args.inventory.is_absolute() or args.inventory.is_symlink()
            or args.inventory.stat().st_mode & 0o077):
        parser.error("private absolute inventory required")
    try:
        result = run(json.loads(args.inventory.read_text()), args.known_hosts, args.report)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    print(result["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
