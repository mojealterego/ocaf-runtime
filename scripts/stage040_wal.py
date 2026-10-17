#!/usr/bin/env python3
"""Check consistency of supplied PostgreSQL WAL evidence (0.40.3/0.40.7).

This parser does not collect observations or authenticate hosts. Even a fully
consistent document cannot prove that an interhost TCP connection occurred.
"""
from __future__ import annotations

import re
from uuid import UUID

LSN = re.compile(r"[0-9A-Fa-f]{1,8}/[0-9A-Fa-f]{1,8}")
SHA256 = re.compile(r"[0-9a-f]{64}")
HOST_ID = re.compile(r"[0-9a-f]{32}")
EVIDENCE_SOURCES = frozenset(("operator_supplied", "live_collector", "simulation"))


class WalEvidenceError(ValueError):
    pass


def _lsn(value: str) -> int:
    """Parse the two unsigned 32-bit words of PostgreSQL's uint64 LSN."""
    if not isinstance(value, str) or not LSN.fullmatch(value):
        raise WalEvidenceError("invalid PostgreSQL LSN: two 1..8-digit hex words required")
    left, right = value.split("/", 1)
    return (int(left, 16) << 32) | int(right, 16)


def _boot_id(value: str) -> str:
    if not isinstance(value, str) or len(value) != 36:
        raise WalEvidenceError("canonical Linux kernel boot UUID required")
    try:
        parsed = UUID(value)
    except ValueError as exc:
        raise WalEvidenceError("invalid Linux kernel boot UUID") from exc
    if str(parsed) != value or parsed.version != 4:
        raise WalEvidenceError("canonical non-nil UUIDv4 kernel boot ID required")
    return value


def _host_id(value: str) -> str:
    if not isinstance(value, str) or not HOST_ID.fullmatch(value) or int(value, 16) == 0:
        raise WalEvidenceError("nonzero canonical Linux machine ID required")
    return value


def _timeline(value: int) -> int:
    if type(value) is not int or not 1 <= value <= 0xFFFFFFFF:
        raise WalEvidenceError("PostgreSQL timeline must be a nonzero uint32")
    return value


def validate_stream(*, primary_boot_id: str, standby_boot_id: str,
                    wal_receiver_status: str, primary_lsn: str,
                    standby_lsn: str, checksum_primary: str,
                    checksum_standby: str, primary_host_id: str | None = None,
                    standby_host_id: str | None = None,
                    primary_timeline: int | None = None,
                    standby_timeline: int | None = None,
                    evidence_source: str | None = None) -> dict:
    """Validate a quiesced, exactly caught-up snapshot on the same timeline.

    Boot/machine IDs and the source label are supplied claims, not identity
    attestation. Callers must authenticate live collection separately. Timeline
    history across promotion needs its own trace; equal LSNs on different
    timelines cannot be treated as evidence of matching WAL.
    """
    if not isinstance(evidence_source, str) or evidence_source not in EVIDENCE_SOURCES:
        raise WalEvidenceError("explicit evidence source required")
    primary_boot_id, standby_boot_id = _boot_id(primary_boot_id), _boot_id(standby_boot_id)
    primary_host_id, standby_host_id = _host_id(primary_host_id), _host_id(standby_host_id)
    if primary_boot_id == standby_boot_id or primary_host_id == standby_host_id:
        raise WalEvidenceError("primary and standby must claim distinct hosts and kernels")
    if wal_receiver_status != "streaming":
        raise WalEvidenceError("supplied standby receiver status is not streaming")
    timeline = _timeline(primary_timeline)
    if timeline != _timeline(standby_timeline):
        raise WalEvidenceError("LSN comparison across different timelines requires timeline history")
    primary, standby = _lsn(primary_lsn), _lsn(standby_lsn)
    if standby != primary:
        raise WalEvidenceError("standby has not exactly caught up to the sampled primary LSN")
    for checksum in (checksum_primary, checksum_standby):
        if not isinstance(checksum, str) or not SHA256.fullmatch(checksum):
            raise WalEvidenceError("nonempty canonical SHA-256 dataset checksum required")
    if checksum_primary != checksum_standby:
        raise WalEvidenceError("replicated dataset checksum mismatch")
    return {
        "status": "WAL_STREAM_EVIDENCE_CONSISTENT",
        "evidence_source": evidence_source,
        "observation_authenticated": False,
        "interhost_streaming_verified": False,
        "primary_kernel": primary_boot_id,
        "standby_kernel": standby_boot_id,
        "primary_host": primary_host_id,
        "standby_host": standby_host_id,
        "primary_lsn": primary_lsn,
        "standby_lsn": standby_lsn,
        "timeline": timeline,
        "lag_bytes": primary - standby,
        "checksum_sha256": checksum_primary,
        "wal_receiver_status": wal_receiver_status,
        "production_approved": False,
        "stage_040_complete": False,
    }
