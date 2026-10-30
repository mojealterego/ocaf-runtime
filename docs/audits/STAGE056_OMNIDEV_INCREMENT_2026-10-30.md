# Etap 0.56 OmniDev — kolejne wdrożenie częściowe (2026-10-30)

## Źródło wymagań

Historyczny etap 0.56: OmniDev Architect (Tree-sitter CST, repo graph), OmniDev Security Critic, Firecracker Executor, generator oraz bounded repair loop. Źródło: dostarczony `ocaf-runtime_ROADMAP(1).PDF` i dotychczasowe `ROADMAP.md` (PDF06/40: Analiza Architektury Systemu OCAF(4).PDF s.15–17), audyt `docs/audits/PDF18_OCAF_OMNIDEV_SEMANTIC_REVIEW_2026-10-16.md`. OmniDev jest odrębnym rojem; nie zastępuje produktów Agents Builder.

## Implementacja i rzeczywiste granice

| Element | Wdrożony kod i dowód | Ograniczenie |
|---|---|---|
| Admission gate, poprzedni commit | `scripts/omnidev_patch_gate.py`: realny `git apply --check`, przypięty HEAD, polityka ścieżek i budżetów; 10 testów | Nie uruchamia kodu i nie zatwierdza merge |
| Workspace dependency graph | `scripts/omnidev_workspace_graph.py`: realne `cargo metadata --locked --offline --no-deps`, topologiczny DAG wewnętrznych crate'ów, SHA-256 grafu, odrzucanie cykli, duplikatów i ścieżek poza root | Graf pakietów Cargo, **nie** Tree-sitter CST/AST, symbol search ani analiza wpływu na poziomie funkcji |
| Security Critic — wstępna analiza | `scripts/omnidev_security_critic.py`: po pozytywnej walidacji diff skanuje dodane linie na konkretne ryzyka (unsafe, TLS verification bypass, shell execution, dynamic code, potencjalne sekrety); SHA-256 związany z zatwierdzonym diff; brak automatycznego bezpieczeństwa | Heurystyka, podatna na obfuskację. Nawet brak findings kończy się `SECURITY_REVIEW_REQUIRED`; potrzebna niezależna kontrola uprawnień, injection i semantyki |
| Testy negatywne | `scripts/tests/test_omnidev_workspace_graph.py`, `scripts/tests/test_omnidev_security_critic.py` oraz wcześniejsze testy gate: 27 testów łącznie, uruchomione lokalnie `unittest` i PASS | Testy `cargo metadata` na prawdziwym workspace wymagają runnera Cargo w CI |
| CI | istniejący `.github/workflows/rust.yml`: Python unittest + po Clippy uruchomienie realnego grafu workspace Cargo, bez wykonywania kodu patchy | CI dla nowego commitu wymaga oddzielnej weryfikacji; `cargo fmt/clippy/test` nie były lokalnie dostępne |

## Otwarta bramka 0.56

- Realny Tree-sitter CST/AST, symbol index + incremental updates + impact analysis.
- Typowane role / ich kontrakty i autoryzacja tenantów; bounded repair loop, checkpointy, idempotencja, audit receipts.
- Firecracker/KVM guest executor, podpisane artefakty, rzeczywiste compile/test w guest, negatywne testy sandbox/egress i odzyskiwanie.
- Formalna kontrola aktualizacji chronionego `main` i testy E2E bez deklarowania fikcyjnego operator acceptance.

**Status kanoniczny: `PLANNED_OR_UNVERIFIED` (udokumentowany częściowy przyrost `IMPLEMENTED / LOCAL_TESTED`, pełne kryteria otwarte).** `production_approved=false`. Nie modyfikować statusu całego etapu na COMPLETE ani statusów 0.57–1.14.

## 2026-10-30 — typed Rust DAG, fencing i bounded recovery (kolejny przyrost)

- **Kod:** `src/omnidev_workflow.rs` załączony do `src/lib.rs`; `tests/omnidev_workflow.rs`.
- **Typowane role:** Planner, Architect, Coder, SecurityCritic, Tester, Verifier, Recovery; kontrakty wejściowe i wyjściowe, izolacja tenantów i ograniczenia liczbowe.
- **Graf i harmonogram:** sprawdzenie zależności/cykli, tylko ukończone prerequisites odblokowują kolejną rolę; maksymalnie 32 węzły, 8 prób/etap, leasing <=300 sekund, limit kosztów i termin.
- **Fencing:** wymagany monotoniczny epoch od zaufanego koordynatora, powiązanie leased attempt z wynikiem, brak auto-retry po timeout, rozliczenie jawne przez rolę Verifier.
- **Audyt:** lokalny sha256 hash-chain z licznikiem. Wymaga zewnętrznego przypięcia head; nie jest zewnętrznym niepodrabialnym podpisem.
- **Testy:** 10 scenariuszy kontraktowych, w tym powtarzanie poprawki, inny tenant, inna rola, stale epoch, fail closed timeout, dzienny budżet, cykl DAG.
- **Otwarte:** brak trwałości i koordynacji wielohostowej, podpisanych receiptów, powiązania mTLS z rolami i uruchamiania Firecracker/WASI. Nie jest jeszcze technicznie zakończonym etapem 0.56; `production_approved=false`.
- **CI:** wyniki potrzebują zatwierdzenia po zakończonym workflow dla aktualnego SHA.

## 2026-10-30 — uzupełnienie granic budżetu i izolacji metadanych

- Arytmetyka dostępnego budżetu używa `checked_sub` i odrzuca przypadki przekroczenia `u64::MAX`; wcześniejsze `saturating_add` mogło błędnie zaakceptować koszt przy maksymalnym budżecie.
- Metody odczytu zakończenia, rezerwacji oraz głowy audytu przyjmują zaufany identyfikator tenanta i odrzucają niezgodność.
- Dodano 2 testy regresyjne Rust (przepełnienie oraz izolacja metadanych). Cały etap 0.56 nadal `PLANNED_OR_UNVERIFIED`; `production_approved=false`.
