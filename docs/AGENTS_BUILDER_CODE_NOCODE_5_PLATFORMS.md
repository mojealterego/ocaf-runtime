# OCAF — Agents Builder Code / NoCode — przywrócenie pierwotnego zakresu (v11)

**To są dziesięć produktów do projektowania, budowania, testowania i publikowania agentów (Agents Builder), NIE dziesięć gotowych agentów AI.** Przypadkowe sprowadzenie tej pozycji do samych „agentów”, „roju agentów” albo pojedynczego generowania skryptu jest błędem inwentaryzacyjnym.

**Pochodzenie:** pierwotna architektura `docs/MASTER_ARCHITECTURE_26_PDFS.md` opisuje `ocaf-builder-core`, Code↔No-Code, Cloud/Axum, Desktop/Tauri i Mobile/UniFFI. Operator potwierdził wymagane, osobne produkty **Cloud / Windows / macOS / Android / iOS**, każdy w dwóch wariantach **Code** oraz **NoCode**. Odróżnienie Windows/macOS i Android/iOS pochodzi z tego doprecyzowania pierwotnego zakresu. Nie należy dopisywać go jako nowej koncepcji pod ostatnim etapem.

**Oryginalne miejsce w kolejności:** 0.50 (Builder Core), 0.51 (Builder Code/NoCode UX), 0.52 (Cloud Code+NoCode), 0.53 (Windows Code+NoCode i macOS Code+NoCode), 0.54 (Android Code+NoCode i iOS Code+NoCode), 0.55 (safety/publish/test). **Żaden z tych numerów nie został skasowany ani przemianowany na nowy etap 1.95+.**

## Macierz dziesięciu produktów (pierwotne wymagania — odzyskane)

| ID produktu | Produkt | Oryginalny etap platformy | Zakres funkcji | Realny test końcowy | Status |
|---|---|---|---|---|---|
| `AB-CLOUD-CODE` | **Agents Builder Chmurowy Code** | 0.52, wspólne 0.50/0.51/0.55 | Web IDE + typed graph IR + Rust backend/Axum + Git integration + isolated build/execute + deployments + versioning | Browser Code IDE edits graph DSL, compiles/simulates typed agent, persists versioned project in tenant-scoped Cloud workspace; full build log | **BRAK IMPLEMENTACJI** |
| `AB-CLOUD-NOCODE` | **Agents Builder Chmurowy NoCode** | 0.52, wspólne 0.50/0.51/0.55 | Browser visual drag/drop graph editor + templates, form nodes, visual tool/MCP grants and one-click preview/publish behind explicit approval | Browser NoCode canvas builds equivalent graph from nodes, round-trips graph JSON with compiled DSL for supported subset, and previews without source editing | **BRAK IMPLEMENTACJI** |
| `AB-WINDOWS-CODE` | **Agents Builder Desktopowy Windows Code** | 0.53, wspólne 0.50/0.51/0.55 | Native Windows desktop Code IDE (Tauri v2) with local code editor, diagnostics, dependency runner, export to local agent bundle + offline mode | Actual Windows runner builds, installs and launches signed/verified executable; Code IDE opens agent project, compiles valid Rust/DSL, persists/reopens offline | **BRAK IMPLEMENTACJI** |
| `AB-WINDOWS-NOCODE` | **Agents Builder Desktopowy Windows NoCode** | 0.53, wspólne 0.50/0.51/0.55 | Windows desktop NoCode designer, node library, visual connections/parameters, local preview, export + user-approved publishing | Actual Windows GUI smoke: NoCode drag/drop creates graph, save/reopen offline, convert supported graph to Code, publish artifact only with explicit approval | **BRAK IMPLEMENTACJI** |
| `AB-MACOS-CODE` | **Agents Builder Desktopowy macOS Code** | 0.53, wspólne 0.50/0.51/0.55 | Native macOS desktop Code IDE (Tauri v2) including notarization-ready packaging, keychain policy, codesign verification and offline agent editing | Actual macOS runner builds/boots executable, validates bundle codesign entitlement metadata, edits and compiles Agent Builder Code project offline | **BRAK IMPLEMENTACJI** |
| `AB-MACOS-NOCODE` | **Agents Builder Desktopowy macOS NoCode** | 0.53, wspólne 0.50/0.51/0.55 | macOS native NoCode canvas, typed forms and graph, local simulator and editable DSL export with reversible subset checks | Actual macOS GUI smoke saves/loads NoCode agent, connects nodes, previews, converts and checks typed IR roundtrip, controls permissions | **BRAK IMPLEMENTACJI** |
| `AB-ANDROID-CODE` | **Agents Builder Android Code** | 0.54, wspólne 0.50/0.51/0.55 | Android Code editor (Kotlin/Compose + UniFFI Rust core), permission-aware on-device DSL edit/validation, mobile build through explicit external authorized compiler | Actual Android ARM64 emulator/device installs APK, creates/edits Code agent in bounded DSL, saves locally offline and resumes after app restart | **BRAK IMPLEMENTACJI** |
| `AB-ANDROID-NOCODE` | **Agents Builder Android NoCode** | 0.54, wspólne 0.50/0.51/0.55 | Android touch NoCode designer with offline graph CRUD, gestures, templates, safe-preview, signed import/export and user-mediated permission grant | Android instrumentation creates visual NoCode graph, validates typed edges and safe preview, exports/imports offline, survives process death | **BRAK IMPLEMENTACJI** |
| `AB-IOS-CODE` | **Agents Builder iOS Code** | 0.54, wspólne 0.50/0.51/0.55 | iOS Code editor (SwiftUI + UniFFI Rust core), edit and static-validate bounded agent DSL, code build/export via authorized remote toolchain, Sandbox constraints | Actual Xcode iOS simulator/device run creates/edits Code agent under app Sandbox, validates typed agent DSL, restores local project after restart | **BRAK IMPLEMENTACJI** |
| `AB-IOS-NOCODE` | **Agents Builder iOS NoCode** | 0.54, wspólne 0.50/0.51/0.55 | iOS SwiftUI touch NoCode agent builder with offline graph designer, export/import via Document Picker, user-controlled tool entitlements | Actual iOS simulator UI builds multi-node NoCode workflow, round-trips supported IR with Code view, persists offline and respects app life-cycle | **BRAK IMPLEMENTACJI** |

