# OCAF AB-OM-03/14 — wdrożony katalog źródłowy 111 agentów i 33 rojów

Data: 2026-11-26. Repozytorium: `mojealterego/ocaf-runtime`. Gałąź: `main`. Bazowy commit: `41a928a4d193d6f42c1c0c9ea453c33b0caedb28`.

## Faktyczny zakres implementacji

- `scripts/build_public_agent_catalog.mjs`: generator odczytuje wersjonowane dane `docs/agents/agent_registry_111.json` i `docs/agents/swarm_groups_v1.json`. Odmawia sfałszowanych statusów ACTIVE/BUILT/DEPLOYED, uaktywnionego OSTc, podmiany ID lub nazwy, duplikatów, niezgodnych grup, nieprawidłowego UTF-8 oraz niezgodności 111/33. Wytwarza publiczny, jednoznacznie **nieautorytatywny** katalog z Git blob SHA obu źródeł.
- `apps/agents-builder-web/agent-catalog.json`: dane 111 agentów i 33 rojów z faktycznych rejestrów, bez wymyślonych cen, certyfikatów lub uruchomień.
- `apps/agents-builder-web/agent-catalog.mjs`: ścisły parser JSON, walidacja schematu i integralności struktury, limity transferu, wymuszone `credentials:omit`, `redirect:error`.
- `apps/agents-builder-web/agent-catalog-ui.mjs`: widok przeglądarkowy wyszukiwania i statusów, wyłącznie `textContent` dla danych. Pokazuje `NOT_VERIFIED`, `NOT_ISSUED`, `NOT_PRICED` i zakaz wykonania.
- Integracja w `app.mjs`, `index.html`, `style.css`, PWA `sw.js`, `local_runner.py`, `builder_cloud_ingress.py`, testowym serwerze Chromium, selektorze paczek źródłowych i CI. Brak automatycznego wykonania, zakupów, licencji, dostępu do źródeł agentów czy aktywacji OSTc.

## Przeprowadzone dowody

- Testy lokalne, Node.js 22.16: `node --test scripts/tests/test_public_catalog.mjs` **5/5 PASS**.
- Testy lokalne, Node.js 22.16: `node --test apps/agents-builder-web/tests/agent-catalog*.test.mjs` **8/8 PASS**.
- `node --check` trzech nowych modułów — PASS.
- Zweryfikowano, że blob SHA plików przesłanych do GitHub odpowiada SHA plików testowanych lokalnie: generator `53eb69210f5df241c0117e1ebf2fa658cd7c8ece`; model `f435e4cd65e3c6d6deb9ffb36bc12d2968d7cffa`; UI `ddbce3403f4553466ab6d269b771c20177ee1a2e`; testy generatora `de12f1be920c6fbda57efeb33eec60688c73fbbe`, modelu `fc0c532e70758e7ad57f1d35f1ccf8b949313524`, UI `4f4f21af577861eb68eab7e834f74aabf0df3867`. Blob katalogu `8036cfc6e22e09c21eb419930c9cd1735f233b80`.

## Otwarte bramki

Testy w izolowanym katalogu nie potwierdzają uruchomienia kompletnego, historycznego OCAF Runtime. Pełne Chrome E2E, Rust/Zenoh, platformowe buildy, kompletne dossier, podpisany dynamiczny katalog, certyfikacja, cenniki i wydania nadal NIEZWERYFIKOWANE. AB-OM-03 i AB-OM-14 pozostają **OPEN_UNVERIFIED**. `production_approved=false`. Nie włączać transakcji ani OSTc na podstawie tego przyrostu.
