# PDF16 — audyt semantyczny i uzgodnienie roadmapy (2026-10-16)

Źródło: `Analiza Architektury Systemu OCAF(2).PDF`, 17 stron. Zakres: strony 1–17, w tym tabela sześciu faz na s. 13, demonstracyjny kod na s. 14–15 oraz bibliografia s. 16–17.

| Strony | Wymaganie źródłowe | Etap istniejący | Weryfikacja i otwarta luka |
|---|---|---|---|
| 1–3 | Zenoh pub/sub, query/reply, SHM TypedLayout, zero-copy, wydajność | 0.46, 0.47 | Test SHM host-local, fallback sieciowy, ACL, p50/p95/p99 i backpressure; liczby 35 µs/40 Gbps są deklaracją źródła |
| 3–4 | Automerge/Yrs, offline-first, DAG, delta/Bloom | 0.46, 0.47 | Testy konwergencji, konfliktów, partycji, replay i izolacji tenantów |
| 4–5 | OTLP, tracing i propagacja spanów przez tokio::spawn | 0.42, 0.47 | Potwierdzić eksport do kolektora, trace continuity, sampling i redakcję danych |
| 5–6 | Dynamic MCP, JSON Schema, Tree-sitter/type-sitter, poison metadata | 0.43, 0.44, 0.63 | Parsowanie CST nie jest walidacją JSON Schema ani ochroną przed prompt injection; wymagane testy semantyczne i negatywne |
| 6–8 | OAuth PKCE, per-user scope, AES-256, ZKP Arkworks/Halo2/Groth16 | 0.44, 0.45, 0.63 | Ochrona sekretów, token rotation, dowód i weryfikator z wiążącymi public inputs; przykład obwodu sprawdza wyłącznie równość |
| 8–9 | Firecracker/Jailer, broker sekretów, Aya/eBPF, wykrywanie ransomware | 0.39, 0.55, 0.63 | Prawdziwy guest KVM, guest/host telemetry boundary, egzekucja polityki; pokazany kod nie implementuje detekcji ani SIGKILL |
| 9–10 | L402/Lightning/Macaroons, offline cache i budżety | 0.49 | Faktyczne invoice settlement, idempotent ledger, limity wydatków, brak automatycznych płatności bez autoryzacji; przykład używa mocked_* i fikcyjnego preimage |
| 10–11 | RAG: asynchroniczny odczyt, chunking, wektory, OWASP ASI | 0.64, 0.67, 0.98 | Kontrola ACL, provenance, prompt injection, embedding/retrieval E2E; funkcja zwraca stały wektor 0.5 |
| 11–12 | PQC ML-KEM/ML-DSA, hybrydowy TLS, Loihi2 SNN | 0.45, 0.91 | Handshake interoperability, weryfikacja algorytmów, HIL na prawdziwym sprzęcie, CPU fallback; konfiguracja default provider nie dowodzi wynegocjowania PQC |
| 13 | Sześć faz wdrożenia: Zenoh, MCP, sandbox, OAuth, L402/x402, RAG/ASI | 0.39, 0.43, 0.44, 0.46, 0.49, 0.63, 0.64, 0.67, 0.98 | Kryteria <1 ms, -80% kontekstu, start <1 s wymagają benchmarków i baz odniesienia |
| 14–15 | Przykładowy Cargo.toml i OcafAgent z discover_tool | 0.43, 0.46, 0.49 | Szkielet nie wykonuje tool invocation, płatności ani zabezpieczonego routingu; nie integrować bez walidacji wersji Zenoh |
| 16–17 | Podsumowanie i 47 pozycji bibliografii | etapy powyżej | Źródła nie są dowodami działania w repozytorium; weryfikować wersje API i twierdzenia |

## Wynik porównania
Główne obszary były już przypisane w ROADMAP do istniejących etapów. Doprecyzowano wymagania dotyczące: propagacji trace context przez tokio::spawn, negatywnych testów semantycznych metadanych MCP, formalnej semantyki ZKP caveats, granicy obserwacji eBPF host/guest, realnego settlement L402, weryfikacji negocjacji hybrydowego TLS i pomiarów sześciu faz.

Nie dodano etapu 216: żaden nowy niezależny produkt/obszar nie wynika z PDF16. Status wszystkich nowych kryteriów: PLANNED / UNVERIFIED. Ten audyt dokumentacyjny nie dowodzi wdrożenia, testów, CI ani odbioru operatora.
