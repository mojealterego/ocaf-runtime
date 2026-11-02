# OCAF 0.50–0.51 — przeglądarkowy Code/NoCode i magazyn projektów

**Data:** 2026-11-02
**Web CI SHA:** 7f578d8c617cdeadc2472628ba34decd55c18839
**Web CI:** https://github.com/mojealterego/ocaf-runtime/actions/runs/41526215520 — completed/success (Node tests, Chromium GUI E2E, Rust fixture).
**Store CI:** oczekuje na zakończenie; nie oznaczać jako TESTED bez dowodu.
**Status:** WEB TESTED_LOCAL_PARTIAL; SQLite IMPLEMENTED_UNVERIFIED; 0.50–0.55 PLANNED_OR_UNVERIFIED.

## Faktycznie dodane komponenty

- apps/agents-builder-web/editor.mjs — ograniczony, walidowany typed DAG, deterministyczna reprezentacja JSON DSL, kontrola typów portów, uprawnień i budżetu, edycje i historia lokalna.
- apps/agents-builder-web/app.mjs, index.html, style.css — działające dwa tryby GUI: NoCode z węzłami i połączeniami oraz edytor Code JSON; historia i restore, import/eksport, statyczny preview.
- apps/agents-builder-web/sw.js, manifest.webmanifest — statyczne zasoby PWA, cache wyłącznie publicznych plików edytora.
- apps/agents-builder-web/tests/editor.test.mjs — testy jednostkowe modelu i negatywne.
- apps/agents-builder-web/tests/browser-smoke.mjs — rzeczywisty Chrome GUI smoke: dodanie węzła, przejście Code, zapis rewizji, odmowa nieobsługiwanego DSL, preview, ponowny odczyt po przeładowaniu.
- tests/builder_web_contract.rs — ten sam fixture walidowany przez Rust.
- src/builder_store.rs, tests/builder_store.rs — SQLite CAS, tenant-scoped revisions, hash chain i pinned-head, status testów CI do potwierdzenia.

## Granice dowodów i bezpieczeństwa

Browser localStorage nie zapewnia uwierzytelnienia ani transakcyjnej trwałości Cloud. Tenant local to wyłącznie przestrzeń nazw lokalnego projektu. Walidator JS nie jest granicą bezpieczeństwa: każdy serwer musi ponownie wywołać Rust compile() z tożsamością ustaloną przez uwierzytelniony ingress. Nie ma wywołań narzędzi, autoryzacji wykonania, sandboxa, podpisanych paczek, publikacji ani wdrożenia.

Web/PWA nie jest natywną aplikacją Windows/macOS/Android/iOS. Nie przeprowadzono ich buildów i E2E. Dziesięć produktów nadal nie spełnia kryteriów odbioru; statusy kanoniczne pozostają bez zmian.

## Następne obowiązki

1. Dokończyć i potwierdzić CI magazynu SQLite oraz integrację przez uwierzytelniony Cloud API.
2. Wykonać realny backend Cloud Code/NoCode z tenant-scoped durable store i sandboxed preview.
3. Dostarczyć osobne GUI/artefakty Windows, macOS, Android, iOS dla Code i NoCode, z platformowym E2E.
4. Dodać signed export, verified import, approvals i fail-closed publishing.
