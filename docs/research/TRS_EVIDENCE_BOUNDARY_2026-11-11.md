# TRS research intake — evidence boundary (2026-11-11)

Source: user-supplied 24-page report **Architektura Teorii Rezonansu Strukturalnego (TRS)**, especially sections 9–13. This is a research hypothesis, **not a validated cryptographic specification or mathematical proof**.

## Mathematical correctness blockers

1. Section 10.1's spin penalties are reversed. For sigma_i = 1 - 2*x_i, clause C1 = (x1 OR x2 OR NOT x3) is violated at (sigma1,sigma2,sigma3)=(+1,+1,-1), so its correct indicator is (1+sigma1)(1+sigma2)(1-sigma3)/8, not the formula printed. C2 = (NOT x1 OR x3 OR x4) is violated at (-1,+1,+1), so its correct indicator is (1-sigma1)(1+sigma3)(1+sigma4)/8.
2. The document's negative off-diagonal covariance (-1/256) does **not** imply negative Ricci curvature or nontrivial first homology. A positive-semidefinite covariance matrix can have negative off-diagonal entries.
3. The selected two-clause 3-SAT formula is satisfiable, e.g. x1=1, x3=1. The asserted unavoidable contradictory cycle is not established.
4. The displayed Perelman W functional in section 10.7 omits its standard curvature, gradient and normalization terms; the subsequent singularity claim is not proved.
5. Self-adjointness of a chosen operator alone does not identify its spectrum with the nontrivial Riemann zeta zeros. Section 12 does not establish that spectral correspondence.
6. The BSD rank equality is restated via standing-wave metaphors, without a theorem connecting the proposed modes to Mordell-Weil rank. The report does not prove BSD.
7. Section 13's 'topological no-cloning' is not a digital anti-copy primitive: classical bits can be copied, and neither Betti numbers nor Euler characteristic establish unforgeability.
8. Section 9's Q=sqrt(B) does not in general imply Q is unitary when B is a heat diffusion matrix.

## Engineering decision

- Implement **TRS-SEAL v1** as a clearly labelled conventional HMAC-SHA256 authentication envelope in `src/trs_seal.rs`, not as a novel public-key scheme.
- Bind the tag to protocol domain, tenant, sender, key epoch, fencing epoch, sequence and payload.
- Keep key epoch and authoritative fencing epoch independent. Epoch updates require authenticated external authority; never infer leadership from a received message.
- Fail closed on invalid MAC, unknown epoch, replay, capacity exhaustion, or fencing mismatch.
- Do not claim post-quantum security, encryption, non-copyability, persistent replay protection or production readiness.
- Pending work: durable replay index and key/epoch authority, HKDF context separation and secret-provider integration, integration tests, fuzzing, formal protocol model, CI evidence, vetted PQC/hybrid handshake.

## Traceability

| Source | Claim | Classification | Action |
| --- | --- | --- | --- |
| 9–10 | Kähler/Fisher metric, 3-SAT penalties | Contains demonstrable algebraic errors | Research-only; correct formulas before any prototype |
| 11–12 | BSD and RH proofs | Unproven spectral and rank correspondences | No production dependency |
| 13.1–13.3 | Topological key and unclonable seal | No standard security game or reduction | Do not deploy as cryptography |
| 13.4 | Seal init/create/verify interface | Useful interface concept | Conventional authenticated envelope implemented |
