#!/usr/bin/env python3
"""Check supplied PostgreSQL fencing soak measurement sequences.

A JSON document cannot authenticate a live experiment. This validator checks
internal consistency only and never turns caller-supplied flags into a real
soak or production approval. Each sample is a quiesced synchronous-commit
checkpoint, so its durable transaction IDs must exactly match acknowledged IDs.
"""
from __future__ import annotations

from datetime import datetime, timezone
import math
import re

from stage040_wal import EVIDENCE_SOURCES, WalEvidenceError, _lsn, _timeline

PHASES = ("baseline", "partition", "failover", "old_primary_return")
TX_ID = re.compile(r"[1-9][0-9]{0,19}")
MAX_SAMPLES = 200_000
MAX_TX_IDS = 1_000_000
MAX_SAMPLE_GAP_SECONDS = 60


class SoakEvidenceError(ValueError):
    pass


def _number(value, label: str, *, positive: bool = False) -> float:
    if type(value) not in (int, float):
        raise SoakEvidenceError(label + " must be a finite number")
    try:
        finite = math.isfinite(value)
    except OverflowError as exc:
        raise SoakEvidenceError(label + " is out of range") from exc
    if not finite or value < 0 or (positive and value == 0):
        raise SoakEvidenceError(label + " must be finite and " + ("positive" if positive else "nonnegative"))
    return float(value)


