# G01 certification status

**IN_DEVELOPMENT — no independent certification or production deployment.** This package implements a local engineering workflow for a single G01 swarm. A stage marked `COMPLETE_VERIFIED` in its local ledger means that the configured operator signed a structurally valid receipt and the named artifact bytes matched its SHA-256 at handoff and replay. It does not certify that the declared commit built the artifact or that the declared test ran; those are operator review duties.

Integration tests cover dependency order, restart replay, concurrent claims, forged or changed signatures, artifact substitution and mutation, rejected `ACTIVE`/OSTc/source access, wrong operator keys, and store-bound receipts. No live Zenoh, multi-host operation, autonomous AI workflow, source access, OSTc, release acceptance, or 33-swarm completion is claimed. Formal release still requires independently reviewed CI and operator evidence.
