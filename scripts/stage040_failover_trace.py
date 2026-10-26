#!/usr/bin/env python3
"""Check measured WAL and timeline continuity in a one-kernel PostgreSQL lab.

This receipt proves replay of specific committed markers only. It cannot prove
zero RPO for all clients or fencing of a restarted primary on another host.
"""
from __future__ import annotations

import re

LSN = re.compile(r"[0-9A-F]+/[0-9A-F]+\Z")
TIMELINE = re.compile(r"[0-9A-F]{8}\Z")


class FailoverTraceError(ValueError):
    pass


def _lsn(value: object) -> int:
    if not isinstance(value, str) or LSN.fullmatch(value) is None:
        raise FailoverTraceError("invalid PostgreSQL LSN")
    high, low = value.split("/")
    return (int(high, 16) << 32) | int(low, 16)


def _timeline(value: object) -> int:
    if not isinstance(value, str) or TIMELINE.fullmatch(value) is None:
        raise FailoverTraceError("invalid PostgreSQL WAL timeline")
    return int(value, 16)


def validate_trace(trace: dict) -> dict:
    if (not isinstance(trace, dict) or trace.get("schema_version") != 1
            or trace.get("scope") != "ONE_KERNEL_THREE_POSTMASTERS"
            or trace.get("committed_markers_replayed") is not True
            or trace.get("all_client_rpo_zero_proved") is not False
            or trace.get("external_fencing_proved") is not False
            or trace.get("stage_040_complete") is not False):
        raise FailoverTraceError("unsupported or overstated WAL trace")
    before = _timeline(trace.get("a_timeline_before_crash"))
    promoted_b = _timeline(trace.get("b_timeline_after_promotion"))
    promoted_c = _timeline(trace.get("c_timeline_after_promotion"))
    if not (0 < before < promoted_b < promoted_c):
        raise FailoverTraceError("promotion timelines did not advance")
    source = _lsn(trace.get("a_commit_lsn"))
    b_before = _lsn(trace.get("b_replay_lsn_before_promotion"))
    c_before = _lsn(trace.get("c_replay_lsn_before_promotion"))
    b_commit = _lsn(trace.get("b_commit_lsn"))
    c_after_b = _lsn(trace.get("c_replay_lsn_after_b_commit"))
    c_commit = _lsn(trace.get("c_commit_lsn"))
    a_after_c = _lsn(trace.get("a_replay_lsn_after_c_commit"))
    if not (source > 0 and b_before >= source and c_before >= source
            and b_commit > source and c_after_b >= b_commit
            and c_commit > b_commit and a_after_c >= c_commit):
        raise FailoverTraceError("committed marker missing from WAL replay chain")
    return trace
