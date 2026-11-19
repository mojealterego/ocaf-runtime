# KaiOS Builder Cloud lifecycle hardening — 2026-11-19

Base main: a581f082fa02a53d1a7ab463edd2a4d6de3e391f
Verified baseline Git blobs:
- cloud.js: 72e124d99d64904bd71aa8f50c9da589ca101ab8
- canonical-graph.js: bc8f7228eefbe9127fd7920a0b6f6f32a0f9226b
- cloud.test.cjs: c988af5f8f5196967bf1597214eab68de2de9eb0

A timed-out request could release the single-flight guard while an older
engine's Fetch was still active. Explicit disconnect did not settle a
stalled operation until network completion/timeout. A stalled streaming
reader could also remain un-cancelled. The client now permanently revokes
its session on timeout, clears its stored token, and requires reconnection
and project reload before another write. Explicit disconnect settles the
pending promise immediately. Stream cancellation is attempted regardless of
AbortController availability. There is no automatic retry. Already-sent
requests may still commit on servers; this does not assert remote rollback
or cancellation on engines that cannot abort Fetch.

Executed local deterministic tests on Linux, Node 22.16.0 with actual
graph and transport source, controlled Fetch, timer and reader adapters:

    node --test apps/agents-builder-kaios/tests/cloud.test.cjs

RED: 10/13 pass, 3 failures, exit 1.
GREEN: 13/13 pass, 0 failures, 0 skips, exit 0.
New regression cases: timeout revokes client after uncertain CAS;
disconnect settles stalled Fetch; timeout cancels stalled reader.
node --check cloud.js and git diff --check PASS.

Verified final Git blobs:
- cloud.js: 3ac7c17166fae0e2cd6cb24af3c98d78247e9de6
- cloud.test.cjs: 88a0ee0052a00b515d4eaf56c590174cf91c3f69

SHA-256:
- f572a45d51876cded6ff8997de7070aaa8ed7a019de172e4f1158539de3b65d1 cloud.js
- c76076eafb22630e25eddfa03865a48cb3ccbf22c6527cc867c7615d264c0fa8 cloud.test.cjs

SCOPED_PASS: transport regression tests, not physical KaiOS E2E.
Full UI suite, physical device installation, live authenticated HTTPS,
release signatures and GATE-10 remain unverified. No manual paid CI or
new infrastructure launched.
