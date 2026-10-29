# Aktualizacja audytu Drive i repozytorium

Punkt odniesienia: origin/main `bda7e63`. Poprzednia mapa oparta na braku plików stage045–stage049 była błędna: implementacje mają także nazwy sasos_* i są rozwijane w zakresie 334.

## Potwierdzony stan

- OAuth: istnieje kod i testy 0.44; rzeczywisty IdP/Vault/provider E2E i akceptacja wdrożenia pozostają otwarte.
- Capabilities: `src/sasos_capability.rs` i `tests/sasos_omat_capability.rs` już istnieją. Nie dodawać równoległego stage045_capability.
- Adaptery: `src/sasos_service_adapters.rs` łączy jawne bindingi Ring2 z VaultSecretBroker. Brak dowodu działania wszystkich 555 dostawców.
- Zenoh: rzeczywisty dwuprocesowy SHM/fallback na jednym kernelu; produkcyjny OMAt SHM oraz CRDT convergence po partycjach pozostają do weryfikacji.
- Firecracker: rzeczywisty laboratoryjny guest/KVM; host operatora i akceptacja produkcyjna pozostają otwarte.
- Formalizacja: ograniczone formuły Z3; brak pełnego dowodu programu Rust/ImandraX.
- Pamięć: działające referencyjne archiwum zlib/SHA-256 i bounded EMA; brak pełnych Titans/NICE/R3Mem oraz dowodów jakości na korpusie.

Wyniki GitHub Actions sprawdzone przez API: 40941557248 (261f8ed), 40942073444 (d52c9d9), 40942543567 (52be86f), wszystkie completed/success. Wyniki odnoszą się do tych commitów i zakresów testów, nie do całości produkcji.

## Numeracja

Kanoniczne 0.44–0.49 to OAuth, Crypto, Transport, Telemetry, Memory i Economics. PDF używa innej numeracji dla capabilities, sandbox, replikacji, BMC i pamięci. Wymagania PDF mapować semantycznie do istniejących etapów, bez przemianowania historycznych ID. Zobacz `ROADMAP.md` i `deploy/stage334/README.md`.

## Trzy dodatkowe źródła

Zaimportowano teksty i sumy kontrolne do `docs/sources/drive-ocaf/`:
- Google Docs: Agents Builder 0.50–0.55, dziesięć produktów platforma × Code/NoCode.
- Dokument bez nazwy(8).PDF: zakres 1.55–2.06.
- Wdrożenie Roadmapy Projektu OCAF-Runtime.PDF: zakres 1.15–1.54.

Import źródeł jest zakończony. Pełne uzgodnienie każdego wymagania tych trzech dokumentów z kodem i implementacja braków pozostają otwarte; sam import nie zmienia statusów roadmapy na DONE. Dokumenty zawierają rekonstrukcje i przykłady, które wymagają sprawdzenia z rzeczywistym repozytorium.
