# OCAF Agents Builder — wielojęzyczność, klucze API i GGUF (2026-11-03)

## Kontrakt
- Używamy **jednego** istniejącego UI dla Code/NoCode i Local/Cloud.
- Język polski pozostaje fallback; katalogi en-US, cs-CZ, sk-SK, de-DE, uk-UA i ar-SA.
  Inne języki jako kontrolowane katalogi BCP47; brak obietnicy przetłumaczenia 7000 języków.
- Tokeny OAuth Cloud pozostają oddzielne od kluczy dostawców. Klucze API
  są sesyjne, nigdy nie są zapisywane ani dołączane do projektu.
- Publiczny katalog Hugging Face pod stałym origin; URL pobierania tylko
  zweryfikowany repo/path i przypięty commit SHA. Plik lokalny podlega
  kontroli podpisu GGUF i wersji, nie tylko rozszerzenia.
- Model profile schema v1 jest ściśle walidowany. Inferencja nie jest uruchamiana.

## Zakres implementacji
- `apps/agents-builder-web/i18n.mjs`
- `apps/agents-builder-web/credentials.mjs`
- `apps/agents-builder-web/models.mjs`
- `apps/agents-builder-web/integrations-ui.mjs`
- `apps/agents-builder-web/tests/integrations.test.mjs`
- Integracja z istniejącymi `app.mjs`, `index.html`, `style.css`, `sw.js`,
  test Chromium i workflow `stage051-builder-web.yml`.

## Otwarta bramka i ryzyka
- Aktywowanie **rzeczywistej inferencji GGUF** wymaga adaptera llama.cpp
  lub równoważnego, z limitem pamięci/VRAM, kontrolą urządzenia i testami.
  Odczyt nagłówka/wybranie pliku nie potwierdza załadowania modelu.
- Aby użyć kluczy w usługach produkcyjnych, wymagany szyfrowany vault/
  systemowy Keychain/Keystore, broker tenant-scoped, polityki egress,
  audyt, rotacja i nieprzechowywanie sekretów w JS przez czas nieograniczony.
- HF private/gated i licencje modeli pozostają poza nieautoryzowanym automatem.
- Kod klienta i testy są przypięte do SHA (poniżej), status `TESTED_LOCAL_PARTIAL`.
- Macierz 10 produktów i status `production_approved=false` bez zmian.

## Zweryfikowane wykonanie

**CI potwierdzone 2026-11-03:** commit `eb400184b69dfc80d59aa4f525bcaa9e47ed2c6b`; [OCAF Builder Web #41578863024](https://github.com/mojealterego/ocaf-runtime/actions/runs/41578863024) zakończony `completed/success`: 20/20 testów Node JavaScript, Chrome GUI E2E (w tym locale/RTL, nietrwały klucz sesyjny, walidacja lokalnego GGUF, zapis profilu), 1/1 Rust fixture PASS. [Roadmap v11 #41578863062](https://github.com/mojealterego/ocaf-runtime/actions/runs/41578863062) zakończony `completed/success`. **Jedynie `TESTED_LOCAL_PARTIAL`; brak wykonania inferencji, sejfu API, realnego pobierania modelu w CI, natywnych buildów i produkcyjnego Cloud.**
