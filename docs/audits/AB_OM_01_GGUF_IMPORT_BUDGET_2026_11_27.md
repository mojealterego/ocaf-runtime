# AB-OM-01 / 0.51 — GGUF import resource admission

Date: 2026-11-27. Base commit 3ea096c712fe50e81b87a36f63df590ef3405d75. TESTED_LOCAL_PARTIAL.

Changes: one concurrent import per LocalRunner via independent bounded semaphore, HTTP 429 before reading the competing upload, slot release on success/failure, controlled 507 on temporary-file creation failure, strict read chunk type/length admission. Existing atomic no-overwrite publication and SHA-256 receipt retained. Independent import and inference slots remain intentional.

Evidence: Python 3.13.5, Linux, real filesystem and synchronized worker thread, injected disk failure and nonconforming stream. Exact LocalRunner class prefix (constructor through import_model) reconstructed and SHA-1 matched against GitHub; minimal module harness for imports, regex and RunnerError, not full module import. Baseline full blob 9d5ca79b93296c756423e53b1fb6603795feef55; original section SHA-1 a909e4e549dca8aee0acf36d2d760b8d109f71d7; fixed section 775f01ec178e04e112380048cd43f7c19ff30ff8; test blob 7ca4c0d188ceb1f14b6258501613cd637a465e2c; patched full source blob 36b194bb942b713962460dc2d8769ecb70bb02d4.

Command: python3 -m unittest discover -s scripts/tests -p test_local_runner_import_budget.py -v.
RED: 2 PASS, 2 FAIL, 1 ERROR, exit 1. SHA-256 7bbdaa0c0937d003673e3d90feab560e3c5d56b8f370e8ab4624d131a15f5e91.
GREEN: 5/5 PASS, exit 0. SHA-256 741a1ebe7c50840f271d7138abcb967c144342e2559aaecc53c2b552d09315a4.
python3 -m py_compile reconstructed module and test file: exit 0. Logs local, not included. Existing CI workflow extended; no manual dispatch.

Limits: one process-local upload slot, not a cross-process quota; total disk quota, slow-client deadlines, cleanup failure recovery and model launch TOCTOU still open. Full module, repository regression, Rust and actual llama inference not executed. No requirement or release status promoted.
