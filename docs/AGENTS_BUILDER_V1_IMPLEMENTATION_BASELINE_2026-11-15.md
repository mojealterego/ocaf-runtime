# OCAF Agents Builder v1.0 — przyjęty zakres i stan implementacji

**Data bazy wymagań:** 2026-11-15  
**Podstawa:** „Architektura i Kompletna Implementacja Referencyjna Systemu OCAF Agents Builder v1.0”, przekazana przez operatora w rozmowie OCAF.  
**Gałąź:** `main` · **Zakaz automatycznego zatwierdzenia produkcji.**

## Zakres obowiązujący

- **INV-001–013; REQ-001–131; SEC-001–010; ADR-01–07; EPIC-01–12; GATE-01–08, GATE-10.** Rejestr ID jest bazą śledzenia wymagań, **nie dowodem** zrealizowania każdego numeru; zakres REQ i SEC obejmuje identyfikatory z dokumentu, bez domniemywania treści nieprzekazanych w źródle.
- **GATE-10:** 5 bazowych kanałów (Cloud, Windows, macOS, Android, iOS) × Code/NoCode = **10 produktów**. Linux, HarmonyOS, KaiOS stanowią kanały rozszerzone M6 i nie zmieniają GATE-10. Ta normalizacja jest decyzją referencyjną ADR-01 do formalnego zamknięcia w odpowiednim rejestrze.
- **Architektura:** View → Application → Domain ← Infrastructure. Nie twórz równoległego rdzenia zastępującego obecne `src/builder_core.rs`, `src/builder_store.rs`, `src/builder_cloud.rs`, `src/builder_http.rs` bez świadomej migracji. Istniejące moduły są wykorzystane; referencyjny tekst `crates/builder-*` nie zastępuje ich automatycznie.
- **Zatwierdzona polityka:** zewnętrzny audyt nie jest obowiązkowy dla wewnętrznego odbioru inżynieryjnego. Deterministyczne testy/symulacja stanowią odrębny zakres dowodu; nie potwierdzają urządzeń, natywnych buildów ani produkcyjnych wdrożeń (patrz `docs/ENGINEERING_SIMULATION_ACCEPTANCE.md`).

## Dostarczone w tym przyroście

| Artefakt | Pokrycie | Rzeczywisty status |
|---|---|---|
| `schemas/agents_builder_v1_workspace.schema.json` | INV-011; REQ-020–025; ADR-04 | **IMPLEMENTED**, strukturalny JSON Schema Draft 2020-12; nie jest kompilatorem ani walidatorem semantyki DAG |
| `scripts/verify_builder_gate10.py` | INV-013; REQ-120/130/131; GATE-07/08/10 | **IMPLEMENTED / LOCALLY_TESTED**, odczytowy weryfikator podpisanych dowodów 10 produktów, nie wykonawca buildów |
| `scripts/verify_gate10.sh` | Wrapper dla niezależnego narzędzia GATE-08 | **IMPLEMENTED**, uruchamia sprawdzanie w trybie fail-closed |
| `scripts/tests/test_builder_v1_workspace_schema.py` | REQ-023–025, regresje schematu | Testy kontraktowe struktury, bez dowodzenia poprawności semantycznej |
| `scripts/tests/test_verify_builder_gate10.py` | GATE-08, próby fałszowania dowodów | Testy regresyjne z **syntetycznymi** podpisanymi fixture; nie dowodzą działania 10 urządzeń |
| `release/CERTIFICATE_RECORD.json` | REQ-120/130, status produktu | **NIEPOTWIERDZONE** dla wszystkich 10; brak wymyślonych SHA binariów |

## Wykryte luki w dostarczonej implementacji referencyjnej

Poniższe punkty są rezultatem statycznego przeglądu przekazanego tekstu. **Nie stanowią raportu z natywnej kompilacji tego tekstu.**

