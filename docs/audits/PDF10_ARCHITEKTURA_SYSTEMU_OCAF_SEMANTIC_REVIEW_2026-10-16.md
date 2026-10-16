# PDF 10 — audyt semantyczny względem ROADMAPY OCAF

Źródło: **Analiza Architektury Systemu OCAF.PDF**, 13 stron, tytuł „Architektura OmniContext Agent Framework (OCAF): Paradygmaty, Skalowalność i Bezpieczeństwo Autonomicznych Systemów AI”. Audyt opiera się na tekście rzeczywiście załączonego dokumentu. **Status: wymagania przypisane do istniejących etapów; implementacja i testy NIEPOTWIERDZONE.**

| Strona | Obowiązek wynikający z PDF | Etap | Rozstrzygnięcie / dowód wymagany |
|---|---|---|---|
| 1 | Kontekst, opis problemów: przeciążenie schematami, auth, katalogi i M2M | 0.43, 0.44, 0.49 | Wprowadzenie; deklaracje autora nie są dowodem wdrożenia |
| 2 | Lost in the Middle; tabela GeoEngine 19/46 narzędzi, 8K/16K, 85%/<50%, 17/20/30 s, 22/26/27 W | 0.98 | Dane przypisane do źródła zewnętrznego; nie przenosić jako wyniku OCAF; powtórzyć benchmark |
| 2–3 | Dynamic Tool Loading; hypothesize–retrieve–invoke, MCP JSON-RPC tools/list, JSON Schema 2020-12 inputSchema i opcjonalne outputSchema, usuwanie schematów z kontekstu | 0.43 | Rozdzielić zwykłe tools/list od semantycznego wyszukiwania; ACL, podpis, wersja, TTL/revoke, cache invalidation, E2E i porównanie statycznego/dynamicznego routingu |
| 3–4 | Confused Deputy, consent/client_id mixup, OAuth 2.1, PKCE, step-up, szyfrowane tokeny, dedykowane endpointy per user | 0.44 | Tenant/user/session binding, callback state/nonce, refresh/revoke, brak sekretów w LLM, testy A→B i log redaction; unikalny URL nie zastępuje autoryzacji |
| 5 | Code-as-a-Tool Python/TypeScript @tool, introspekcja typów/docstrings, schemat MCP; zagrożenie RCE | 0.55, 0.39 | Odrzucać nieobsługiwane typy, złośliwe docstrings; wykonywać tylko w izolowanym, podpisanym sandboxie; przykładowy dekorator nie jest implementacją |
| 6 | Firecracker MicroVM, efemeryczny lifecycle, guest kernel, teardown, sekretowy broker po egress | 0.39 | Dowody prawdziwego gościa, timeoutów, teardown, fail-closed, SSRF/DNS rebinding; poświadczenia tylko do zaufanego HTTPS, nie do gościa |
| 7 | Publiczne i prywatne rejestry MCP, Glama/Smithery/mcp.so, twierdzenie o 950k węzłów | 0.43, 0.98 | Zaufanie do manifestów, reputacja, provenance i odwołanie; liczba 950k bez audytowanego inwentarza jest deklaracją autora |
| 7–8 | L402/x402, HTTP 402 challenge, płatność, invoice/preimage, Macaroon attenuation, budżet M2M | 0.49 | Oddzielić Lightning L402 od x402, autentyczna weryfikacja i settlement, trwały ledger reserve/commit/refund, idempotencja, replay i cross-tenant negative tests |
| 8 | Enterprise RAG: rekurencyjne drzewa plików, PDF/logi/kod, chunking/overlap, embedding asynchroniczny | 0.67 | Proweniencja, izolacja tenantów, ACL, symlinks, delete/reindex, odporność na zatruty dokument i bounded traversal |
| 9 | ASI01 goal hijack przez pośrednie prompt injection; ASI02 tool misuse i HITL | 0.63 | Nie ufać treści narzędzi; testy exfiltration, deny i zatwierdzenia przed efektami ubocznymi |
| 10 | ASI03 identity/privilege abuse, ASI04 rug-pull/supply-chain, ASI05 RCE | 0.63, 0.44, 0.39 | Negatywne testy token swap, manifest mutation, nieautoryzowany kod, izolacja gościa |
| 11 | ASI06–ASI10 jedynie wspomniane zbiorczo, bez indywidualnych definicji lub kryteriów; podsumowanie | 0.63 | **Nie dopisywać wymyślonych treści źródła**; uzupełniać niezależną, wersjonowaną matrycą OWASP i jawnie oznaczyć pochodzenie |
| 12–13 | Bibliografia i odnośniki | — | Referencje nie dowodzą wdrożenia ani kompatybilności z deklarowaną datą specyfikacji MCP |

## Wynik

- Uzupełniono w istniejących etapach: **0.39, 0.43, 0.44, 0.49, 0.55, 0.63, 0.67, 0.98**.
- Nie utworzono nowego etapu: zakres mieści się w istniejących identyfikatorach, których nie wolno przenumerowywać.
- Zsynchronizowano `ROADMAP.md`, `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, pierwszą tabelę `README.md` i `docs/roadmap/canonical_v11_inventory.json`.
- Status implementacji: **UNVERIFIED**. Brak podstaw do oznaczenia PASS dla guest WASI, prawdziwego settlement, OAuth end-to-end lub benchmarków. Nie uruchomiono CI w ramach tego audytu.
