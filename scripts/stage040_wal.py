#!/usr/bin/env python3
"""Evidence validator for direct PostgreSQL WAL streaming (0.40.3/0.40.7)."""
from __future__ import annotations

import re

LSN = re.compile(r"^[0-9A-Fa-f]+/[0-9A-Fa-f]+$")


class WalEvidenceError(ValueError):
    pass


def _lsn(value: str) -> int:
    if not isinstance(value, str) or not LSN.fullmatch(value):
        raise WalEvidenceError("invalid PostgreSQL LSN")
    left, right = value.split("/", 1)
    return (int(left, 16) << 32) + int(right, 16)


def validate_stream(*, primary_boot_id: str, standby_boot_id: str,
                    wal_receiver_status: str, primary_lsn: str,
                    standby_lsn: str, checksum_primary: str,
                    checksum_standby: str) -> dict:
    if not isinstance(primary_boot_id, str) or not isinstance(standby_boot_id, str):
        raise WalEvidenceError("kernel boot IDs are required")
    if primary_boot_id == standby_boot_id:
        raise WalEvidenceError("primary and standby must have distinct kernels")
    if wal_receiver_status != "streaming":
        raise WalEvidenceError("standby is not receiving WAL over TCP")
    primary = _lsn(primary_lsn)
    standby = _lsn(standby_lsn)
    if standby > primary:
        raise WalEvidenceError("standby LSN is ahead of primary")
    if not isinstance(checksum_primary, str) or checksum_primary != checksum_standby:
        raise WalEvidenceError("replicated checksum mismatch")
    return {
        "status": "DIRECT_INTERHOST_WAL_STREAM_VERIFIED",
        "primary_kernel": primary_boot_id,
        "standby_kernel": standby_boot_id,
        "primary_lsn": primary_lsn,
        "standby_lsn": standby_lsn,
        "wal_receiver_status": wal_receiver_status,
        "checksum_equal": True,
        "production_approved": False,
    }

