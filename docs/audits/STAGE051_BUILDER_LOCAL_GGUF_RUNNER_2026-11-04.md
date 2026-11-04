# OCAF — odbiór przyrostu lokalnej inferencji GGUF / 2026-11-04

## Kontrakt techniczny
- UI pozostaje jednym edytorem Code/NoCode z przełącznikiem Local/Cloud.
- Lokalna inferencja jest **oddzielona** od chmurowej autoryzacji Rust.
- Operator instaluje binarny `llama-server`, określa dozwolony katalog GGUF
  i uruchamia `apps/agents-builder-web/local_runner.py` na 127.0.0.1.
- Endpoints: `GET /v1/local-runner/models`, `GET /v1/local-runner/status`,
  `POST /v1/local-runner/start`, `POST /v1/local-runner/chat`,
  `POST /v1/local-runner/stop`.
- Oddzielne Bearer tokeny operator/browser i bridge/llama.cpp.
- Bez pobierania lub wykonywania kodu z modeli, tools, shell, arbitrary URL,
  katalogów poza dozwolonym, symlinków i połączeń z zewnętrznym backendem.
- Profil settings v1 walidowany po stronie usługi, limity rozmiaru, kontekstu,
  wątków, batch, modelu i długości wiadomości.
- Przed uruchomieniem: magic GGUF, wersja 2/3, liczba tensorów i metadanych.
- Test deterministyczny nie jest prawdziwym testem inferencji wag modelu.

## Pliki
- `apps/agents-builder-web/local_runner.py`
- `apps/agents-builder-web/runner-client.mjs`
- `apps/agents-builder-web/tests/runner.test.mjs`
- `scripts/tests/test_agents_builder_local_runner.py`
- Zmodyfikowano GUI, i18n, Service Worker, Chromium smoke, CI i pakowanie ZIP.

## Dowody i otwarte bramki
- RED: `63142b70e8fcbf9d427d19c75838d34f5973206a` (test runnera
  nie mógł przejść bez nieistniejącej wtedy implementacji).
- GREEN: do przypięcia po zakończeniu CI bieżącego commita.
- Status dopuszczalny: `TESTED_LOCAL_PARTIAL` dopiero po CI.
- **Bez dowodu:** uruchomienia rzeczywistego modelu GGUF, parametrów
  RAM/VRAM na Samsung Android, niezależnego audytu, systemowego sandboxa,
  produkcyjnego uwierzytelniania wielotenantowego i akceptacji operatora.
  `production_approved=false`; historyczna roadmapa pozostaje nienaruszona.

## Uzupełnienie 2026-11-04: bezpieczny browser-to-local upload

- Endpoint POST /v1/local-runner/import wymaga sesyjnego Bearer i Origin; transfer odbywa się wyłącznie do dozwolonego katalogu GGUF na localhost.
- JSON nie zawiera wag ani kluczy API; plik strumieniowany do 1 MiB porcji, z limitem rozmiaru, integralnością SHA-256, weryfikacją nagłówka i atomowym utworzeniem nazwy bez nadpisania.
- Testy red commit af8b3058161ca1aa8839664f084068a3c27ea41a; green CI do weryfikacji. Produkcyjne wagi/model E2E niepotwierdzone.
