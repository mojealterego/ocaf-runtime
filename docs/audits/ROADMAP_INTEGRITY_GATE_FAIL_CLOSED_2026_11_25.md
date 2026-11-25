# OCAF — fail-closed Roadmap integrity gate security regression

**Date:** 2026-11-25
**Repository:** mojealterego/ocaf-runtime
**Branch:** main only
**Base HEAD:** b11f763d0c783267f82eeda33c3c3efe857a337d
**Scope:** scripts/roadmap_integrity_gate.py, scripts/tests/test_roadmap_integrity_security.py
**Status:** IMPLEMENTED / TESTED_LOCAL_SCOPED. Full roadmap release gates: NOT_ACCEPTED. production_approved=false.

## Requirements and threat model

Roadmap inventory and optional source.sha256 snapshot must never return PASS when validation fails. The original gate used Python assert expressions: CPython -O removes all such checks. It also accepted empty snapshot manifests, silently overwrote duplicate paths and hashed files outside the repo or reached through symlinks. A fabricated partial snapshot is not proof of complete historical append-only provenance.

## TDD evidence

Environment: Linux x86_64, CPython 3.13.5, standard library unittest, no external network, isolated tempfile fixtures, no production state mutation.

RED against exact original `scripts/roadmap_integrity_gate.py` from base main:
- Command: `python3 -m unittest discover -s scripts/tests -p test_roadmap_integrity_security.py -v`
- 9 executed; **7 FAIL, 2 PASS**. Confirmed invalid count and duplicate IDs pass under python -O, empty/duplicate SHA-256 snapshot entries accepted, path traversal and symlink accepted, noncanonical stage ID accepted.

GREEN after fixing the actual Python CLI:
- `python3 -m unittest discover -s scripts/tests -p test_roadmap_integrity_security.py -v` → **15/15 PASS**
- `python3 -O -m unittest discover -s scripts/tests -p test_roadmap_integrity_security.py -q` → **15/15 PASS**
- `python3 -m py_compile scripts/roadmap_integrity_gate.py scripts/tests/test_roadmap_integrity_security.py` → **PASS**
- `python3 -m compileall -q scripts` on isolated patch directory → **PASS**

Individual test IDs are the fifteen `test_*` methods inside `RoadmapIntegritySecurityTests`. Conditions include valid manifests in normal/optimized Python, corruption of count/order/ID, invalid JSON, malformed/wrong SHA-256, empty and duplicate records, missing file, path traversal, symlinks in target or ancestor, and positive snapshot receipt.

## Source digests

- `scripts/roadmap_integrity_gate.py`: SHA-256 `79f6bf9613b566f3e018eb641b8db2a2ff9b315fca53854f8867f1991ef1db05`
- `scripts/tests/test_roadmap_integrity_security.py`: SHA-256 `f17d7a1a3643c92fe39a103baa076d806abe21200e8c543c72c5a7f3d5e12d0c`

## Scope limitations and release decision

- Local executions were run against the exact source/test bytes in an isolated patch directory. The complete OCAF repository was not cloned in this environment (Git remote DNS unavailable). No Rust compiler or full dependency graph was available here.
- The real canonical-v11 manifest was inspected through the GitHub connector and found to have 334 numeric, unique, ordered IDs. Running the patched CLI over the complete checkout remains pending.
- Historical `append_only` is backward-compatible as an ordered-unique-ID structural field only. New field `historical_append_only_verified=false` explicitly states a prior trusted signed baseline has **not** been verified.
- A snapshot PASS validates only the paths listed in its source.sha256, not completeness of the repository inventory or independent provenance.
- Snapshot path checks reject ordinary traversal and static symlink attacks; this is not a proof of OS-level TOCTOU race resistance with hostile concurrent filesystem mutation.
- Existing `.github/workflows/rust.yml` runs `python3 -m unittest discover -s scripts/tests -v` and therefore includes the added suite when CI executes. No workflow run on the new exact HEAD has been observed.
- This security increment does not close the 334 main roadmap stages, Agents Builder GATE-10, GATE-09, release, deployment, or production certification.
