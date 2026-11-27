# AB-OM-01 / 0.51 — bounded local HTTP connection admission

Date: 2026-11-27. Baseline 0afa334777a421852202c249a23f75b6486a4024. Status TESTED_LOCAL_PARTIAL.

The server previously spawned one unbounded worker per accepted TCP connection. Bound worker admission to 16 concurrent connections per server (class-level override, validated 1..128), reject overload before thread creation and request parsing with best-effort HTTP 503, Retry-After 1 and bounded send timeout. Release permits in the worker finally block and when worker startup fails. This is a process-local bound, not a client rate limit.

Evidence: actual Linux TCP connections and HTTP handler, Python 3.13.5, deterministic event barriers, recording backend, injected thread startup failure. The exact source section from parse_request_json to before main was reconstructed and verified against GitHub: baseline SHA-1 03202c1c319beebb56f53e893588b07d754639dc, 8952 bytes; changed SHA-1 4f63bd818db4c14f5b77104cb9ca56b442dda497, 10300 bytes. Local harness supplies imports, RunnerError, MAX_REQUEST, and minimal static allowlist for index.html. Full module was not locally imported or compiled. Test file imports the full module when run in the repository.
Baseline Git blob 36b194bb942b713962460dc2d8769ecb70bb02d4; modified full source blob a82a73a9987d261f5bbb68ebe08b9d395811b2f6; tested test blob bee6c444c2a7ffa70f520d3eec8c2e49df8990f7.

Command: python3 -m unittest discover -s scripts/tests -p test_local_runner_connection_budget.py -v
RED: 4 methods, 2 PASS, 1 FAIL, 1 ERROR; exit 1. SHA-256 878079d63aa87241ee7bdd14b11d8f9dfbcee603c53e36483d4965bb30a41ab6.
GREEN: 4 PASS, 0 FAIL, 0 ERROR; exit 0. SHA-256 e699d1f11b02b2ee3e402dbcb4bc720ed87eb5f34c53ac6b02bbc79dd2cabb79.
python3 -m py_compile apps/agents-builder-web/local_runner.py scripts/tests/test_local_runner_connection_budget.py on reconstructed module: exit 0.

GREEN cases:
- test_invalid_connection_budget_rejected
- test_malformed_http_releases_slot
- test_overload_rejected_and_slot_recovers
- test_worker_start_failure_does_not_leak_admission_slot

RED overload test timed out reading a response; invalid budget was not rejected. Existing workflow extended. Raw logs retained locally; no manual CI dispatch.

Remaining: no full module/regression, Rust/Cargo absent, direct Git clone DNS failure; actual llama inference and browser not tested. Saturated connections may see a reset rather than 503, the accept loop may spend up to 0.1s attempting overload response, and the worker limit does not establish total body/header deadlines or reserve a slot for stop. No canonical requirement or production status promotion.
