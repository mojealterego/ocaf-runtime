# TRS mathematical implementation — source-to-code evidence ledger

This document distinguishes *computable finite models* from research claims.
Source basis: "Raport Badawczy: Architektura Teorii Rezonansu
Strukturalnego (TRS)" within "ONTOLOGIA LICZBY I GEOMETRII:
Dowód Problemów Milenijnych" (source document contains extensive
additional material). The names below preserve the author's terminology.

| TRS concept | Implementation | Scope | Evidence / remaining gap |
|---|---|---|---|
| 3-SAT spin penalty and energy landscape | `src/trs_math.rs::Clause3::penalty`, `enumerate_3sat` | Exact finite truth-table baseline | Five source-related unit tests in code; Rust execution not yet observed |
| Topological loop invariant beta_1 | `graph_betti_one` | Undirected finite 1-dimensional cell complexes | NOT an invariant of a SAT complexity class |
| Diffusion / statistical relaxation matrix B | `graph_heat_step` | Explicit Euler discretization of graph Laplacian | NOT a proof that Q=sqrt(B) is unitary |
| TRS-Key / Topological Seal | `src/trs_seal.rs`, `src/trs_seal_sqlite.rs` | HMAC authenticated envelope and replay/fencing | Conventional cryptography; NOT a novel proven topological cryptosystem |
| Ricci-flow routing | `crates/ocaf-transport-crdt/src/ricci_routing.rs` | Routing heuristic | NOT continuous geometric Ricci flow on a Kähler manifold |
| Operator H_Omega, adelic Hilbert space, Sonin space | Not implemented | Research | Requires operator domain, self-adjoint extension and verified spectrum |
| Kähler-Fisher-Rao geometry, Hodge Laplacian | Not implemented | Research | Requires explicit metric, connection, boundary conditions |
| BSD elliptic-curve L-functions | Not implemented | Research | Requires arithmetic definitions and numerical validation |
| P vs NP / RH / Hodge / BSD proofs | Not established | Open mathematical claims | Code experiments cannot substitute rigorous proofs |
| Topological cryptography | Not implemented as secure cryptosystem | Research | Requires threat model, key generation, reductions, cryptanalysis |

## Acceptance policy

1. Every formula must have an exact source reference and explicit domain.
2. Numerical algorithms need deterministic fixtures, convergence checks,
   nonfinite input handling and comparison with independent baselines.
3. Mathematical hypotheses are labelled as hypotheses until independently
   checkable proofs exist.
4. Production security uses established primitives and separate key authority.
5. No blanket '100% implemented' claim without compilation, integration
   tests, mathematical validation and release evidence.

## Commands (not yet executed in this work session)

```sh
cargo test --lib trs_math -- --nocapture
cargo test --lib trs_seal -- --nocapture
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```
