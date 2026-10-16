# PDF 11 — audyt semantyczny źródła względem ROADMAPY OCAF

**Źródło:** „Kompleksowa Roadmapa Wdrożenia Ekosystemu OCAF w Rust.PDF”, 11 stron. **Status:** wymagania przypisane do istniejących etapów; brak dowodu wdrożenia. Materiał zawiera sześć faz oraz demonstracyjny kod, którego nie należy integrować zamiast istniejącego runtime.

| Strona | Ustalenie źródłowe | Etap | Wymaganie odbioru i luka |
|---|---|---|---|
| 1 | Rust/Tokio, Zenoh pub/sub/queryables, SHM, deklarowane sub-ms IPC/LAN, RAM 15–30 MB | 0.46, 0.47, 0.98 | Zmierzyć SHM i interhost osobno, p50/p95/p99, RSS, CPU, hardware, SHA; liczby źródłowe nie są wynikami OCAF |
| 2 | Faza 1: pętla agenta, rejestr Queryable i magistrala | 0.38, 0.46 | Typowane RPC, autoryzacja, timeout, bounded queues, recovery |
| 2 | Faza 2: hypothesize–retrieve–invoke, JSON Schema 2020-12, czyszczenie kontekstu; cel -80% | 0.43, 0.98 | MCP tools/list, podpisane schematy, per-tenant ACL, version pinning, TTL, walidacja, porównawczy benchmark tokenów |
| 2 | Faza 3: #[tool], Firecracker REST/E2B, cgroups v2, cold start <1s | 0.39, 0.55 | Rzeczywisty gość i kontrola uprawnień, izolacja, teardown; nie zastępuje 10 produktów Agents Builder |
| 2 | Faza 4: OAuth 2.1 PKCE, RFC 7636/8414, szyfrowanie, unikalne endpointy per-user | 0.44 | State/PKCE, token refresh/revoke, per-tenant scopes, testy confused deputy |
| 2 | Faza 5: L402/x402, HTTP 402, invoice, macaroon attenuation, budżet | 0.49 | Osobne protokoły, weryfikacja kryptograficzna i realny settlement, atomic ledger, replay |
| 2 | Faza 6: natywny RAG i OWASP ASI | 0.63, 0.67 | E2E ingestion/ACL, prompt injection, poisoning i niezależne testy negatywne |
| 3 | Cargo.toml i ToolSchema/ToolExecutionRequest/Response/L402InvoiceChallenge | 0.43, 0.49 | Typowanie i walidacja; snippet nie jest testem kompilacji z aktualnym Cargo.lock |
| 4–5 | FastToolProvider przechowuje mapę schematów, publikuje cały katalog przez Zenoh Queryable | 0.43, 0.63 | Brak tenant ACL, signed manifest, schematu output, wersji i rate limits; test wycieku katalogu |
| 5–6 | Endpoint execute/* deserializuje JSON; wymaga tylko obecności macaroon/preimage; zwraca SUCCESS bez wykonania narzędzia | 0.38, 0.49, 0.63 | Fail-closed auth, realne wykonanie, weryfikacja narzędzia, schema i efektu; brak automatycznego PASS |
| 6–7 | OcafAgent pobiera pełną mapę z registry/discover i szuka narzędzia po nazwie | 0.43, 0.46 | Nie jest to semantic retrieval; wymaga indeksu i pomiaru payloadu |
| 7–8 | RwLock<u64> budżet, odejmowanie przed płatnością, stały settled_preimage_hash_77492104 | 0.49 | To symulacja, nie płatność; wymagana transakcyjna rezerwacja/rozliczenie/zwrot i trwały ledger |
| 8–9 | Ponowne wywołanie z przykładowym macaroon; brak realnej walidacji preimage i invoice | 0.49, 0.63 | Reject forged preimage, token replay, tenant crossover, wrong invoice i failed settlement |
| 9–10 | main uruchamia przykładowe narzędzie financial_analyzer i wypisuje wynik | 0.38, 0.43 | Nie dowodzi realnego backendu, uprawnień, WASI ani Firecracker |
| 10–11 | Instrukcja cargo build/run; przykładowy log 180 µs, deklaracja 100–350 µs | 0.47, 0.98 | Wymagane uruchomienie na aktualnym main i odtwarzalny benchmark; log nie jest pomiarem |
| 11 | Interpretacja L402 jako pomyślnego rozliczenia 50 sats | 0.49 | Sprzeczne z kodem demonstracyjnym; status UNVERIFIED |

## Reguły integracji

1. Zachować 215 etapów głównych, 64 podetapy i 10 produktów Agents Builder; sześć faz PDF to grupy wymagań, nie nowe numery.
2. Nie kopiować demonstracyjnego src/main.rs ani Cargo.toml do istniejącego runtime. Nie zaliczać przykładowego SUCCESS, wydrukowanego czasu ani odejmowania salda jako rzeczywistego wykonania/płatności.
3. Weryfikacja implementacji wymaga przeglądu rzeczywistego kodu, testów, CI i artefaktów przypiętych do SHA. Status etapów pozostaje bez zmian.
4. W manifeście wymagania PDF11 przypisano do: 0.38, 0.39, 0.43, 0.44, 0.46, 0.47, 0.49, 0.55, 0.63, 0.67, 0.98.

**Wynik:** semantyczna inwentaryzacja 11 stron zakończona; implementacja, rzeczywiste benchmarki i odbiór produkcyjny niepotwierdzone.
