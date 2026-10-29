# Audyt semantyczny i integracja wymagań PDF Drive — OAuth, SASOS, 555 usług

**Źródło:** [Google Drive](https://drive.google.com/drive/folders/1EKb08OvnajBbhiBLiuWuhwInUy4odc19), plik `Architektura OCAF_ OAuth i Usługi - Google Gemini.PDF.pdf`, 28 stron A4, SHA-256 `e3c74e0b5b0f68fa31f5c91ac6bb446cdf495e1b66cd724cc66953d099c1a8ee`.
**Repozytorium:** `mojealterego/ocaf-runtime`, wyłącznie `main`. **Status:** SOURCE_REVIEWED / PARTIAL_PRIMITIVE; nie wdrożenie produkcyjne.

## Krytyczna korekta numeracji

Numery **0.45–0.49 z PDF nie odpowiadają numerom kanonicznej Roadmapy**. Zachowujemy historyczne etapy: 0.45 PQC/ML-KEM/ML-DSA, 0.46 Zenoh SHM/CRDT, 0.47 telemetria, 0.48 enterprise data, 0.49 ekonomia L402/x402. Zapis wymagań do 0.45–0.49 pod innymi nazwami naruszałby zgodność manifestu v11. Nowa bramka aktywacji to **334**; wspólne wymagania są przypisane do istniejących etapów.

| PDF | Wymaganie normatywne | Kanoniczne etapy | Zweryfikowany stan / bramka |
| --- | --- | --- | --- |
| s.1–3 | Aktywny dyspozytor native/WASI/MicroVM/MCP/API; 555 usług | **334**, 0.43, 0.39 | `src/omat_server.rs` ma admission i fail-closed executor, `src/runtime/mod.rs` ma prawdziwy `ToolExecutor`, lecz brak 555 backendów i OSTc. Wymagany adapter realnego procesu z revocation i E2E; 555 wpisów pozostaje `CATALOG_ONLY`. |
| s.4–9 | OAuth/OIDC PKCE S256, state/nonce, MFA, consent, refresh/revoke, Vault | **0.44** | Istnieją `src/stage044_oauth.rs`, `stage044_oidc.rs`, `stage044_vault.rs`, `stage044_provider_refresh.rs`, `ocaf-oauth-gateway`. Brak dowodu pełnego flow na docelowym IdP + Vault + ingress. Nie zastępować działającego kodu PDF-em. |
| s.10–12 | HMAC capability tickets, caveats, PPID, ochrona BOLA | **334**, 0.07, 0.43, 1.51 | Dodano `src/sasos_capability.rs`: związek tenant/subject/resource, MAC obejmuje wszystkie pola, zawężanie bytes/expiry/verbs, weryfikacja MAC i testy negatywne. **Nie jest podłączony do OMAt**; brak trwałego revocation/replay. |
| s.13–14 | Wasmtime fuel/memory, Firecracker KVM, zeroize, brak sieci | **0.38**, **0.39**, 329, 334 | Istnieją kontrakty sandbox i testy 0.39; PDF nie wykonuje rzeczywistego Wasmtime ani Firecracker. Wymagane realne guest E2E, seccomp, broker, limity i obserwowalne czyszczenie zasobów. |
| s.15–17 | Zenoh SHM, Merkle DAG, CRDT, Bloom, konwergencja | **0.46**, 1.15, 1.18, 1.19, 327, 334 | `crates/ocaf-transport-crdt` istnieje, ale deklaracja zero-copy między hostami przez przesłanie wskaźnika nie jest prawdziwym SHM. Test dwóch hostów, konwergencja po partycji, fencing stanu autorytatywnego, fałszywe trafienia Bloom. |
| s.18–20 | BMC/ImandraX, AST/SMT, LTN i ślady kontrprzykładów | **328**, 326, 334 | Bez rzeczywistego solvera i dowodu związania artefaktu nie ma formalnego PASS. Testy: deadlock ABBA, błędne finally, reentrancy, nieznany wynik i timeout => INCONCLUSIVE/DENY. |
| s.21–28 | Titans test-time memory, NICE/R3Mem lossless, RQGM AST | **327**, 322, 326, 328, 334 | Brak potwierdzenia realnej nauki wag, odwracalności arbitralnych bajtów i poprawnej walidacji mutacji AST. Wymagane osobne testy jakości, round-trip danych, trwałości, provenance, regresji i rollback. |

## Odrzucone fragmenty kodu źródłowego

1. **Pozorne wykonanie:** `InKernelNative` tylko odwraca payload; `WasmSandbox` dodaje `WASM_SANITY_PASS::`; Firecracker wywołuje `sh -c echo`; MCP/API zwracają tekstowe statusy bez I/O. Żadna z tych ścieżek nie aktywuje usługi.
2. **OAuth/Vault:** `rand_entropy_seed()` zwraca stałą; `exchange_code_via_tls` generuje fikcyjne tokeny; `fetch_client_secret` zwraca konstruowany tekst; `store_tokens` tylko wypisuje log. Nie ma rzeczywistego IdP, weryfikacji ID Token i odwołania u dostawcy.
3. **BOLA/OCAP:** oryginalny podpis nie obejmuje `target_resource_tag`, używa porównania tablic zamiast API weryfikacji MAC, a atenuacja kontroluje tylko `max_byte_len`, nie wyklucza poszerzenia `allowed_operations` i `expiry_timestamp`. Test z PDF poszerza `Read,Write` do `Read,Execute` — powinien zostać odrzucony.
4. **Izolacja:** deklaracja `memory_zeroized=true` nie dowodzi zerowania pamięci gościa; brak realnego KVM/Wasmtime i pomiaru czasu uruchomienia.
5. **Zenoh:** kolejka w pamięci nie jest transportem między procesami; przesyłanie adresu/hasza wskaźnika nie daje dostępu do regionu SHM na innym hoście. Bloom ma false positives; nie jest źródłem prawdy o brakujących zmianach.
6. **Weryfikacja i pamięć:** ograniczony przegląd ścieżek AST nie dowodzi globalnej poprawności; heurystyczny wynik LTN nie jest dowodem matematycznym. Odwracalność kompresji wymaga porównania wszystkich bajtów i właściwej obsługi błędów, nie tylko jednego przykładowego stringu.

## Dowody i kryteria statusu

- **Źródło sprawdzone:** rozdziały i strony 1–28, zgodność SHA-256.
- **Kod istniejący:** `src/omat_server.rs`, `src/runtime/mod.rs`, moduły 0.44, `crates/ocaf-transport-crdt` i rejestr 555.
- **Nowy kod:** `src/sasos_capability.rs`, testy w module (podpis, cross-tenant, eskalacja, expiry, tampering). Do czasu wyniku CI: IMPLEMENTED_UNVERIFIED.
- **Brak:** aktywnych backendów 555, podłączenia capability do OMAt, realnego E2E z IdP/Vault/KVM, niezależnych benchmarków, produkcyjnego odbioru.
- **Bramka:** nie zmieniać `CATALOG_ONLY` ani `production_approved=false` bez weryfikowalnego dowodu rzeczywistego wykonania i decyzji operatora.