## Wspólne, obowiązkowe moduły produktu

| ID | Przynależność | Mechanizm | Kryterium odbioru | Status |
|---|---|---|---|---|
| `BUILDER-CORE-IR` | 0.50 | Common ocaf-builder-core typed agent DAG, schema, immutable project IDs, versions, agent manifest compiler and honest Code/NoCode capability negotiation. | Graph→DSL→Graph deterministic round-trip for supported DSL, schema version conversion and rejected unrepresentable constructs | **BRAK IMPLEMENTACJI** |
| `BUILDER-CODE-ENGINE` | 0.51 | A real source-code editing product with diagnostics, reusable agent graph libraries and controlled build/sandbox/debug/deploy; not merely an LLM coding agent. | Edit typed agent project, compile and run in constrained executor, edit history and durable revert | **BRAK IMPLEMENTACJI** |
| `BUILDER-NOCODE-ENGINE` | 0.51 | A real visual NoCode editor with palette, draggable nodes, typed edges, permission prompts, simulation and publish gating, not a static picture. | UI interactions persist correct typed graph, generate valid supported DSL, unsupported edge/action denied | **BRAK IMPLEMENTACJI** |
| `BUILDER-CONVERSION` | 0.51 | Bidirectional supported-subset Code↔NoCode round trip; unsupported arbitrary Rust cannot silently claim semantic equivalence. | Executable program subset preserves properties and requires explicit lossiness warning | **BRAK IMPLEMENTACJI** |
| `BUILDER-SECURITY-PUBLISH` | 0.55 | Agent graph permissions/cost guard, sandbox execution, run receipts, versioned signed export, package verification and explicit user-approved deployment. | Rogue tool edge, cyclic effects, malicious import, cross-tenant export or unauthorized deployment denied | **BRAK IMPLEMENTACJI** |
| `BUILDER-TEMPLATES-DIAGNOSTICS` | 0.50 | Project gallery/template versioning, syntax hints, typed error messages, debugging trace and reproducible agent project snapshots. | Template creates real valid editable project; broken template fails validation and never publishes | **BRAK IMPLEMENTACJI** |

## Granica zakresu: Agents Builder ≠ Agent

**Agents Builder Code:** użytkownik tworzy **projekty agentów**, edytuje źródło, moduły i graf, kompiluje/uruchamia w sandboxie oraz zapisuje własną wersję aplikacji. **Agents Builder NoCode:** ten sam format projektu i prawdziwy silnik wykonawczy, lecz edycja przez interaktywny UI drag/drop, typed connectors, konfigurację modeli i narzędzi bez pisania kodu.

Platformy Cloud, Windows, macOS, Android i iOS muszą mieć swoje realne buildy i E2E — brak możliwości uruchamiania Firecracker pod Androidem/iOS nie uprawnia do obiecywania Linux KVM na telefonach. Mobile Code może używać *ograniczonego* DSL oraz autoryzowanego zewnętrznego kompilatora; nie obiecywać dowolnego natywnego kompilowania każdej biblioteki na iOS.

**Niezmienniki:** wspólna typed-IR/DSL; autoryzacja i izolacja tenantów; wybór modeli i MCP/Skills; edytowalny Code i NoCode projekt; wersjonowanie; symulacja/test/sandbox; debugowanie i logi; wymagana świadoma autoryzacja działań zewnętrznych; eksport/import; sprawdzalny artefakt publikacji; brak samowolnych płatności/wdrożeń.

## Dowody implementacji (stan inwentaryzacji 2026-10-15)

- W repo aktualnie istnieje pojedynczy `Cargo.toml` dla `ocaf_runtime`, `src/executor`, `src/rpc` i pliki testowe. Brakuje katalogów ze źródłami aplikacji `agents-builder-*`, `ocaf-builder-core`, `src/builder`, a także projektów Tauri/SwiftUI/Compose. **Wynik: wszystkie 10 wariantów = `NOT_IMPLEMENTED`.**
- To, że roadmapa opisuje `ocaf-builder-core` lub `Omni*`, nie dowodzi skompilowanego Agents Builder. Przy zmianie statusu wymagany jest rzeczywisty commit, workflow platformowe, dowód uruchomionej aplikacji i test Code oraz NoCode osobno.
- Te kryteria pochodzą z pierwotnego planu; przywrócenie ich zapisów **nie zwiększa liczby głównych etapów** (195). Nowe, naprawdę później odkryte funkcje dostają wyłącznie numery od 1.95 w górę.

## Raportowanie po zakończeniu KAŻDEGO etapu

W tym repo na zakończenie etapu aktualizuje się główny `ROADMAP.md`, jego lustrzany plik `docs/ROADMAP_FULL_ECOSYSTEM_2026.md`, maszynowy rejestr `docs/roadmap/canonical_v11_inventory.json`, tę macierz produktów, `docs/roadmap/STAGE_CHANGELOG.md` oraz manifest dowodów konkretnego milestone (commit, run CI, SHA256 artefaktów, realność backendu, liczbę hostów, falsyfikowalne ograniczenia). Jeżeli produkt nadal nie istnieje: pozostaje **NOT_IMPLEMENTED**, bez dopowiadania pass.
