# Common evolution contract — DGM / RSI

The shared Rust core maintains a bounded lineage of candidate proposals, SHA-256 parent-linked identifiers, separately evaluated safety/regression flags, a retained archive and strict-score RSI selection. DGM-style exploration uses alternate archive parents; RSI refines the current incumbent. Maximum 32 rounds, 16 KiB proposal, scoring 0–1000, baseline and hard safety gates. Failed evaluations cannot win; rejected proposals remain inspectable. No candidate is deployed.

The host must provide a trusted model proposer and independently controlled evaluator with immutable benchmark and safety policy. A callback result is not a cryptographic attestation. Persistent archives, actual source transformation/build sandbox, independent benchmarks, signed admission and rollback via OM Engine are separate unfinished integrations. The algorithm is a bounded DGM/RSI kernel for draft proposals, not end-to-end recursive self-modification or verified capability gains.

Release always DENY until actual integration, OSTc/PQC, evaluation and operator approval.

## 2026-11-20 — owner-controlled evolution

DGM/RSI may work on authorized plaintext Rust sources inside the isolated owner-controlled workspace; distribution encryption/obfuscation is not an iteration precondition. This is not an authorization to export protected sources, bypass OMAs/OMAt, alter the evaluator or deploy candidates. External proprietary release requires OSTc/PQC and all existing admission, signature and license gates. Internal execution admission remains separately unimplemented.
