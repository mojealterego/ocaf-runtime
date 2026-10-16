# PDF15 — Architektura OCAF w Języku Rust(1): audyt 20 stron

Źródło: 20 stron. Wymagania nie dowodzą wdrożenia.

| Strony | Zakres | Etapy |
|---|---|---|
| 1–2 | Rust, Tokio, Zenoh pub/sub/queryables, typed SHM, waitable provider; deklarowane 5–35 µs/67 Gbps | 0.46, 0.98 |
| 3 | Automerge/Yrs CRDT DAG, Bloom/delta, offline; OTLP batch and Tokio instrument | 0.46, 0.47, 0.67 |
| 4–5 | Lost in the Middle, hypothesize-retrieve-invoke, MCP tools/list, Tree-sitter CST/type-sitter | 0.43, 0.98 |
| 5–6 | OAuth PKCE, per-user endpoints, isolated secrets, Arkworks/Halo2 ZKP and ZKPoU | 0.44, 0.64 |
| 6–7 | Code-as-a-Tool, Firecracker KVM, guest broker, Aya/eBPF | 0.39, 0.55, 0.63 |
| 7–8 | L402, Macaroons, preimage, offline CRDT debt | 0.49 |
| 8–9 | RAG ingestion, OWASP ASI, FIPS 203/204, Loihi 2 | 0.42, 0.63, 0.64, 0.67 |
| 9–10 | Six phases and performance targets; aspirational, not verified | 0.39, 0.43, 0.44, 0.46, 0.49, 0.67, 0.98 |
| 11–12 | Cargo.toml, tool schemas, Zenoh session, in-memory registry | 0.43, 0.46 |
| 12–14 | Queryable discovery/execution; fake invoice and Macaroon, no schema or ACL checks, unconditional SUCCESS | 0.38, 0.43, 0.49, 0.63 |
| 14–16 | Client discovery, in-memory budget debit, hardcoded fake preimage, no refund/settlement/replay protection | 0.49, 0.63 |
| 16–17 | Demo financial tool and cargo build instructions; no real financial execution or test output | 0.43, 0.98 |
| 18–20 | 60 citations; require source/version verification | all |

## Krytyczne luki

1. Nie istnieje w demonstracji weryfikacja kryptograficzna preimage ani Macaroon; samo Some(...) otwiera wykonanie.
2. Odpowiedź SUCCESS jedynie odsyła argumenty, nie uruchamia narzędzia.
3. Brak uwierzytelnienia tenantów, izolacji rejestru, autoryzacji, limitów zasobów, walidacji JSON Schema, timeoutów i idempotencji.
4. In-memory budżet jest zmniejszany przed faktyczną płatnością, bez rollbacku i transakcyjnego ledger.
5. Kod Zenoh API 1.10 nie został skompilowany w ramach audytu; nazwy metod wymagają walidacji na repo.
6. Host eBPF nie daje automatycznie widoczności guest kernel. CRDT nie zastępuje quorum, fencing ani transakcyjnego ledger.
7. Twierdzenia o 80% redukcji kontekstu, <1ms, 67 Gbps, ZKP, PQC i Loihi 2 nie są dowodami uzyskanymi z OCAF.

Status: REQUIREMENTS DOCUMENTED; nie TESTED, DEPLOYED ani OPERATOR_ACCEPTED.
