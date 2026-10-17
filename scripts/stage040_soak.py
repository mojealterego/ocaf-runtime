#!/usr/bin/env python3
"""Validate measured PostgreSQL fencing soak evidence without inventing it."""
from __future__ import annotations


class SoakEvidenceError(ValueError):
    pass


def validate_soak(report: dict, *, minimum_seconds: int = 3600) -> dict:
    if not isinstance(report, dict):
        raise SoakEvidenceError("soak report must be an object")
    if type(report.get("duration_seconds")) is not int or report["duration_seconds"] < minimum_seconds:
        raise SoakEvidenceError("required measured soak duration is missing")
    if type(report.get("client_count")) is not int or not 8 <= report["client_count"] <= 32:
        raise SoakEvidenceError("soak must use 8..32 real clients")
    if report.get("partition_injected") is not True or report.get("old_primary_writable") is not False:
        raise SoakEvidenceError("fencing partition evidence is incomplete")
    if report.get("committed_tx_ids_match") is not True or report.get("wal_ack_loss") is not False:
        raise SoakEvidenceError("commit/WAL acknowledgement evidence is incomplete")
    return {
        "status": "POSTGRES_FENCING_SOAK_VERIFIED",
        "duration_seconds": report["duration_seconds"],
        "client_count": report["client_count"],
        "production_approved": False,
        "stage_040_complete": False,
    }

