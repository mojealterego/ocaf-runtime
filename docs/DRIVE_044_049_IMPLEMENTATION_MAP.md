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

## Dalsza integracja etapu 334 — bramka capability / OMAt (2026-10-29)

**Kod dodany:** `src/stage043_omat_route.rs` udostępnia
`omat_resource_id(call)` (SHA-256 z rozdzielonymi identyfikatorami
tenantu, agenta i narzędzia) oraz `dispatch_tool_call_with_capability`.
Weryfikacja biletu obejmuje podmiot uwierzytelniony przez transport,
tożsamość zasobu, prawo `Execute`, limit bajtów argumentów i czas
ważności ustalany przez zegar serwera. Dopiero potem wywoływana jest
dotychczasowa polityka OMAt i bramka OSTc. Brak biletu nie powoduje
przejścia na starszy, mniej restrykcyjny dyspozytor.

**Żywy ingress:** `src/stage043_omat_zenoh.rs` obsługuje podpisane
ramki zawierające opcjonalny bilet, ale **odrzuca ramki bez biletu**.
MAC ramki obejmuje również cały bilet. Trwały rejestr nonce SQLite
pozostaje wymaganym mechanizmem ochrony przed powtórzeniem. Sam bilet
nie jest tokenem jednorazowym ani substytutem mTLS. Serwer
`src/bin/ocaf-omat-zenoh.rs` może załadować osobny klucz urzędu
capability z pliku wskazanego przez `OCAF_OMAT_CAPABILITY_KEY_FILE`.
Plik musi być prywatny i zawierać Base64 z dokładnie 32 bajtami
klucza, różnego od klucza HMAC OMAs. Brak pliku oznacza **DENY**,
nie automatyczne wystawianie biletów. Wystawienie biletu jest operacją
uprawnionego operatora, nie funkcją dostępną modelowi.

**Klient OMAs:** `src/bin/ocaf-omas-zenoh-client.rs` może odczytać
wystawiony przez operatora bilet z prywatnego pliku
`OCAF_OMAT_CAPABILITY_TICKET_FILE` i podpisać ramkę wraz z biletem.
Bez tego pliku klient zachowuje protokół historyczny, ale OMAt
odmawia jego wykonania. Test podpisanej ramki nie jest dowodem
aktywacji executora.

**Kompozycja OAuth:** `src/stage044_omat_authz.rs` udostępnia
`dispatch_with_capability`, który wymaga jednocześnie ważnego
OAuth grant, zaufanej tożsamości peera, biletu OCAP, polityki OMAt
oraz niezależnej bramki wykonania. Testy w
`tests/stage044_integration.rs` obejmują odwołanie tokenu OAuth,
nieprawidłowy bilet i brak atestacji OSTc.
`tests/sasos_omat_capability.rs` obejmuje zmianę agenta, narzędzia,
tenantu, limitu bajtów, brak uprawnień, modyfikację podpisanej ramki
i powtórzenie nonce.

**Katalog 555:** `scripts/sasos_activation_gate.py` sprawdza
kompletność identyfikatorów 001–555, unikalność, deklaracje runtime,
schematy pól i niezmienniki `CATALOG_ONLY`. Odrzuca fałszywe
`executable=true`, `imandra_verified=true`, `zero_copy_verified=true`
i wymyślone pomiary latencji. Testy negatywne:
`scripts/tests/test_sasos_activation_gate.py`.
CI: `.github/workflows/sasos-activation-contract.yml`.
Ten gate **nie uruchamia** 555 usług i nie stanowi dowodu istnienia
backendów.

**Nie zamknięto**: podłączenia rzeczywistych executorów OSTc
z podpisanymi manifestami i potwierdzonymi efektami, pełnych
backendów 555 usług, realnych testów z zewnętrznym IdP/Vault,
docelowego Firecracker KVM, Zenoh SHM (tylko ten sam host),
certyfikatów formalnego solvera i walidacji modelu Titans/NICE.
Brak dowodów oznacza status PLANNED / NOT_VERIFIED, nie PASS.
