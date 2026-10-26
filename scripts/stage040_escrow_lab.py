#!/usr/bin/env python3
"""PostgreSQL-local escrow replay after synchronous standby promotion.

This is a two-postmaster, ONE-KERNEL lab. It never claims external effects,
independent hosts, quorum, or production approval.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time


class EscrowLabError(ValueError):
    pass


def validate_report(report: dict) -> dict:
    required = (
        "synchronous_remote_apply_observed", "first_reservation_replayed",
        "first_settlement_replayed", "duplicate_settlement_denied_after_promotion",
        "stale_epoch_denied_after_promotion", "new_epoch_settlement_committed",
        "old_primary_stopped_before_promotion", "both_postmasters_stopped_on_cleanup",
    )
    if (not isinstance(report, dict)
            or report.get("status") != "LOCAL_PG_ESCROW_SYNC_FAILOVER_REPLAY_PASS"
            or any(report.get(name) is not True for name in required)
            or report.get("distinct_kernel_boot_ids") is not False
            or report.get("external_effect_exactly_once_proved") is not False
            or report.get("stage_040_13_complete") is not False
            or report.get("production_approved") is not False):
        raise EscrowLabError("incomplete or overstated escrow failover receipt")
    return report


def command(argv: list[str], *, timeout: int = 30, success: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(argv, capture_output=True, text=True, timeout=timeout, check=False)
    if success and result.returncode:
        raise EscrowLabError(f"PostgreSQL command failed ({Path(argv[0]).name}): {result.stderr[-500:]}")
    return result


def available_ports() -> tuple[int, int]:
    sockets = [socket.socket(), socket.socket()]
    try:
        for item in sockets:
            item.bind(("127.0.0.1", 0))
        ports = tuple(item.getsockname()[1] for item in sockets)
        if ports[0] == ports[1]:
            raise EscrowLabError("PostgreSQL ports not distinct")
        return ports
    finally:
        for item in sockets:
            item.close()


SCHEMA = """
CREATE TABLE ocaf40_escrow_control (id integer PRIMARY KEY CHECK (id = 1), epoch bigint NOT NULL CHECK (epoch > 0));
INSERT INTO ocaf40_escrow_control VALUES (1, 1);
CREATE TABLE ocaf40_escrow_reservations (
  reservation_id text PRIMARY KEY, amount_cents bigint NOT NULL CHECK (amount_cents > 0));
CREATE TABLE ocaf40_escrow_settlements (
  settlement_id text PRIMARY KEY, reservation_id text NOT NULL UNIQUE REFERENCES ocaf40_escrow_reservations,
  amount_cents bigint NOT NULL CHECK (amount_cents > 0), leader_epoch bigint NOT NULL);
CREATE FUNCTION ocaf40_settle(expected_epoch bigint, settlement text, reservation text, amount bigint)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE actual_epoch bigint; reserved_amount bigint;
BEGIN
  SELECT epoch INTO actual_epoch FROM ocaf40_escrow_control WHERE id = 1 FOR UPDATE;
  IF actual_epoch IS DISTINCT FROM expected_epoch THEN
    RAISE EXCEPTION 'stale escrow epoch';
  END IF;
  SELECT amount_cents INTO reserved_amount FROM ocaf40_escrow_reservations
    WHERE reservation_id = reservation FOR UPDATE;
  IF reserved_amount IS DISTINCT FROM amount THEN
    RAISE EXCEPTION 'missing or mismatched reservation';
  END IF;
  INSERT INTO ocaf40_escrow_settlements VALUES (settlement, reservation, amount, expected_epoch);
