# AB-OM-01 / stage 0.51 — local process generation and backend JSON

Date: 2026-11-27. Baseline c2524c4e849bd2a0684d76ef1789ca02c0109a6e. TESTED_LOCAL_PARTIAL.

Health observations from replaced, stopped or exited processes must not be reported as ready. Capture the process, settings, port, key and model together under lock; recheck process identity and exit status after health and before returning inference output. Strict backend JSON reuses the existing parser (duplicate keys, non-finite values, isolated surrogates, depth). Return 502 for invalid backend data, 409 for changed generation; release the inference semaphore on all paths.

Local Python 3.13.5, deterministic process/HTTP doubles with synchronous fault injection. Tests execute the exact status, chat and JSON parser sections extracted from the baseline; local harness provides imports, constants and RunnerError. Not a full module import, real network, subprocess, llama model or concurrent load test. Source section SHA-1 before: status 7a8f2ebabb9e8d090c2e37f791aa2d2a43db7da3, chat cd443c428af340de0d680147a39dbc35c468d5ec, parser 5226a33dfccd519adfbb5c09988b58754902e788; after: status 71da6b75ad45af263bc46bb691971df993b90fc8, chat 37ff791982870e903390c624093617e64b62ceb2. Test Git blob d422515e21dfc9f631f6f5440a46527cecd23805; modified full source Git blob 13601834912011b4cd4c64c9dfdd76483a9cc6e9.

Command: python -m unittest discover -s scripts/tests -p test_local_runner_generation.py -v.
RED: 9 test methods executed, 9 failure reports (including subtests), 1 error, exit 1. GREEN: 9/9 PASS, exit 0. Local reconstructed module py_compile exit 0.
RED log SHA-256 8036ef0f190cfe20fc282cbae90bffab0efbc5d293234e25c850395cb59c6d00.
GREEN log SHA-256 0213a67a8e8bc25f16b31a2c04ace1df7b2a4a192b709ab9bb8dadd5ad4223ec.
Logs local, not embedded in Git. Existing workflow extended; no manual paid CI dispatch.

Remaining limitations: a process may still stop after pre-dispatch check; in-flight requests are not synchronously canceled, and only successful stale results are explicitly suppressed. Port binding race, shutdown/reaping and actual inference remain distinct tasks. No stage acceptance, native build, full regression or production approval.
