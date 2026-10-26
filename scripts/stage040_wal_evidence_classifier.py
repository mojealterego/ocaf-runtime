#!/usr/bin/env python3
"""Classify 0.40 WAL receipts without treating artifact handoff as live TCP.

This is an offline contract check. A signed or trusted collection channel is
still required before an operator can attribute a receipt to actual hosts.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from stage040_interhost_live import BOOT, InterhostPgError, _ipv4, lsn_position, safe_nonce, validate_receipt, validate_receiver


ARTIFACT = "THREE_REAL_DISTINCT_LINUX_KERNELS_LOCAL_POSTGRES_FAILOVERS_VERIFIED"
TWO_HOST = "TWO_KERNEL_PG_WAL_STREAMING_OBSERVED_LAB_ONLY"
THREE_HOST = "REAL_DISTINCT_LINUX_KERNELS_POSTGRESQL_LIVE_TCP_STREAMING_VERIFIED"


def classify(receipt: dict) -> dict:
    if not isinstance(receipt, dict):
        raise InterhostPgError("PostgreSQL evidence must be a JSON object")
    status = receipt.get("status")
    if status == ARTIFACT:
        boots = receipt.get("boot_ids")
        if (type(receipt.get("schema_version")) is not int or receipt["schema_version"] != 1
                or not isinstance(boots, dict) or set(boots) != {"a", "b", "c"}
                or any(not isinstance(x, str) or BOOT.fullmatch(x) is None for x in boots.values())
                or len(set(boots.values())) != 3
                or receipt.get("distinct_kernels_observed") != 3
                or receipt.get("three_real_pg_failovers_on_three_distinct_kernels") is not True
                or receipt.get("real_local_wal_primary_crash_and_reparent_each_kernel") is not True
                or receipt.get("interhost_postgres_streaming_verified") is not False
                or receipt.get("host_power_loss_tested") is not False
                or receipt.get("split_brain_fencing_tested") is not False
                or receipt.get("stage_040_complete") is not False
                or receipt.get("production_approved") is not False):
            raise InterhostPgError("artifact-mediated local failover receipt is inconsistent")
        kind = "ARTIFACT_MEDIATED_LOCAL_FAILOVER_ONLY"
        direct = False
        kernels = 3
    elif status == TWO_HOST:
        boots = receipt.get("boot_ids_before")
        after = receipt.get("boot_ids_after")
        if (type(receipt.get("schema_version")) is not int or receipt["schema_version"] != 1
                or not isinstance(boots, dict) or set(boots) != {"a", "b"}
                or after != boots or len(set(boots.values())) != 2
                or any(not isinstance(x, str) or BOOT.fullmatch(x) is None for x in boots.values())
                or receipt.get("direct_tcp_stream_observed") is not True
                or receipt.get("third_host_witness_tested") is not False
                or receipt.get("fencing_tested") is not False
                or receipt.get("stage_040_complete") is not False
                or receipt.get("production_approved") is not False):
            raise InterhostPgError("two-host live WAL receipt is incomplete")
        nonce = safe_nonce(receipt.get("fresh_nonce"))
        if not nonce:
            raise InterhostPgError("fresh SQL challenge missing")
        source = lsn_position(receipt.get("primary_wal_lsn"))
        replay = lsn_position(receipt.get("standby_replay_lsn"))
        health = receipt.get("transport_health")
        if (source <= 0 or replay < source or not isinstance(health, dict)
                or health.get("primary_challenge_lsn") != receipt["primary_wal_lsn"]
                or health.get("standby_replay_lsn") != receipt["standby_replay_lsn"]
                or health.get("received_not_before_challenge") is not True
                or health.get("replay_not_ahead_of_received") is not True
                or health.get("distinct_kernel_boot_ids") is not True):
            raise InterhostPgError("two-host WAL transport telemetry is inconsistent")
        receive = lsn_position(health.get("standby_receive_lsn"))
        if (receive < source or replay > receive
                or health.get("primary_to_receive_lag_bytes") != max(0, source - receive)
                or health.get("primary_to_replay_lag_bytes") != max(0, source - replay)):
            raise InterhostPgError("two-host WAL receive/replay order is inconsistent")
        primary = receipt.get("primary_replication_address")
        standby = receipt.get("standby_replication_address")
        if not isinstance(primary, str) or not isinstance(standby, str) or primary == standby:
            raise InterhostPgError("distinct PostgreSQL TCP peers missing")
        _ipv4(primary)
        _ipv4(standby)
        port = None
        for phase in ("standby_receiver_before", "standby_receiver_after"):
            receiver = receipt.get(phase)
            if not isinstance(receiver, dict):
                raise InterhostPgError("live receiver observation missing")
            observed_port = receiver.get("sender_port")
            if type(observed_port) is not int or not 1024 <= observed_port <= 65535:
                raise InterhostPgError("bounded PostgreSQL TCP port missing")
            if port is None:
                port = observed_port
            if observed_port != port:
                raise InterhostPgError("receiver port changed across live WAL observation")
            validate_receiver(receiver, primary, port)
        kind = "TWO_KERNEL_LIVE_TCP_WAL_RECEIPT_CONTRACT"
        direct = True
        kernels = 2
    elif status == THREE_HOST:
        validate_receipt(receipt)
        kind = "THREE_KERNEL_LIVE_TCP_WAL_RECEIPT_CONTRACT"
        direct = True
        kernels = 3
    else:
        raise InterhostPgError("unrecognized PostgreSQL WAL evidence class")
    return {
        "schema_version": 1,
        "status": kind,
        "observed_kernel_count_in_receipt": kernels,
        "direct_tcp_wal_claim_in_validated_receipt": direct,
        "offline_receipt_contract_only": True,
        "host_identity_independently_attested": False,
        "stage_040_complete": False,
        "production_approved": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if args.report.exists() or args.report.is_symlink():
        parser.error("fresh report path required")
    try:
        result = classify(json.loads(args.input.read_text()))
        args.report.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n")
    except (OSError, ValueError, TypeError) as exc:
        parser.error(str(exc))
    print(result["status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
