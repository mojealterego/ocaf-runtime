#!/usr/bin/env python3
"""Reproducible Z3 evidence for the *bounded* OCAF stage 334 authorization model.

UNSAT proves the SMT formula is inconsistent, not that the Rust implementation
is formally verified. A missing solver, unknown, timeout or malformed output
is an explicit failure. Never report Imandra verification from this gate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MODELS = {
    "tenant_isolation.smt2": "unsat",
    "attenuation_no_escalation.smt2": "unsat",
    "replay_uniqueness.smt2": "unsat",
    "negative_control_sat.smt2": "sat",
}


class SolverEvidenceError(RuntimeError):
    pass


def verify_result(output: str, expected: str) -> None:
    if output.strip() != expected:
        raise SolverEvidenceError(
            "solver returned unknown, unexpected or inconclusive result"
        )


def check_models(solver: str = "z3", root: Path = ROOT) -> dict:
    resolved = shutil.which(solver)
    if resolved is None:
        raise SolverEvidenceError("Z3 is not installed")
    try:
        version = subprocess.run(
            [resolved, "-version"], capture_output=True, text=True,
            check=True, timeout=10,
        ).stdout.strip()
    except (subprocess.SubprocessError, OSError) as error:
        raise SolverEvidenceError("Z3 version unavailable") from error
    if not version.startswith("Z3 version "):
        raise SolverEvidenceError("unexpected solver identity")
    cases = []
    for name, expected in MODELS.items():
        path = root / "verification" / "sasos334" / name
        try:
            content = path.read_bytes()
            if len(content) > 100_000 or not content:
                raise SolverEvidenceError("SMT input size invalid")
            completed = subprocess.run(
                [resolved, "-smt2", str(path)],
                capture_output=True, text=True, timeout=15, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise SolverEvidenceError("solver unavailable or timed out") from error
        if completed.returncode != 0 or completed.stderr.strip():
            raise SolverEvidenceError("solver rejected the SMT model")
        verify_result(completed.stdout, expected)
        cases.append({
            "file": name,
            "sha256": hashlib.sha256(content).hexdigest(),
            "result": completed.stdout.strip(),
            "expected": expected,
        })
    return {
        "status": "BOUNDED_SMT_MODEL_CHECKED_NOT_RUST_FORMAL_PROOF",
        "solver": version,
        "cases": cases,
        "rust_implementation_formally_verified": False,
        "imandra_verified": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    try:
        report = check_models()
        encoded = json.dumps(report, sort_keys=True, indent=2) + "\n"
        if args.report:
            args.report.write_text(encoded, encoding="utf-8")
        print(encoded)
        return 0
    except (SolverEvidenceError, ValueError) as error:
        print(f"SASOS_SMT_FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