END $$;
"""


def run_lab(pg_bin: Path) -> dict:
    if os.geteuid() == 0 or os.uname().sysname != "Linux":
        raise EscrowLabError("dedicated nonroot Linux runner required")
    if not pg_bin.is_absolute() or pg_bin.is_symlink() or not pg_bin.is_dir():
        raise EscrowLabError("absolute PostgreSQL binary directory required")
    for name in ("initdb", "pg_ctl", "pg_basebackup", "psql"):
        if not (pg_bin / name).is_file():
            raise EscrowLabError("PostgreSQL binary missing: " + name)
    tool = lambda name: str(pg_bin / name)
    port_a, port_b = available_ports()
    started: set[str] = set()
    stopped = False
    with tempfile.TemporaryDirectory(prefix="ocaf40-escrow-") as directory:
        base = Path(directory)
        a, b = base / "a", base / "b"
        a.mkdir(mode=0o700)
        def ctl(node: str, *args: str, success: bool = True):
            root = a if node == "a" else b
            return command([tool("pg_ctl"), "-D", str(root), *args], success=success)
        def start(node: str):
            root, port = (a, port_a) if node == "a" else (b, port_b)
            started.add(node)
            ctl(node, "-o", f"-p {port} -h 127.0.0.1 -k {base}",
                "-l", str(base / (node + ".log")), "-w", "start")
        def sql(node: str, query: str, *, success: bool = True) -> str:
            port = port_a if node == "a" else port_b
            result = command([tool("psql"), "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1",
                              "-h", "127.0.0.1", "-p", str(port), "-U", "postgres",
                              "-d", "postgres", "-c", query], timeout=10, success=success)
            return result.stdout.strip() if success else str(result.returncode)
        def until(check, label: str):
            deadline = time.monotonic() + 25
            while time.monotonic() < deadline:
                try:
                    if check():
                        return
                except EscrowLabError:
                    pass
                time.sleep(0.2)
            raise EscrowLabError("timed out waiting for " + label)
        try:
            command([tool("initdb"), "-D", str(a), "--auth-local=trust", "--auth-host=trust",
                     "--no-instructions"], timeout=45)
            with (a / "postgresql.conf").open("a") as stream:
                stream.write("\nwal_level = replica\nmax_wal_senders = 4\nmax_replication_slots = 4\n"
                             "hot_standby = on\nsynchronous_commit = remote_apply\nfsync = on\n")
            start("a")
            sql("a", SCHEMA)
            command([tool("pg_basebackup"), "-h", "127.0.0.1", "-p", str(port_a),
                     "-U", "postgres", "-D", str(b), "-F", "p", "-X", "stream",
                     "-R", "-c", "fast"], timeout=90)
            with (b / "postgresql.auto.conf").open("a") as stream:
                stream.write(f"\nprimary_conninfo = 'host=127.0.0.1 port={port_a} user=postgres application_name=b'\n")
            start("b")
            until(lambda: sql("a", "SELECT count(*) FROM pg_stat_replication WHERE application_name='b' AND state='streaming'") == "1",
                  "streaming standby")
            sql("a", "ALTER SYSTEM SET synchronous_standby_names = 'b'")
            sql("a", "SELECT pg_reload_conf()")
            until(lambda: sql("a", "SELECT count(*) FROM pg_stat_replication WHERE application_name='b' AND sync_state='sync'") == "1",
                  "synchronous remote_apply standby")
            if sql("a", "SHOW synchronous_commit") != "remote_apply":
                raise EscrowLabError("remote_apply commit acknowledgement not configured")
            sql("a", "INSERT INTO ocaf40_escrow_reservations VALUES ('reserve-1',500)")
            sql("a", "SELECT ocaf40_settle(1,'settle-1','reserve-1',500)")
            until(lambda: sql("b", "SELECT count(*) FROM ocaf40_escrow_settlements WHERE settlement_id='settle-1'") == "1",
                  "applied settlement WAL")
            first_lsn = sql("a", "SELECT pg_current_wal_lsn()")
            if re.fullmatch(r"[0-9A-F]+/[0-9A-F]+", first_lsn) is None:
                raise EscrowLabError("source WAL LSN missing")
            until(lambda: sql("b", f"SELECT pg_wal_lsn_diff(pg_last_wal_replay_lsn(), '{first_lsn}') >= 0") == "t",
                  "standby replay through acknowledged source WAL")
            replay_lsn = sql("b", "SELECT pg_last_wal_replay_lsn()")
            ctl("a", "-m", "immediate", "-w", "stop")
            started.remove("a")
            if sql("a", "SELECT 1", success=False) == "0":
                raise EscrowLabError("old primary remained available")
            ctl("b", "-w", "promote")
            until(lambda: sql("b", "SELECT pg_is_in_recovery()") == "f", "standby promotion")
            if sql("b", "SELECT reservation_id||':'||amount_cents FROM ocaf40_escrow_reservations") != "reserve-1:500":
                raise EscrowLabError("reservation lost after promotion")
            if sql("b", "SELECT settlement_id||':'||reservation_id||':'||amount_cents||':'||leader_epoch FROM ocaf40_escrow_settlements") != "settle-1:reserve-1:500:1":
                raise EscrowLabError("settlement lost after promotion")
            sql("b", "UPDATE ocaf40_escrow_control SET epoch=2 WHERE id=1")
            sql("b", "INSERT INTO ocaf40_escrow_reservations VALUES ('reserve-stale',600)")
            stale_denied = sql("b", "SELECT ocaf40_settle(1,'settle-stale','reserve-stale',600)", success=False) != "0"
            duplicate_denied = sql("b", "SELECT ocaf40_settle(2,'settle-1','reserve-1',500)", success=False) != "0"
            if not stale_denied or not duplicate_denied:
                raise EscrowLabError("stale epoch or duplicate settlement accepted")
            sql("b", "INSERT INTO ocaf40_escrow_reservations VALUES ('reserve-2',700)")
            sql("b", "SELECT ocaf40_settle(2,'settle-2','reserve-2',700)")
            if sql("b", "SELECT count(*) FROM ocaf40_escrow_settlements") != "2":
                raise EscrowLabError("new leader settlement not committed")
            result = {
                "schema_version": 1, "status": "LOCAL_PG_ESCROW_SYNC_FAILOVER_REPLAY_PASS",
                "observed_at_utc": datetime.now(timezone.utc).isoformat(),
                "host_boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                "source_commit_lsn": first_lsn, "standby_replay_lsn_before_promotion": replay_lsn,
                "synchronous_remote_apply_observed": True, "first_reservation_replayed": True,
                "first_settlement_replayed": True, "duplicate_settlement_denied_after_promotion": True,
                "stale_epoch_denied_after_promotion": True, "new_epoch_settlement_committed": True,
                "old_primary_stopped_before_promotion": True,
                "distinct_kernel_boot_ids": False, "external_effect_exactly_once_proved": False,
                "stage_040_13_complete": False, "production_approved": False,
            }
        finally:
            for node in tuple(started):
                ctl(node, "-m", "immediate", "-w", "stop", success=False)
            stopped = all(ctl(node, "status", success=False).returncode == 3 for node in ("a", "b"))
        result["both_postmasters_stopped_on_cleanup"] = stopped
        return validate_report(result)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pg-bin-dir", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--ack-local-postgres-escrow-lab", action="store_true")
    args = parser.parse_args()
    if not args.ack_local_postgres_escrow_lab or args.report.exists() or not args.report.is_absolute():
        parser.error("explicit lab acknowledgement and new absolute report path required")
    try:
        result = run_lab(args.pg_bin_dir)
        args.report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    except (OSError, subprocess.SubprocessError, EscrowLabError) as exc:
        parser.error(str(exc))
    print(result["status"])


if __name__ == "__main__":
    main()
