# OCAF — rekonstrukcja źródeł PDF 01–09 względem ROADMAPY

**Stan:** PARTIAL_SEMANTIC_RECONCILIATION; **nie jest to zatwierdzenie produkcyjne**. Sprawdzono aktualny manifest `docs/roadmap/canonical_v11_inventory.json` (215 głównych + 64 techniczne) i źródłowe dokumenty PDF dostępne w kontekście projektu. Numeracja 01–09 jest numeracją audytu z wcześniejszych zapisów, a nie gwarantowaną kolejnością nazw plików z bieżącej listy załączników. Rozdzielono materiały historyczne, rewizje i duplikaty. Brak dowodu wykonania kodu i testów oznacza UNVERIFIED.

## Źródła i rozliczenie

| PDF / źródło | Strony źródła | Potwierdzone obowiązki i ograniczenia | Etapy istniejące | Wynik |
|---|---|---|---|---|
| 01 — `Analiza AGI I Ludzkich AI (1).PDF` | 1–4 | Architektury modularne, OpenCog Hyperon, bitemporalna pamięć i proweniencja; referencje do obcych repozytoriów nie są integracjami | 0.67, 1.95, 1.97, 2.04 | COVERED_AS_PLAN; integracja UNVERIFIED |
| 01 | 5–8 | Symulacje społeczne i teoria umysłu wyłącznie opt-in; DSPy holdout i kontrola wycieku danych; AI Scientist z izolacją i zgodą człowieka | 0.83, 1.98, 2.01, 208–211 | COVERED_AS_PLAN |
| 01 | 9–14 | Bounded task agents, GodMode, OpenHands, SWE-bench; benchmarki źródeł nie są benchmarkami OCAF | 2.02–2.06, 212–215 | COVERED_AS_PLAN; E2E UNVERIFIED |
| 02 — `Analiza Architektury Systemu OCAF (1).PDF` | 1–4 | Dynamiczne MCP z krótkim życiem schematów, kontrolą odkrywania; OAuth per tenant/user/session, PKCE/state, refresh/revoke, brak tokenów w LLM | 0.43, 0.44, 0.38 | COVERED_AS_PLAN; negative tests OPEN |
| 02 | 5–7 | Code-as-a-Tool, typowanie, sandbox; egress broker i SSRF/DNS rebinding; L402/x402 z rzeczywistym settlement | 0.39, 0.43, 0.49, 0.55, 0.63 | COVERED_AS_PLAN; execution OPEN |
| 02 | 8–13 | Zagrożenia OWASP agentic, katalogi narzędzi, ograniczenia claims; nie traktować liczb marketingowych jako dowodu | 0.63, 0.98 | EVIDENCE_REQUIRED |
| 03 — rewizja `Analiza Architektury Systemu OCAF` | 1–10 | Obowiązki PDF 02, w tym niezależne testy izolacji, autoryzacji i schematów | 0.39, 0.43–0.49, 0.63 | REPEATED/REFINED; nie duplikować etapów |
| 03 — dodatkowy rozdział Rust/Zenoh/ZKP | 11–13 | Realny transport, proof verification, odróżnienie zero-copy lokalnego od międzyhostowego; bibliografia nie stanowi dowodu | 0.45–0.47, 1.15, 1.41 | COVERED_AS_PLAN |
| 04 — `Architektura OCAF w Języku Rust (1).PDF` | 1–7 | Zenoh, CRDT, OTel, MCP Tree-sitter, L402; kontrakty i testy z realnymi backendami, a nie przykłady | 0.43, 0.46, 0.47, 0.49 | COVERED_AS_PLAN |
| 04 | 8–11 | Aya/eBPF, KVM/Firecracker, ML-KEM/ML-DSA i Loihi; tracing openat nie jest blokadą ransomware, symulacja SNN nie jest Loihi | 0.39, 0.45, 0.91 | ACCEPTANCE_GAPS; HARDWARE_UNVERIFIED |
| 04 | 12–15 | Bibliografia i deklaracje; brak podstaw do automatycznego zaliczenia | powiązane wyżej | REFERENCE_ONLY |
| 05 — `Architektura OCAF w Języku RustPDF_261012_042941.pdf` | 1–15 | Tekstowa rewizja/kopia 04 (15 stron); zachować osobną proweniencję, nie liczyć wymagań podwójnie | jak 04 | DUPLICATE_CONTENT_CANDIDATE |
| 06 — `Dokument bez nazwy (6).PDF` | 1–6 | Rust + Zenoh, tool provider/registry, dynamiczne schematy; `<1 ms` wymaga pomiaru, nie jest wynikiem CI | 0.43, 0.46, 0.47 | COVERED_AS_PLAN |
| 06 | 7–12 | Przykład odejmuje budżet przed settlement i używa `settled_preimage_hash_77492104`; samo pole `macaroon_token` nie dowodzi autoryzacji; brak dowodu rzeczywistego płacenia | 0.44, 0.49, 0.65 | **KRYTYCZNY BRAK DOWODU:** reject fake preimage, reserve/commit/refund, replay/idempotency |
| 06 | 12–14 | Rig, E2B, x402/L402 — rozważyć adaptery dopiero po kontrakcie, sandbox, kontroli kosztów i niezależnych testach | 0.39, 0.43, 0.49, 0.55 | REFERENCE/PLANNED |
| 07 — `Projekt i Mapa Drogowa Rewolucyjnej Architektury S.. (1).PDF` | 1–8 | Rust, rmcp/pmcp, Zenoh, MCP, Firecracker, PQC i Loihi; deklaracje przewag bez benchmarków OCAF | 0.39, 0.43, 0.45–0.49, 0.91 | COVERED_AS_PLAN |
| 07 | 9–12 | Fazy M0–M5, KPI (m.in. throughput 500, latency 35 µs, 10k agentów, cold start 125 ms) to **cele badawcze**, nie osiągnięte wyniki; programy dotacyjne są tłem | 0.46, 0.47, 0.49, 0.91, 0.98 | KPI_NEEDS_REPRODUCIBLE_BENCHMARK |
| 08 — rewizje `Projekt i Mapa Drogowa Rewolucyjnej Architektury S..(1)/(2)` | 1–12 | Rozwinięcie M0–M5, kryteria testowe i warianty źródła; zachować pochodzenie bez powielania etapu | 0.39, 0.43, 0.45–0.49, 0.91 | RECONCILED_AS_REVISIONS |
| 08 — rewizja `...(3).PDF` | 8–9 | JIT/hot-swap oraz M7 wetware, M8 holograficzny stan, M9 topologiczna kryptografia: hipotezy B+R, nie produkcyjne obietnice | 0.81, 0.88–0.93 | RESEARCH_ONLY |
| 08 | 10–14 | Mierzalne KPI, harmonogram audytów, fallback Loihi i ograniczenia sprzętowe; kwoty dotacji wymagają osobnego aktualnego sprawdzenia | 0.91, 0.98 | EVIDENCE_REQUIRED |
| 09 — dodatkowe wymagania AGI/agentic, rejestr wcześniejszego audytu PDF09 | s. 5–11 według wpisów źródłowych w manifeście | Symulacje społeczne, DSPy, Agent-Zero/Hermes, AI Scientist, AutoGPT/BabyAGI, GodMode, OpenHands, SWE-bench | **208–215**, 1.98–2.06 | **ETAPY JUŻ DODANE**; bez dowodu wykonania; numeracja tego wpisu wymaga uzgodnienia z rejestrem oryginalnych plików |

