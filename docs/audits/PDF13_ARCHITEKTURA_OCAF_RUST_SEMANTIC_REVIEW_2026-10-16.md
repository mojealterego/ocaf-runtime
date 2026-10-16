# PDF13 — Architektura OCAF w Języku Rust: audyt semantyczny

Źródło: `Architektura OCAF w Języku Rust.PDF` (15 stron). Dokument opisuje propozycje; sam nie potwierdza implementacji OCAF.

| Strony | Wymaganie | Etapy | Weryfikacja |
|---|---|---|---|
| 1–2 | Zenoh pub/sub, query, typed SHM zero-copy, waitable provider | 0.46, 0.98 | Benchmarki SHM/LAN, profilowanie, backpressure i tenant isolation |
| 2–3 | Automerge/Yrs CRDT, offline-first, DAG, delta/Bloom sync | 0.46, 0.67 | Partition/recovery/convergence; nie używać CRDT dla fencing/quorum |
| 3 | OpenTelemetry OTLP batch, tracing, Tokio span, sampling | 0.47 | Cross-host trace continuity i redakcja sekretów |
| 3–5 | L402, Macaroons, preimage, offline ledger, ZKP Arkworks/Halo2, ZKPoU | 0.49, 0.64 | Kryptograficzna walidacja, limity długu, replay, idempotencja; ZKP jako R&D |
| 5–7 | MCP tools/list, Tree-sitter CST, type-sitter, dynamic schema routing | 0.43, 0.63 | Schema fuzzing, injection tests, round-trip code transforms |
| 6–7 | OAuth PKCE, per-user MCP endpoint, scoped secret broker | 0.44 | Confused deputy, CSRF, SSRF, fail-closed |
| 7–8 | Firecracker/Jailer/seccomp/cgroups, Aya/eBPF, anomaly/self-healing | 0.39, 0.63 | Real KVM, isolation negatives, checkpoint recovery; eBPF host has limited guest visibility |
| 8–10 | FIPS 203 ML-KEM, FIPS 204 ML-DSA, hybrid rustls/aws-lc-rs TLS | 0.44, 0.64, 0.98 | Compatibility, downgrade, memory and performance; no implied FIPS certification |
| 10–11 | Loihi 2 SNN/LIF/STDP, rust-snns/ferromorphic, hardware-in-the-loop | 0.42, 0.98 | Hardware access, CPU/GPU baseline, latency/energy and cost |
| 11–12 | Synthesis of nine architecture modules | 0.39, 0.43, 0.46, 0.47, 0.49, 0.64, 0.67, 0.98 | Assertions are not integration evidence |
| 13–15 | 75 references | all | Verify primary sources, versions, and reproducibility |

## Krytyczne rozbieżności

- Źródło używa czasu dokonanego dla integracji bez wskazania SHA, kodu, CI i środowiska wykonania.
- CRDT eventual consistency nie zastępuje linearizowalnych uprawnień, lease, fencing i transakcyjnego ledger.
- Offline L402 z odroczonym długiem wymaga dodatkowego modelu ryzyka, limitów i rozliczeń.
- Host-side eBPF nie obserwuje automatycznie całego jądra guest VM.
- Wyniki Zenoh 5–35 mikrosekund, 67 Gbps i Loihi 2 nie są pomiarami OCAF.
- ZKP, ZKPoU, post-quantum TLS i Loihi 2 nie są potwierdzone jako wdrożone.

Status: AUDITED / REQUIREMENTS DOCUMENTED. Nie oznacza TESTED, DEPLOYED ani OPERATOR_ACCEPTED.
