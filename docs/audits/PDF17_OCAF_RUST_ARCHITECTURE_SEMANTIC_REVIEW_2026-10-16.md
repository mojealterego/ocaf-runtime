# PDF17 — audyt semantyczny OCAF (2026-10-16)

Źródło: `Analiza Architektury Systemu OCAF(3).PDF`, 18 stron (włącznie z bibliografią), 51 pozycji bibliografii. Status: wymagania zidentyfikowane, wdrożenie UNVERIFIED.

| Strony | Obowiązek/element | Przypisanie do istniejącej roadmapy | Luka i dowód odbiorowy |
|---|---|---|---|
| 1–3 | Zenoh pub/sub, queryable, SHM TypedLayout/Typed i zero-copy | 0.46, 0.47 | Kompatybilność Zenoh 1.x, kontrola inicjalizacji SHM, ACL i fallback, benchmark p50/p95/p99; liczby 35 µs i 40 Gbps niezweryfikowane |
| 3–4 | Automerge/Yrs CRDT, DAG, Bloom/delta, offline merge | 0.46, 0.47 | Konwergencja przy partycjach, trwałość, tenant isolation, brak CRDT dla autorytatywnych płatności |
| 4–5 | OTLP, tracing-opentelemetry, instrumentowanie tokio::spawn | 0.42, 0.47 | Eksport do realnego collectora, ciągłość trace_id, redakcja sekretów |
| 5–6 | Dynamic MCP, Tree-sitter/type-sitter, JSON Schema i ASI01/ASI02 | 0.43, 0.44, 0.63 | Parser CST i pojedyncza sygnatura nie są pełną walidacją schematu ani obroną przed injection; testy adversarial |
| 6–8 | OAuth 2.1 PKCE, izolacja per-user, ZKP Arkworks/Halo2 | 0.44, 0.45, 0.63 | AuthZ per tenant, rotacja, powiązanie public inputs z caveats; pokazany obwód dowodzi tylko równości |
| 8–9 | Firecracker, Jailer, cgroups v2, broker sekretów, Aya eBPF | 0.39, 0.55, 0.63 | Real KVM/guest, host eBPF nie widzi bezpośrednio syscalli gościa, brak implementacji detekcji ransomware i egzekucji SIGKILL |
| 9–10 | HTTP402/L402, Lightning, Macaroons, x402, offline budgets | 0.49 | Prawdziwe settlement, ledger, idempotencja, limit kosztów i explicit approval; kod używa mocked_ i fikcyjnego preimage |
| 10–11 | RAG z katalogów, chunking, embeddingi, OWASP ASI | 0.64, 0.67, 0.98 | Weryfikacja ACL/provenance i rzeczywistego embedding/retrieval; kod zwraca stały wektor |
| 11–12 | ML-KEM/ML-DSA, rustls/aws-lc-rs, Loihi 2 SNN | 0.45, 0.91 | Dowód negotiated hybrid KEX, certyfikaty/podpisy, rzeczywisty HIL i CPU fallback |
| 13 | Sześć faz wdrożenia i kryteria <1ms, -80% tokenów, start <1s | 0.39, 0.43–0.49, 0.63–0.67, 0.91, 0.98 | Zdefiniować baseline, metodykę, testy E2E; nie uznawać deklaracji za benchmark |
| 13–15 | Cargo.toml i przykładowy OcafAgent::discover_tool | 0.43, 0.46, 0.49 | Szkielet tylko discovery, nie ma realnego invoke, auth, płatności; sprawdzić API i build |
| 15–16 | **NOWE względem PDF16:** porównanie Claude Code, Codex, Cursor, OpenHands, Devin oraz inspiracje OpenHands/ACP i Aider/Tree-sitter repo map | 0.43, 0.44, 0.50, 0.51, 0.55, 0.63, 0.67, 0.98 | Dopisać adapter Agent Client Protocol (ACP) z negotiation/authZ/sandbox i contract/E2E tests; repo map Tree-sitter z symbol graph, rankingiem kontekstu, token budget i testami poprawności; ewaluacja SWE-bench Verified z odtwarzalną wersją datasetu, baseline i kosztami. Wyniki procentowe w tabeli są niezweryfikowanymi twierdzeniami dokumentu |
| 17–18 | Podsumowanie i bibliografia | powyższe | Nie stanowi dowodu wdrożenia ani benchmarku |

## Różnica PDF17 względem PDF16

PDF17 ma 18 stron, a nie 17. Strony 1–15 w dużej mierze pokrywają PDF16. Istotne nowe wymagania na stronach 15–16: porównanie coding agents, protokół ACP jako interoperacyjny adapter zewnętrznych agentów, repo-map ranking wzorowany na Aider oraz odtwarzalna ewaluacja coding-agent na SWE-bench. Nie utożsamiać nazwy ACP z istniejącym MCP ani referencyjnego OpenHands/Aider z działającą integracją.

## Kryteria

Nie renumerować historycznych etapów. Nie tworzyć nowego etapu bez dowodu, że istniejące zakresy nie obejmują wymagań. Status: PLANNED/UNVERIFIED. Brak uruchomionych testów kodu, brak zatwierdzenia produkcyjnego.