## Wnioski audytorskie

1. **Nie dodawać ponownie** etapów 208–215 ani dziesięciu produktów Agents Builder: istnieją w manifeście. Wykryte wymagania PDF 02–08 są w dużej mierze tematycznie obecne w istniejących etapach; zakres szczegółowych testów i semantyczne pokrycie wymagają kontroli.
2. **Luka bezpieczeństwa z PDF 06 s. 7–10:** pokazany przepływ nie weryfikuje prawdziwego settlement, używa stałego preimage i pomniejsza budżet przed rozliczeniem. W 0.49 wymagane są rzeczywiste dowody invoice/preimage/macaroon, idempotentny ledger i zwrot rezerwacji; żadnego PASS z tego przykładu.
3. **KPI M0–M5** są hipotezami/celami źródłowymi. Nie wpisywać ich jako osiągniętych parametrów. Zaliczenie wymaga wersjonowanego środowiska, powtarzalnych prób, pomiarów i przypisania SHA.
4. **Weryfikacja negatywna** pozostaje otwarta: SSRF, confused deputy, tenant crossing, fake invoice, replay, manifest rug pull, eBPF trace-only, PQC downgrade, Loihi simulation.
5. **Niepełność:** ten dokument nie zastępuje atomowego rejestru każdej normatywnej klauzuli z każdego wariantu źródła. Bez pełnego mapowania strony → obowiązek → ID → kod → test i wynik status pozostaje PARTIAL, a nie COMPLETE. Dokumentacja historyczna PDF 01–09 ma różne numeracje i rewizje; nie utożsamiać numeru pliku z numerem rewizji bez dowodu.

**Źródło prawdy:** aktualny manifest v11 i oryginalne pliki; historyczne raporty `docs/audits/PDF40_01...`, `PDF40_02...`, `PDF40_03...` oraz `OCAF_40_PDF_PAGE_BY_PAGE_ROADMAP_RECONCILIATION_2026-10-16.md`. **Odbiór:** OPEN.
