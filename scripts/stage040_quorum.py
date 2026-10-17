#!/usr/bin/env python3
"""Fail-closed witness lease for PostgreSQL promotion decisions (0.40.8).

This is the authority boundary used by a real inter-host adapter.  It never
claims that a database is promoted; it only issues a single-use fencing epoch
when a third witness has granted a live lease.
"""
from __future__ import annotations

from dataclasses import dataclass
import time


class QuorumError(ValueError):
    pass


@dataclass(frozen=True)
class WitnessLease:
    holder: str
    epoch: int
    expires_at: float

    def valid_for(self, holder: str, now: float) -> bool:
        return self.holder == holder and self.epoch > 0 and now < self.expires_at


class QuorumGate:
    def __init__(self, *, clock=time.monotonic):
        self._clock = clock
        self._epoch = 0
        self._lease: WitnessLease | None = None
        self._fenced: set[int] = set()

    def grant(self, holder: str, *, ttl_seconds: float) -> WitnessLease:
        if not isinstance(holder, str) or not holder or not holder.isascii():
            raise QuorumError("invalid witness holder")
        if type(ttl_seconds) not in (int, float) or not 0 < ttl_seconds <= 300:
            raise QuorumError("invalid bounded witness lease")
        self._epoch += 1
        self._lease = WitnessLease(holder, self._epoch, self._clock() + ttl_seconds)
        return self._lease

    def authorize_promotion(self, holder: str, epoch: int) -> WitnessLease:
        lease = self._lease
        if lease is None or not lease.valid_for(holder, self._clock()):
            raise QuorumError("witness lease is absent or expired")
        if type(epoch) is not int or epoch != lease.epoch or epoch in self._fenced:
            raise QuorumError("stale or fenced promotion epoch")
        self._fenced.add(epoch)
        return lease

    def authorize_effect(self, holder: str, epoch: int) -> None:
        lease = self._lease
        if lease is None or epoch in self._fenced or not lease.valid_for(holder, self._clock()):
            raise QuorumError("stale primary effect denied")
        if epoch != lease.epoch:
            raise QuorumError("effect epoch does not match witness lease")

