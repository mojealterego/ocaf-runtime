# Implementacja dokumentu „Architektura OCAF: OAuth i Usługi”

Źródło: folder Drive `1EKb08OvnajBbhiBLiuWuhwInUy4odc19`, plik
`Architektura OCAF_ OAuth i Usługi - Google Gemini.PDF.pdf` (28 stron).
SHA-256 pobranego PDF:
`e3c74e0b5b0f68fa31f5c91ac6bb446cdf495e1b66cd724cc66953d099c1a8ee`.

Dokument wyznacza zakres prac 0.44–0.49. Kod w PDF jest szkicem; poniższa mapa
wiąże jego wymagania z obecnym repozytorium i konkretnymi bramkami odbioru.

| Etap | Wymaganie z PDF | Obecny stan w repo | Następna bramka |
| --- | --- | --- | --- |
| Dyspozytor | Rozdział na native, WASI, MicroVM, MCP i zewnętrzne API | `src/omat_server.rs` ma manifest, politykę i odmowę wykonania bez OSTc; `src/stage043_store.rs` ma trwały podpisany rejestr i odwołanie | Adapter dla jednego rzeczywistego backendu, związany z manifestem, polityką i revocation; test procesu |
| 0.44 | OAuth/OIDC, S256 PKCE, MFA, Vault i rotacja | `src/stage044_oauth.rs`, `src/stage044_oidc.rs`, `src/stage044_vault.rs`, `src/stage044_provider_refresh.rs` oraz `ocaf-oauth-gateway` | Test z rzeczywistym IdP, Vault i bramą na docelowej infrastrukturze; warunki w `deploy/stage044/README.md` |
| 0.45 | Zdolności przypięte do obiektu i podmiotu | Brak modułu `stage045` | Projekt biletu podpisanego, scope, tenant, resource i atomowego odwołania; test odmowy BOLA |
| 0.46 | Wasmtime i Firecracker z limitami i czyszczeniem pamięci | Brak modułu `stage046` | Procesowy test realnego guest/wasm, fuel, pamięci, timeoutu i izolacji |
| 0.47 | Zenoh, Merkle/CRDT i replikacja | Zenoh transport 0.43 istnieje; brak deklarowanej replikacji 0.47 | Test dwóch węzłów, konfliktów, replay, odzyskiwania i spójności po partycji |
| 0.48 | Bounded model checking i zapora LTN | Brak modułu `stage048` | Jawny model i ograniczenie głębokości, kontrprzykład lub ślad; bez deklaracji pełnego dowodu |
| 0.49 | Titans, NICE, RQGM i pamięć | Brak modułu `stage049` | Odwracalność na arbitralnych bajtach, trwałość danych, limity aktualizacji i pomiar jakości |

## Granica między szkicem a wykonaniem

PDF pokazuje `InKernelNative` odwracające bajty, `WasmSandbox` dopisujące
`WASM_SANITY_PASS::`, `FirecrackerMicroVm` wywołujące `sh -c echo`, a trasy
MCP/API zwracające tekstowy status bez żądania sieciowego. Przykład Vault
konstruuje ciąg podobny do sekretu i wypisuje komunikat zamiast odczytu/zapisu.
Takie operacje nie mogą aktywować wpisów `CATALOG_ONLY` ani tworzyć potwierdzeń
wykonania. Wszystkie 555 wpisów pozostaje metadanymi do czasu powstania
osobnych adapterów i przejścia bramek bezpieczeństwa.

Implementację prowadzimy kolejno po jednej weryfikowalnej granicy. Pierwszą
jest pojedynczy backend dyspozytora połączony z obecnym manifestem OMAt,
podpisem, tenantem, revocation i OSTc. Dopiero wtedy dodajemy następne klasy
backendów i mierzymy faktyczne opóźnienia. Status 0.44 pozostaje
„NOT PRODUCTION ACCEPTED” do czasu wskazanej tam akceptacji sieciowej.