def _timestamp(value) -> datetime:
    if not isinstance(value, str) or len(value) > 40:
        raise SoakEvidenceError("bounded UTC observation timestamp required")
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError as exc:
        raise SoakEvidenceError("invalid observation timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset() != timezone.utc.utcoffset(parsed):
        raise SoakEvidenceError("observation timestamp must explicitly use UTC")
    return parsed


def _tx_id(value) -> str:
    if not isinstance(value, str) or not TX_ID.fullmatch(value) or int(value) >= 1 << 64:
        raise SoakEvidenceError("canonical nonzero unsigned xid8 strings required")
    return value


def _tx_ids(values, label: str) -> set[str]:
    if not isinstance(values, list) or not 1 <= len(values) <= MAX_TX_IDS:
        raise SoakEvidenceError(label + " must contain measured transaction IDs")
    result = {_tx_id(value) for value in values}
    if len(result) != len(values):
        raise SoakEvidenceError(label + " contains duplicate transaction IDs")
    return result


def _wal_acks(values, tx_ids: set[str]) -> dict[str, tuple[int, int]]:
    if not isinstance(values, list) or len(values) != len(tx_ids):
        raise SoakEvidenceError("each acknowledged transaction needs its WAL position")
    result = {}
    for value in values:
        if not isinstance(value, dict):
            raise SoakEvidenceError("WAL acknowledgement must be an object")
        tx_id = _tx_id(value.get("tx_id"))
        if tx_id in result:
            raise SoakEvidenceError("duplicate transaction WAL acknowledgement")
        try:
            result[tx_id] = (_timeline(value.get("timeline")), _lsn(value.get("lsn")))
        except WalEvidenceError as exc:
            raise SoakEvidenceError(str(exc)) from exc
    if result.keys() != tx_ids:
        raise SoakEvidenceError("WAL acknowledgements do not match committed transaction IDs")
    return result


def _write_rejection(value, *, returned: bool) -> None:
    # TCP failure demonstrates unreachability, not fencing after a host returns.
    allowed = ("25006", "42501") if returned else ("25006", "42501", "08006", "57P01")
    if (not isinstance(value, dict) or value.get("sqlstate") not in allowed
            or type(value.get("rows_affected")) is not int or value["rows_affected"] != 0):
        raise SoakEvidenceError("measured old-primary write rejection required")


def _timeline_history(values) -> dict[int, tuple[int, int]]:
    """Map each child to its parent and exact PostgreSQL history-file fork."""
    if not isinstance(values, list) or not 1 <= len(values) <= 128:
        raise SoakEvidenceError("bounded promotion timeline history required")
    history = {}
    parents = set()
    for value in values:
        if not isinstance(value, dict):
            raise SoakEvidenceError("timeline history entries must be objects")
        try:
            parent = _timeline(value.get("parent_timeline"))
            child = _timeline(value.get("child_timeline"))
            fork = _lsn(value.get("fork_lsn"))
        except WalEvidenceError as exc:
            raise SoakEvidenceError(str(exc)) from exc
        if child <= parent or child in history or parent in parents:
            raise SoakEvidenceError("single unambiguous forward promotion history required")
        history[child] = (parent, fork)
        parents.add(parent)
    return history


def _validate_ack_replay(ack: dict[str, tuple[int, int]], timeline: int,
                         replay_lsn: int, history: dict[int, tuple[int, int]]) -> None:
    for ack_timeline, ack_lsn in ack.values():
        ancestor = timeline
        while ancestor != ack_timeline:
            if ancestor not in history:
                raise SoakEvidenceError("acknowledged WAL timeline is not a replay ancestor")
            ancestor, fork_lsn = history[ancestor]
            if ack_lsn > fork_lsn:
                raise SoakEvidenceError("acknowledged WAL lies beyond promotion's retained fork")
        if ack_lsn > replay_lsn:
            raise SoakEvidenceError("replay has not reached an acknowledged WAL position")


def validate_soak(report: dict, *, minimum_seconds: int = 3600) -> dict:
    """Reject flags, missing samples, lost ACKs, stale replay and unfenced return.

    Monotonic timestamps, wall timestamps and explicit contiguous sequence
    numbers must agree. Samples cannot have gaps exceeding 60 seconds. These
    constraints bound the supplied measurement trace; they do not prove its
    origin or observe behavior between samples.
    """
    minimum = _number(minimum_seconds, "minimum soak duration", positive=True)
    if not isinstance(report, dict) or type(report.get("schema_version")) is not int or report["schema_version"] != 2:
        raise SoakEvidenceError("soak schema version 2 with raw measurements required")
    source = report.get("evidence_source")
    if not isinstance(source, str) or source not in EVIDENCE_SOURCES:
        raise SoakEvidenceError("explicit evidence source required")
    duration = _number(report.get("duration_seconds"), "soak duration", positive=True)
    clients = report.get("client_count")
    if type(clients) is not int or not 8 <= clients <= 32:
        raise SoakEvidenceError("soak must measure 8..32 clients")
    samples = report.get("samples")
    if not isinstance(samples, list) or not 4 <= len(samples) <= MAX_SAMPLES:
        raise SoakEvidenceError("bounded multi-sample measurement sequence required")
    history = _timeline_history(report.get("timeline_history"))
    observed_promotions = set()

    start_mono = start_wall = previous_mono = previous_wall = None
    previous_phase = -1
    previous_ack = {}
    previous_replay = None
    phase_first_ids = {}
    phase_last_ids = {}
    for sequence, sample in enumerate(samples):
        if not isinstance(sample, dict) or type(sample.get("sequence")) is not int or sample["sequence"] != sequence:
            raise SoakEvidenceError("contiguous integer measurement sequence required")
        mono = _number(sample.get("monotonic_seconds"), "sample monotonic timestamp")
        wall = _timestamp(sample.get("observed_at_utc"))
        if previous_mono is not None:
            step = mono - previous_mono
            wall_step = (wall - previous_wall).total_seconds()
            if not 0 < step <= MAX_SAMPLE_GAP_SECONDS or wall_step <= 0 or abs(step - wall_step) > 1:
                raise SoakEvidenceError("measurement timestamps regressed, diverged or contain a sampling gap")
            if abs((wall - start_wall).total_seconds() - (mono - start_mono)) > 1:
                raise SoakEvidenceError("wall clock and monotonic measured duration disagree")
        else:
            start_mono, start_wall = mono, wall
        if type(sample.get("active_clients")) is not int or sample["active_clients"] != clients:
            raise SoakEvidenceError("measured client count must match the requested continuous load")
        phase = sample.get("phase")
        if not isinstance(phase, str) or phase not in PHASES:
            raise SoakEvidenceError("explicit fault phase required")
        phase_index = PHASES.index(phase)
        if phase_index not in (previous_phase, previous_phase + 1):
            raise SoakEvidenceError("baseline, partition, failover and old-primary-return phases required in order")
        if phase_index > 0:
            _write_rejection(sample.get("old_primary_write_attempt"), returned=phase == "old_primary_return")
        tx_ids = _tx_ids(sample.get("acknowledged_tx_ids"), "acknowledged transaction IDs")
        replayed = _tx_ids(sample.get("replayed_tx_ids"), "replayed transaction IDs")
        if tx_ids != replayed:
            raise SoakEvidenceError("durable replay differs from exactly acknowledged committed transactions")
        ack = _wal_acks(sample.get("wal_acks"), tx_ids)
        if not previous_ack.keys() <= ack.keys() or any(ack[tx_id] != pos for tx_id, pos in previous_ack.items()):
            raise SoakEvidenceError("previously acknowledged transaction or WAL position was lost or rewritten")
        try:
            timeline, replay_lsn = _timeline(sample.get("timeline")), _lsn(sample.get("replay_lsn"))
        except WalEvidenceError as exc:
            raise SoakEvidenceError(str(exc)) from exc
        _validate_ack_replay(ack, timeline, replay_lsn, history)
        if previous_replay is not None:
            previous_timeline, previous_lsn = previous_replay
            if timeline < previous_timeline or replay_lsn < previous_lsn:
                raise SoakEvidenceError("timeline or replay LSN regressed")
            if timeline != previous_timeline and phase != "failover":
                raise SoakEvidenceError("timeline change outside failover lacks promotion evidence")
            if timeline != previous_timeline:
                transition = history.get(timeline)
                if (transition is None or transition[0] != previous_timeline
                        or not previous_lsn <= transition[1] <= replay_lsn):
                    raise SoakEvidenceError("promotion fork does not preserve the observed prior WAL")
                observed_promotions.add(timeline)
        phase_first_ids.setdefault(phase, tx_ids)
        phase_last_ids[phase] = tx_ids
        previous_mono, previous_wall, previous_phase = mono, wall, phase_index
        previous_ack, previous_replay = ack, (timeline, replay_lsn)

    measured = previous_mono - start_mono
    if measured < minimum or abs(duration - measured) > 0.001:
        raise SoakEvidenceError("reported duration must match actual sample span and minimum soak duration")
    if tuple(phase_first_ids) != PHASES:
        raise SoakEvidenceError("all required fencing fault phases must be measured")
    if observed_promotions != history.keys():
        raise SoakEvidenceError("every supplied promotion must appear in the measured failover trace")
    for old_phase, new_phase in zip(PHASES, PHASES[1:]):
        if not phase_last_ids[new_phase] > phase_last_ids[old_phase]:
            raise SoakEvidenceError("committed workload must progress through partition, promotion and old-primary return")
    return {
        "status": "SOAK_EVIDENCE_CONSISTENT",
        "evidence_source": source,
        "observation_authenticated": False,
        "real_soak_verified": False,
        "duration_seconds": measured,
        "client_count": clients,
        "samples_count": len(samples),
        "committed_transactions": len(previous_ack),
        "production_approved": False,
        "stage_040_complete": False,
    }