1. `emit_reproducible_wasi_artifact`: dopisanie ASCII SHA do nagłówka `\0asm` **nie kompiluje wykonywalnego agenta WASI**. Nie wolno oznaczyć tego jako GATE-04/07 PASS.
2. `services/cloud-api`: kontekst `x-tenant-id` / `x-user-id` / `x-role` pochodzi wprost z nagłówków requestu, więc **nie dowodzi OAuth 2.1/OIDC ani RBAC**. Należy korzystać z istniejącej granicy `builder_cloud`/`builder_http` ze zweryfikowanym `AuthenticatedSubject`.
3. `signed_export_endpoint`: twardo zakodowane `[7u8;32]` i wspólny HMAC nie mogą być użyte w produkcji. Weryfikacja `signer_public_key_hex` dostarczonego przez ten sam manifest bez przypiętego klucza zaufania nie dowodzi pochodzenia z uprawnionego źródła.
4. `build-worker`: zwraca sukces i syntetyczny checkpoint bez rzeczywistego uruchomienia modułu WASI; „RUNNING” w API jest deklaracją, nie dowodem procesu.
5. UI Code/NoCode, SwiftUI i KaiOS posiadają elementy zwracające komunikat „ZWERYFIKOWANO”/„WYSŁANO” bez przeprowadzenia żądania lub walidacji. Tych pozorowanych reakcji nie wolno użyć w odbiorze.
6. Android deklaruje natywne metody JNI bez dowodu wygenerowania wiązań UniFFI i kompilacji `.so`/APK. Deklaracje iOS / Tauri również nie zastępują binariów i GUI E2E.
7. Dostarczony `verify_gate10.sh` sprawdza jedynie niepustość i wystąpienie SHA w tekście; umożliwia fałszywy PASS. Zamiast niego obowiązuje weryfikator z obliczaniem SHA-256 rzeczywistych bajtów i weryfikacją podpisów Ed25519 względem **zewnętrznie dostarczonego przez operatora klucza publicznego** (to lokalna bramka integralności, nie audyt zewnętrzny).
8. Szkic `canonical_sha256_tree_root` nie obejmuje wszystkich pól Workspace, m.in. agents/tools/policy_refs/secret_refs/artifacts. Nie jest poprawnym dowodem integralności całego projektu.
9. W szkicu walidacji `has_human_approval_ancestor` wystarcza istnienie **jednej** ścieżki z approval; rozgałęzienie bez approval może prowadzić do skutku zewnętrznego. Wymagany dowód dominacji approval na **wszystkich** ścieżkach do każdego węzła wykonującego efekt.
10. ADR-01–07 w dostarczonym tekście mają **proponowane** rozstrzygnięcia, nie należy sztucznie zmieniać ich na formalnie zatwierdzone lub certyfikowane.

## Weryfikacja GATE-10 i model zaufania

`verify_builder_gate10.py` wymaga:

- dokładnie 10 produktów z kanonicznej `docs/roadmap/agents_builder_products_v11.json` zgodnych z `main` i danym SHA (również przy ukrytych zmianach indexu Git);
- na produkt: rzeczywiste, **niepuste** pliki binarne, kompletny sześciostopniowy log GUI E2E `create/edit/validate/compile/export/import` oraz niepusty CycloneDX SBOM; dla produktów innych niż Cloud dodatkowo log instalacji/uruchomienia z target triple i identyfikatorem urządzenia;
- wszystkie digesty SHA-256 policzone na bajtach plików; brak symlink escape, path traversal, dużych nieograniczonych plików lub nieznanych krytycznych pól;
- `CERTIFICATE_RECORD.json` **dla każdego produktu** podpisany Ed25519, weryfikowany kluczem publicznym dostarczonym niezależnie od release, a nie kluczem znajdującym się w pakiecie;
- brak prób ustawienia `PRODUCTION_APPROVED` na podstawie samych podpisanych plików. Poprawny wynik skryptu nosi nazwę `GATE10_EVIDENCE_INTEGRITY_VERIFIED` i zawiera `production_approved=false`; nie potwierdza fizycznej tożsamości runnera ani prawdziwości treści podpisanego logu. Zaufanie do wykonawcy wynika z oddzielnej konfiguracji i procesu operatora.

**Uruchomienie po zainstalowaniu przypiętych zależności:**

```bash
python3 -m pip install -r scripts/requirements-builder-gate10.txt
python3 -m unittest discover -s scripts/tests -p 'test_verify_builder_gate10.py' -v
python3 -m unittest discover -s scripts/tests -p 'test_builder_v1_workspace_schema.py' -v
scripts/verify_gate10.sh "$(git rev-parse HEAD)" /secure/evidence/release '<PINNED_32_BYTE_ED25519_PUBLIC_KEY_HEX>'
```

**Ograniczenie środowiskowe:** pliki dowodowe i katalog release muszą być przygotowane w zaufanym katalogu roboczym bez równoległego modyfikowania przez niezaufane procesy. Testy lokalne w syntetycznym repozytorium nie są dowodem wykonania na bieżącym `main` GitHub.

## Następna praca wykonawcza bez dublowania istniejącego rdzenia

**M0 / M1:** domknąć ADR oraz wersjonowanie i migrację nowego kontraktu do istniejącego Rust `builder_core`. **M2:** prawdziwe round-trip, lossiness snapshots, ochrona `opaque_code`. **M3:** powiązanie istniejącego Cloud OAuth/SQLite z widokami Code/NoCode bez ufań nagłówkom klienta. **M4:** rzeczywiste binaria i GUI E2E Android/iOS/Windows/macOS. **M5:** realny moduł WASI, odseparowane klucze, sandbox i podpis. **M6:** 3 kanały rozszerzone. **M7:** signed export, SBOM, GATE-08 i indywidualna akceptacja operatora. Żaden etap nie zostaje automatycznie podniesiony przez samą obecność tego dokumentu.
