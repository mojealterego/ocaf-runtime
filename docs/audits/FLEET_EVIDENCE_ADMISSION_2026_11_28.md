# Fleet evidence admission — 111 agents / 33 swarms

Date: 2026-11-28. Baseline 99a9193b79bce1a4b62968286b81e27167973a4d. Status TESTED_LOCAL (focused full Python module); no requirement acceptance or deployment approval.

The prior validator treated NOT_RUN, FAIL, and malformed digest strings as evidence of completion, did not validate canonical swarm identities, and could raise exceptions on malformed records. Validate exact canonical agent/swarm ID sets, schema/count types, allowed statuses, PASS build/test values, non-placeholder text, SHA-1 commit and SHA-256 digest syntax, and empty blockers for completed rows. A complete summary cannot override pending rows. The CLI rejects duplicate JSON keys and non-finite constants, limits input to 4 MiB and reports malformed input with exit 1. The CLI success label is PASS_STRUCTURE_ONLY; this is intentionally not proof of execution or authenticity.

Environment: Python 3.13.5 on Linux. Exact complete original source reconstructed from GitHub; Git blob fa83da6b79038ce649cb820c24639c9cbf0c9d43. Patched full source blob 316ed0c08891c0b1e2c557833917ce6f3d67e690. Test blob a10b193294643e70430a8b0dee3fef1a18d9bd3f. Canonical ID sets checked against actual GitHub registries (111/33); local tests use generated evidence fixtures and actual Python CLI subprocess, not actual agent build or deployment.

Command: python3 -m unittest discover -s scripts/tests -p test_evidence_pipeline.py -v.
RED: 7 methods, 21 failure reports and 5 errors (including subtests); exit 1; SHA-256 6ab87a2d458d2071bfb82da1bc70a719b159f43d733108e3b13858d8cfaf274c.
GREEN: 7/7 PASS; exit 0; SHA-256 40734298766bc3c6de2d0b7b0fb2da249c12b77d152e98fb24afc80a8b541260.
Optimized Python: python3 -O -m unittest discover -s scripts/tests -p test_evidence_pipeline.py -v; 7/7 PASS, exit 0; SHA-256 aa163d2a17782b5deb99b3a71d4a60c9ba61eb1ee93b232797eb9388f63d753e.
python3 -m py_compile scripts/evidence_pipeline.py scripts/tests/test_evidence_pipeline.py: exit 0.

Existing fleet structural workflow extended to test both historical baseline documents and negative fixtures. No manual CI dispatch. Local raw logs exported in session. Full repository and actual baseline file regression not run locally (checkout unavailable due DNS). No Rust/Cargo in local runtime.

Limits: structurally valid fabricated receipts can still pass structure validation; this validator does not verify signatures, Git object existence, artifact bytes, member topology, execution logs or receipt issuer. No agent or swarm status was promoted. No external audit requirement introduced. CLI success text changes intentionally; external consumers expecting the former literal PASS may need adjustment.

## Follow-up actual baseline regression (same session)

After implementation commit 8364fddb4e6a182fef6736f1a7de6b97ca9e6371, both historical baseline files were reconstructed locally from their GitHub contents and verified byte-for-byte by Git blob SHA. Each is 87804 bytes. Actual CLI validation in optimized Python 3.13.5 passed for both documents:

- 2026-10-21: c09bb3412407337bd026ab99cb0c27d65cb4a088 — PASS_STRUCTURE_ONLY, exit 0.
- 2026-10-22: 9f4d1ede0804728f71c0e88aa3220c32e662db91 — PASS_STRUCTURE_ONLY, exit 0.

Command for each: python3 -O scripts/evidence_pipeline.py validate --output docs/evidence/FLEET_EVIDENCE_BASELINE_<date>.json. Combined raw log SHA-256 60cd36c86913716967f10a388b1a058fe74303291470e6ef3524b69679dd99ff. This supersedes the earlier local historical-baseline regression limitation, but does not establish full repository regression or actual agent execution.
