# PDF 02/40 — Analiza Architektury Systemu OCAF (1).PDF

Źródło: 13 stron, 7 rozdziałów i bibliografia. Ocena zakresu Roadmapy, **nie** audyt działania runtime. Numery stron są numerami fizycznych stron PDF.

| Strona | Materiał źródłowy | Etapy istniejące | Wynik porównania |
|---|---|---|---|
| 1 | Cel OCAF: dynamiczne MCP, zarządzane sesje OAuth, MicroVM, HTTP 402; kontekst i bezpieczeństwo | 0.43, 0.44, 0.39, 0.49 | Pokrycie architektoniczne; brak dowodu wdrożenia |
| 2 | Lost-in-the-Middle, BFCL/GeoEngine, koszt kontekstu; hipotetyczne wskaźniki jakości | 0.43, 0.69, 0.98 | **Luka kryterium odbioru:** porównanie statycznych i dynamicznych schematów na wersjonowanym zestawie testowym |
| 3 | Hypothesize-retrieve-invoke, tools/list, inputSchema/outputSchema, wyładowanie schematu po użyciu; confused deputy | 0.43, 0.44 | **Luka kontraktu:** cache/TTL/revocation schematów, odmowa nieautoryzowanego odkrywania |
| 4 | Managed OAuth, PKCE, step-up, szyfrowanie i izolacja tokenów, endpoint per user | 0.44, 0.38 | **Luka kontraktu:** binding tenant/user/session, token refresh/revoke, CSRF/state, nonce, zakaz tokenów w logach/LLM |
| 5 | Tabela autoryzacji; dekorator @tool, introspekcja typów/docstrings i publikacja jako MCP | 0.43, 0.50, 0.55 | **Luka funkcji:** bezpieczny adapter Code-as-a-Tool Python/TypeScript z jawnym typowaniem i odrzucaniem nieobsługiwanych sygnatur |
| 6 | Firecracker MicroVM, lifecycle, timeout, izolacja jądra i sieci, broker sekretów dla egress | 0.39, 0.55 | **Luka granicy bezpieczeństwa:** proxy HTTP z wstrzykiwaniem poświadczeń wyłącznie po walidacji destination i capability |
| 7 | Zewnętrzne rejestry MCP, metadane, skala katalogów; L402/x402 | 0.43, 0.49, 0.87 | **Luka bezpieczeństwa:** provenance/poisoning/rug pull na aktualizacji manifestu |
| 8 | HTTP 402, faktura, budżet, macaroon/preimage, attenuation; enterprise ingestion RAG | 0.49, 1.53–1.54, 1.85–1.87, 0.48 | Pokrycie tematyczne; konieczne rozdzielenie L402 od x402 i weryfikacja rozliczenia |
| 9 | Rekurencyjny ingest PDF/log/kod, chunking/overlap, asynchroniczna indeksacja; ASI01 prompt injection | 0.48, 0.43, 0.63 | **Luka kryterium odbioru:** testy provenance, ACL i propagacji usunięć przy reindeksacji |
| 10 | ASI02 tool misuse, ASI03 identity/privilege, ASI04 supply-chain i rug pull | 0.44, 0.63, 0.83, 0.43 | **Luka testowa:** scenariusze negatywne injection/TOCTOU/tenant-crossing/manifest swap |
| 11 | ASI05 RCE, ASI06–10 omówione zbiorczo; podsumowanie | 0.39, 0.63, 0.83 | Wymagane oddzielne przypadki testowe; nie przypisywać pełnego pokrycia ASI06–10 na podstawie skrótu |
| 12 | Bibliografia 9–36, linki do specyfikacji MCP i źródeł marketingowych | 0.43, 0.44, 0.49 | Bibliografia; wersje i fakty wymagają niezależnej weryfikacji |
| 13 | Dokończenie bibliografii 36–37 | 0.63 | Bibliografia, brak samodzielnego obowiązku |

## Wymagania doprecyzowujące istniejące etapy

- **0.43 (MCP):** dynamiczne odkrywanie i krótkotrwałe dołączanie schematów z ograniczonym TTL, pinningiem wersji i cofnięciem uprawnień; podpisany manifest, kontrola zmian/rug-pull; testy porównawcze dokładności i kosztu kontekstu.
- **0.44 (OAuth):** per-tenant/per-user/session binding, PKCE/state, step-up, refresh/revoke, log redaction, negatywne testy confused deputy. Dedykowany logiczny endpoint nie zastępuje niezależnej autoryzacji każdego wywołania.
- **0.39 (MicroVM):** outbound egress proxy/broker poświadczeń, allowlist hosta i ścieżki, capability check, DNS rebinding/SSRF resistance, brak sekretów w gościu i izolacja przy awarii brokera.
- **0.43 / 0.55 (Code-as-a-Tool):** kontrolowana introspekcja Python/TypeScript i generacja typowanego MCP schema, bez automatycznego wykonywania docstrings jako instrukcji; sandbox i testy odrzucania niejednoznacznych sygnatur.
- **0.48 (RAG):** recursive ingest PDF/log/source, ACL i provenance per chunk, incremental indexing, delete/revoke propagation, chunk-boundary tests, quarantine untrusted content.
- **0.49 / 1.85 / 1.86 / 1.87:** ścisłe oddzielenie L402 (Lightning/preimage/macaroons) od x402 (inne szyny i dowody płatności), testy replay, idempotencji, limitu budżetu i potwierdzenia settlement.
- **0.63 (security):** reprodukowalna macierz ASI01–ASI10, ale tylko po zweryfikowaniu autentycznej wersji taksonomii; testy prompt injection, tool misuse, identity spoofing, supply chain, RCE, memory poisoning, multi-agent privilege escalation.

## Uwagi krytyczne

PDF używa sformułowań „OCAF implementuje” oraz podaje liczby dotyczące 950 tys. serwerów, BFCL, wydajności MicroVM i transakcji. **To twierdzenia autora źródła, nie dowody z repozytorium ani wyniki testów.** Nie wolno też przyjąć, że HTTP proxy „wstrzykuje zaszyfrowane autoryzacje do nagłówków TCP/IP”: wymagany jest precyzyjny kontrakt warstwy HTTP/TLS, host allowlist i brak ekspozycji sekretów w gościu. Macaroon i preimage nie potwierdzają automatycznie uczciwości dostawcy narzędzia. Wersję specyfikacji MCP oraz OWASP ASI należy zweryfikować względem pierwotnych źródeł, a nie cytowanych blogów.

**Decyzja numeracyjna:** wskazane braki są doprecyzowaniem istniejących etapów 0.39, 0.43, 0.44, 0.48, 0.49, 0.55, 0.63 i 1.85–1.87; **nie ma podstaw do dodania 2.07 wyłącznie na podstawie PDF 02**. Status implementacji i odbioru pozostaje niezweryfikowany.
