# AB-OM-01 / stage 0.51 — recoverable local process shutdown

Date: 2026-11-27. Base commit 297e3e21e3fe6b78f303f76ac4ec4c47edf35e4e. Status TESTED_LOCAL_PARTIAL; no release approval.

Previously shutdown cleared all process ownership before terminate/wait/kill. A signal error or second timeout could therefore lose the live process handle and let start admit another backend. Retain ownership until wait or poll confirms exit. On unconfirmed shutdown return RunnerError 503 with retryable retained state. Handle concurrent process exit. Preserve the existing 4-second terminate wait and 4-second kill wait. No new dependencies.

Evidence: Python 3.13.5, isolated Linux, actual subprocess.Popen test with a Python sleeping child, plus five deterministic process doubles. Local tests execute exact _stop_locked and stop methods reconstructed from GitHub, with minimal module harness (imports, RunnerError, LocalRunner). Full module not locally imported. Baseline full source blob 13601834912011b4cd4c64c9dfdd76483a9cc6e9; original section SHA-1 8e2a68d64cc3c214531c59b7842293f3b7d8883a; fixed section e8e193bad32e9875ba538f53c4dafdb18d342312; test blob 65dee84e8e3853f96856f3175c4dd831a6a502e6; modified full source blob f564bd6aaabd6d09a4045d3b450d453d6fcd2378.

Command: python3 -m unittest discover -s scripts/tests -p test_local_runner_shutdown.py -v
RED: 2 PASS, 4 ERROR, exit 1. SHA-256 5d190a8b5b4a39a288dd9f039df333bed1378ae1b63e801c988222aa1e330613.
GREEN: 6/6 PASS, exit 0. SHA-256 9d0a5e26de45202a34d15b02878c984d1fc67b53d8f94da5b65681e6091df712.
python3 -m py_compile on reconstructed module: exit 0. Logs retained in local temporary directory, not committed.
Existing workflow extended. No manual CI dispatch. GitHub DNS resolution unavailable in local execution environment; Rust/Cargo not installed.

Remaining: actual llama process and child process group cleanup, active inference cancellation, full-module and full-repository regression, production verification. Retaining state on failed stop allows retry but does not prohibit inference if the original backend remains alive. No status promotion or external audit.
