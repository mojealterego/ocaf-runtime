# OCAF Repository Steward — integrity gate trigger coverage

Date: 2026-11-25
Repo: mojealterego/ocaf-runtime; branch: main.
Base commit: b3a43969d8b2f9329aa97f0462e7ea90eb4857f5
Scope: .github/workflows/ocaf-repository-steward.yml and scripts/tests/test_steward_trigger_coverage.py
Status: CONFIG_IMPLEMENTED / TESTED_LOCAL_STATIC. CI_UNVERIFIED. production_approved=false.

## Contract and risk

Changes to scripts/roadmap_integrity_gate.py or its security test must trigger the dedicated repository steward workflow for both pushes on main and pull requests. The governance job must execute the security regressions before trusting its canonical inventory result. Existing read-only permissions, branch policy, and no-release rules stay unchanged.

The previous GitHub workflow omitted both paths in both event path lists. Because GitHub Actions only triggers matching paths, a future validator-only change might bypass this dedicated gate. The general Rust CI workflow does run all Python tests on every main push, but that does not substitute for the dedicated steward trigger.

## Evidence (real local tests, not remote Actions execution)

Environment: isolated Linux x86_64, CPython 3.13.5, no network, source bytes obtained from the exact GitHub main revision via connector.

- TDD RED: `python3 -m unittest discover -s scripts/tests -p test_steward_trigger_coverage.py -v` → **3 FAIL / 3 executed**, confirming both missing path-filter sets and missing job test commands.
- GREEN: same command after config change → **3 PASS / 3 executed**.
- Regression: `python3 -m unittest discover -s scripts/tests -p test_roadmap_integrity_security.py -q` → **15 PASS / 15 executed**.
- Syntax: `python3 -m py_compile scripts/tests/test_steward_trigger_coverage.py` → PASS.

Source SHA-256:
- .github/workflows/ocaf-repository-steward.yml: `0cf2ad4c80b47ca4df9c8c33c725e92ba9879ea4e8bd31fb873b501b078e0764`
- scripts/tests/test_steward_trigger_coverage.py: `b86846be0399e24d1722cc0c87ec8c581d22f49285a51b1679fe7eedf36039e2`

## Release boundary

Static workflow inspection and local Python tests do not prove successful execution by GitHub Actions. Billing/runner access may block actual jobs; inspect the exact HEAD's run state and annotations before any CI PASS. No full Rust/Cargo build, no deployment, no production release, no closure of Agents Builder or GATE-09/GATE-10.
