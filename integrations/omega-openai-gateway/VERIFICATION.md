# OMEGA Gateway v0.6.0 — scoped engineering verification (2026-11-24)

## Results
- Locally-XXX `978def21408fadb641723233085de4f298224de4`: `node --test test/omega-local.test.mjs` — 4 passed. Mocked HTTP. Source and relevant dependencies matched Git blob hashes.
- Omega-X-Evolution- `b55f3d7fbc0ef3475f8173544dd5a0431b6e26ef`: `PYTHONPATH=. python -m pytest -q tests/test_omega_gateway_provider.py` — 1 passed. Mocked factory. Source/test matched Git blob hashes.
- Digital-Life `7315e8d9dafeaa8f47bc3d4de117cde91cd4d50d`: `PYTHONPATH=src python -m pytest -q tests/test_omega_gateway_provider.py` — 5 passed. Mocked HTTP. Source/test/model contract/GGUF dependency matched Git blob hashes.
- SQLite algorithm engineering simulation: four scenarios passed (budget limit, legacy mismatch, persistence/changed limits, concurrency). 32 attempts under a 10-unit limit and 3-unit charge yielded three reservations totaling nine units. This is an independent Python sqlite3 implementation of transaction logic, not native Rust verification.
- OCAF root head `e921fcfcc19e83ef4004053ba941ad295478223b`: existing Rust CI reports failure (https://github.com/mojealterego/ocaf-runtime/actions/runs/45448235969). No native Rust compiler available locally; direct clone blocked by DNS. No successful Rust build claimed.
- No OpenAI API requests, model inference, or deployment.

## Release status
Gateway 31 authored Rust tests plus three root OMAt staging tests remain unexecuted locally. Dedicated CI resolves dependencies for testing; a reviewed, committed lockfile and successful native CI are required for reproducible release. No full regression executed. Operator reservation estimates are not a guaranteed invoice ceiling. Full duplex audio, automatic adapter registration, actual OMAt executor and OSTc admission, GraphRAG ingestion, arbitrary MCP, and live provider validation remain open. Staged tool is disabled and existing capability boundaries unchanged.

## v0.9.0 / v0.9.1 continuation — 2026-11-24
GraphRAG, durable snapshot and local embedding adapter; explicit one-shot MCP
profiles; multi-turn PCM WebSocket; experimental audio-only WebRTC signalling
and persistent hangup fencing are in source. They are NOT natively compiled.

Fresh executed evidence:
- Python local ingestion client: 5/5 unit tests, RED import failure before
  implementation, GREEN after implementation; Git blobs verified identical:
  b144a39f8a79672dc8a172a21e36b786a95a23de and
  b95cea48e59d10112f6dec9191a8f72f66be38fe.
- Loopback provider fixture self-test: 3/3, script Git blob
  a70ea76bdee545bdd95239566a6191fd4b996315. This tests the fixture ONLY.
- Python py_compile passed for client, tests and fixture script.
- Prior scoped adapter results retained above; not fresh regression results.
- Native compiler/cargo missing from available execution environment; dedicated
  Gateway CI for d29c70834ec75dd4f9ac263ca9e197ab6de85a1d failed:
  https://github.com/mojealterego/ocaf-runtime/actions/runs/45450285803
- Native fixture contract command was added to CI, but has NOT run successfully.
  No lockfile generated, no real provider credentials used, no deployment.

Release remains BLOCKED pending native compile, lockfile review, unit and
integration regression, formatting/Clippy and live provider validation when
approved. OMAt remains staged behind OSTc; existing app adapters require
operator wiring. WebRTC media is negotiated by native clients; no device-side
capture/playback implementation or external hard spend guarantee. No 1.0 tag.
