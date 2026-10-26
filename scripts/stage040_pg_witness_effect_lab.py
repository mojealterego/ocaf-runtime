#!/usr/bin/env python3
"""One-postmaster PostgreSQL lab for a majority-gated effect adapter.

This checks a real SQL write after quorum admission and proves that missing
votes or a stale epoch prevent this adapter from issuing SQL. It does not
fence raw SQL clients, provide independent witness processes, or prove HA.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
import tempfile
import time
from typing import Callable, Sequence

from stage040_quorum_certificate import (
    CertificateError, QuorumCertificateGate, WitnessVote, issue_vote,
)


TOKEN = re.compile(r"[a-f0-9]{32}\Z")
IDENTITY = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}\Z")


class LabError(ValueError):
    pass


def admitted_effect(gate: QuorumCertificateGate, votes: Sequence[WitnessVote],
                    *, candidate: str, nonce: str,
                    execute: Callable[[int, str, str], None]) -> int:
    """Reject before invoking the SQL adapter unless majority authorizes it."""
    if not IDENTITY.fullmatch(candidate) or not TOKEN.fullmatch(nonce):
        raise LabError("invalid candidate or bounded event nonce")
    if not isinstance(votes, Sequence) or isinstance(votes, (str, bytes)):
        raise LabError("typed witness vote sequence required")
    if any(not isinstance(vote, WitnessVote) or vote.candidate_id != candidate
           for vote in votes):
        raise LabError("certificate authorizes a different PostgreSQL writer")
    receipt = gate.admit(votes)
    if receipt.candidate_id != candidate:
        raise LabError("certificate authorizes a different PostgreSQL writer")
    execute(receipt.epoch, candidate, nonce)
    return receipt.epoch


def command(argv: list[str], *, timeout: int = 30) -> str:
    completed = subprocess.run(argv, capture_output=True, text=True,
                               check=False, timeout=timeout)
    if completed.returncode:
        raise LabError("PostgreSQL command failed: " + argv[0] + ": " +
                       completed.stderr[-500:])
    return completed.stdout.strip()


def run_lab(pg_bin: Path) -> dict:
    if (os.geteuid() == 0 or not pg_bin.is_absolute() or pg_bin.is_symlink()
            or any(not (pg_bin/name).is_file() for name in ("initdb", "pg_ctl", "psql"))):
        raise LabError("nonroot PostgreSQL binary directory required")
    with tempfile.TemporaryDirectory(prefix="ocaf40-witness-pg-") as tmp:
        root = Path(tmp)
        data = root/"pg"
        data.mkdir(mode=0o700)
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        initdb, pgctl, psql = (str(pg_bin/name) for name in ("initdb", "pg_ctl", "psql"))
        command([initdb, "-D", str(data), "--auth-local=trust", "--auth-host=trust",
                 "--no-instructions"], timeout=45)
        started = False
        try:
            command([pgctl, "-D", str(data), "-o", f"-p {port} -h 127.0.0.1 -k {root}",
                     "-l", str(root/"postgres.log"), "-w", "start"])
            started = True
            def sql(statement: str) -> str:
                return command([psql, "-X", "-q", "-A", "-t", "-v", "ON_ERROR_STOP=1",
                                "-h", "127.0.0.1", "-p", str(port), "-U", "postgres",
                                "-d", "postgres", "-c", statement])
            sql("CREATE TABLE ocaf40_guarded_events (epoch bigint NOT NULL, "
                "candidate text NOT NULL, nonce text PRIMARY KEY)")
            keys = {name: secrets.token_bytes(32) for name in ("w1", "w2", "w3")}
            gate = QuorumCertificateGate(root/"quorum.sqlite", cluster_id="pg-lab",
                                         witness_keys=keys)
            try:
                def votes(candidate: str, epoch: int, witnesses: tuple[str, ...]):
                    now = time.monotonic()
                    return [issue_vote(witness_id=name, key=keys[name],
                                       cluster_id="pg-lab", candidate_id=candidate,
                                       epoch=epoch, lease_id=f"lease-{epoch}",
                                       issued_at=now, expires_at=now+20)
                            for name in witnesses]
                def execute(epoch: int, candidate: str, nonce: str) -> None:
                    # The three interpolated values have already been reduced
                    # to a bounded integer and strict ASCII identifier/hex.
                    sql(f"INSERT INTO ocaf40_guarded_events VALUES "
                        f"({epoch},'{candidate}','{nonce}')")
                a_nonce = secrets.token_hex(16)
                admitted_effect(gate, votes("node-a", 1, ("w1", "w2")),
                                candidate="node-a", nonce=a_nonce, execute=execute)
                if sql("SELECT count(*) FROM ocaf40_guarded_events") != "1":
                    raise LabError("first majority-gated SQL write missing")
                attempted = False
                def track(epoch: int, candidate: str, nonce: str) -> None:
                    nonlocal attempted
                    attempted = True
                    execute(epoch, candidate, nonce)
                try:
                    admitted_effect(gate, votes("node-a", 1, ("w1",)),
                                    candidate="node-a", nonce=secrets.token_hex(16),
                                    execute=track)
                except CertificateError:
                    pass
                else:
                    raise LabError("minority witness partition allowed SQL write")
                if attempted or sql("SELECT count(*) FROM ocaf40_guarded_events") != "1":
                    raise LabError("SQL reached during witness partition")
                admitted_effect(gate, votes("node-b", 2, ("w2", "w3")),
                                candidate="node-b", nonce=secrets.token_hex(16),
                                execute=execute)
                attempted = False
                try:
                    admitted_effect(gate, votes("node-a", 1, ("w1", "w2")),
                                    candidate="node-a", nonce=secrets.token_hex(16),
                                    execute=track)
                except CertificateError:
                    pass
                else:
                    raise LabError("old writer epoch allowed SQL after promotion")
                if attempted or sql("SELECT count(*) FROM ocaf40_guarded_events") != "2":
                    raise LabError("stale writer reached SQL after epoch advance")
                rows = sql("SELECT epoch||':'||candidate FROM ocaf40_guarded_events ORDER BY epoch")
                if rows != "1:node-a\n2:node-b":
                    raise LabError("actual PostgreSQL rows diverged from certificate order")
                return {
                    "status": "ONE_POSTMASTER_MAJORITY_EFFECT_ADAPTER_LAB_PASS",
                    "postgresql_rows": rows.splitlines(),
                    "majority_vote_required_before_sql": True,
                    "minority_partition_sql_not_invoked": True,
                    "stale_epoch_sql_not_invoked": True,
                    "postgresql_server_side_fencing_proved": False,
                    "independent_witness_hosts_tested": False,
                    "three_host_ha_tested": False,
                    "stage_040_complete": False,
                    "production_approved": False,
                }
            finally:
                gate.close()
        finally:
            if started:
                command([pgctl, "-D", str(data), "-m", "immediate", "-w", "stop"])


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pg-bin-dir", type=Path, required=True)
    p.add_argument("--report", type=Path, required=True)
    p.add_argument("--ack-one-postmaster-lab", action="store_true")
    x = p.parse_args()
    if not x.ack_one_postmaster_lab or not x.report.is_absolute() or x.report.exists():
        p.error("explicit one-postmaster lab and new absolute report required")
    x.report.write_text(json.dumps(run_lab(x.pg_bin_dir), sort_keys=True, indent=2)+"\n")


if __name__ == "__main__":
    main()
