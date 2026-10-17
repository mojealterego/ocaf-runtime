#!/usr/bin/env python3
"""Fail-closed broker lifecycle state machine for stage 0.39.8.

The state machine is deliberately independent from a VM launcher.  A concrete
Firecracker/eBPF adapter must provide the observed facts before each transition;
no transition fabricates a running VM or an attached program.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class LifecycleError(ValueError):
    pass


class State(str, Enum):
    CLAIMED = "claimed"
    JAILER_READY = "jailer_ready"
    EGRESS_ENFORCED = "egress_enforced"
    VM_RUNNING = "vm_running"
    RECEIPT_BOUND = "receipt_bound"
    RELEASED = "released"
    FAILED = "failed"


@dataclass
class Lifecycle:
    workload_id: str
    state: State = State.CLAIMED
    receipt_digest: str | None = None

    def _require(self, expected: State) -> None:
        if self.state is not expected:
            raise LifecycleError(f"invalid transition from {self.state.value}")

    def jailer_ready(self, *, immutable_assets: bool) -> None:
        self._require(State.CLAIMED)
        if not immutable_assets:
            raise LifecycleError("jailer assets are not immutable")
        self.state = State.JAILER_READY

    def enforce_egress(self, *, ebpf_program_id: int) -> None:
        self._require(State.JAILER_READY)
        if type(ebpf_program_id) is not int or ebpf_program_id <= 0:
            raise LifecycleError("a verified eBPF program id is required")
        self.state = State.EGRESS_ENFORCED

    def vm_started(self, *, vm_id: str, cgroup_inode: int) -> None:
        self._require(State.EGRESS_ENFORCED)
        if not vm_id or type(cgroup_inode) is not int or cgroup_inode <= 0:
            raise LifecycleError("kernel VM and cgroup observations are required")
        self.state = State.VM_RUNNING

    def bind_receipt(self, *, receipt_digest: str, exit_code: int) -> None:
        self._require(State.VM_RUNNING)
        if len(receipt_digest) != 64 or any(c not in "0123456789abcdef" for c in receipt_digest):
            raise LifecycleError("receipt digest must be a lowercase SHA-256")
        if type(exit_code) is not int:
            raise LifecycleError("guest exit code is required")
        self.receipt_digest = receipt_digest
        self.state = State.RECEIPT_BOUND

    def release(self) -> None:
        self._require(State.RECEIPT_BOUND)
        self.state = State.RELEASED

    def fail_closed(self) -> None:
        if self.state is State.RELEASED:
            raise LifecycleError("released workload cannot be resurrected")
        self.state = State.FAILED

