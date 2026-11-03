# OCAF 0.51–0.52 — jeden interfejs Code/NoCode, Local/Cloud (2026-11-03)

## Zweryfikowany stan początkowy

- Main przed przyrostem: `dfd225ee8a21d3db7c5550e3e85b46fc46e8a1f6`.
- Istniejące `src/builder_core.rs`, `src/builder_store.rs`, `src/builder_cloud.rs`, `src/builder_http.rs` i `apps/agents-builder-web` zostały zachowane; brak nowego, duplikującego runtime.
- Przed przyrostem `BuilderHttpBoundary` odrzucał brak Origin także na GET, mimo że przeglądarka zwykle nie wysyła Origin przy same-origin GET.

## Zakres zmian

1. Jeden UI, dwa niezależne przełączniki: `Code / NoCode` i `Lokalnie / Chmura`.
2. `cloud.mjs`: ograniczony rozmiarem i timeoutem klient same-origin HTTPS, nietrwały Bearer, `credentials: omit`, `redirect: error`, `cache: no-store`.
3. Odczyt, utworzenie i zapis chmurowy przez istniejące `/v1/projects/{id}`; jawny zapis CAS, bez automatycznego retry HTTP 409; uwierzytelniony, niewykonawczy preview.
4. Odrębne projekty lokalne i chmurowe, brak automatycznej migracji, brak zapisu tokenów w localStorage.
5. Backend dopuszcza brak Origin tylko przy GET z poprawnym Bearer i zaufanym `AuthenticatedSubject`; wszystkie przekazane, niezgodne Origin są odrzucane.
6. Testy klienta, przeglądarkowy E2E z testowym serwerem API, regresja HTTP boundary. Serwer testowy nie jest wdrożeniem ani autoryzacją produkcyjną.

## Kryteria i otwarte bramki

- `IMPLEMENTED_PARTIAL`: kod klienta, UI i regresja HTTP.
- `TESTED_LOCAL_PARTIAL`: wyłącznie po wykonaniu właściwych testów; wynik należy powiązać z commit SHA i logiem CI.
- `INTEGRATED_CLOUD`: **NIE** — nie ma rzeczywistego listenera HTTPS, terminacji TLS, gateway zapewniającego `AuthenticatedSubject`, OAuth flow w UI ani deploymentu.
- `PRODUCTION_APPROVED`: **NIE** — brak rzeczywistego sandbox, publikacji, audytu, podpisów i macierzy 10/10.
- Android: responsywny PWA w przeglądarce, nie natywny APK. Binarny Meta Agent Architekt poza priorytetem.
- Nie aktualizować statusów 10 produktów na PASS na podstawie testowego serwera API.

## Walidacja

```sh
node --test apps/agents-builder-web/tests/*.test.mjs
CHROME_BIN=chromium node apps/agents-builder-web/tests/browser-smoke.mjs
cargo test --locked --no-default-features --test builder_cloud
cargo fmt --all -- --check
cargo clippy --locked --no-default-features --test builder_cloud -- -D warnings
```
