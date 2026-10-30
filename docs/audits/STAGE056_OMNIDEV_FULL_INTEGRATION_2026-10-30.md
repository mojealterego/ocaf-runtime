# OCAF 0.56 — integracja OmniDev: rzeczywisty kod, symulacja infrastruktury

Data: 2026-10-30. Zakres: odrębny produkt OmniDev, nie dziesięć produktów Agents Builder. Dyrektywa: `docs/roadmap/SIMULATION_ONLY_ACCEPTANCE_POLICY_2026-10-30.md`.

## Macierz funkcjonalności i dowodów

| Obowiązek historyczny | Zintegrowana implementacja | Dowód i granica |
|---|---|---|
| OmniDev Architect AST/CST | `scripts/omnidev_tree_sitter.py`: prawdziwe Rust Tree-sitter CST, pozycje symboli, wywołania, `use`, hash pliku, atomowy przyrostowy indeks, usuwanie nieaktualnych plików | 4 testy AST/CST PASS. `impacted()` to konserwatywne dopasowanie nazw, **nie** pełny type-checker ani symbol resolver |
| Graf zależności repo | `scripts/omnidev_workspace_graph.py` z `cargo metadata --locked --offline --no-deps` | DAG paczek Cargo; bez reprezentacji wszystkich przepływów danych |
| Generator patchy | `scripts/omnidev_pipeline.py`: `Repair`, `generate()`, przypięty Git SHA, jednoznaczny anchor, prawdziwy unified diff, walidacja Python AST / Rust CST, następnie `inspect_patch` | Generuje bezpiecznie ograniczone poprawki; **nie** autonomiczne tworzenie dowolnego kodu z naturalnego języka |
| OmniDev Security Critic | `scripts/omnidev_security_critic.py` (diff), `scripts/omnidev_semantic_critic.py` (Python AST / Rust CST), deny importu dynamicznego, FFI, procesów, wybranych efektów | 3 testy semantyczne i negatywne pipeline PASS na run niżej. Analiza statyczna nie zastępuje pełnego taint/dataflow ani formalnego dowodu bezpieczeństwa |
| Wykonawca testów | `DockerExecutor` rozpakowuje `git archive` do jednorazowego workspace, `git apply`, `docker run` bez sieci, capabilities, root, z limitami pids/mem/cpu, read-only bind, no-new-privileges | **Realny Docker Python E2E PASS** w GitHub Actions; dodatkowy test wygenerowanego patcha Rust z `cargo test --offline --locked` dodany, wymaga osobnego zakończonego CI. Wykonawca nie jest rzeczywistym Firecracker/KVM |
| Orkiestracja i recovery | `src/omnidev_workflow.rs`: typowany DAG, role, tenant, budżety, lease/epoch/fencing, receipts. `Stage056`: SQLite, idempotentne klucze, HMAC audit-chain, trwały status RUNNING i terminal reconciliation, bez ponowienia nieznanego efektu | 12 testów Rust PASS na run niżej (dalszy test replay fencing dodany później). Pamięciowy Rust DAG i Python SQLite nie są jeszcze jednym rozproszonym koordynatorem |
| Bramka operatora i protected `main` | `scripts/omnidev_cli.py`: submit, status, approve, execute, reconcile; operator secret poza patchami, token związany z tenant/task/base/patch/command/image | Nie ma samoczynnego merge/push: chroniona gałąź nie jest modyfikowana przez wykonawcę; sprawdzanie podpisu jest lokalne HMAC, nie sprzętowy HSM |
| E2E i testy negatywne | `scripts/tests/test_omnidev_pipeline.py`, `test_omnidev_semantic_critic.py`, `test_omnidev_tree_sitter.py`, `tests/omnidev_workflow.rs` | SHA, liczba testów, izolacja, rzeczywisty sandbox i bramka CI muszą być zapisane po zakończeniu przebiegu |

## Zweryfikowane dowody

- [Run 41009674517](https://github.com/mojealterego/ocaf-runtime/actions/runs/41009674517): `SUCCESS` — 4 testy parsera Tree-sitter + 12 testów sterownika Rust, fmt i Clippy PASS.
- [Run 41010536240](https://github.com/mojealterego/ocaf-runtime/actions/runs/41010536240): `SUCCESS` — **4 testy Tree-sitter, 15 testów pipeline (w tym rzeczywisty wykonawca Docker dla Python), 3 testy AST/CST Security Critic**, oraz **12 testów Rust**; strict Clippy i fmt PASS. Kod SHA: `ab3305c07659a59d0f44e3a6d54a95771285fd7c`.
- Nowsze rozszerzenia (CLI/recovery, 13. test Rust, kompilacja wygenerowanego patcha Rust w Docker) **nie są objęte poprzednimi PASS**. Dla nich wymagany jest zakończony nowy run.

## Granice akceptacji

Operator wymaga odbioru produkcyjnego **wariantu symulacyjnego** bez obcych hostów, KVM, HSM i zewnętrznego audytu. Kryterium nie uprawnia do twierdzenia, że istnieje sprzętowa izolacja, fizyczne HA czy usługa wdrożona w środowisku produkcyjnym.

**Przed końcowym zamknięciem 0.56** sprawdzić wynik dedykowanego CI dla kompilacji Rust patcha, terminal recovery oraz ochrony przed stale receipt; wtedy dopiero zmienić `main[].status` na `PRODUCTION_ACCEPTED_SIMULATION`, podając SHA i ograniczenia.

## Weryfikacja odbioru symulacyjnego, rozszerzona o rzeczywisty Rust guest

Zweryfikowany [run #41011058519](https://github.com/mojealterego/ocaf-runtime/actions/runs/41011058519) (`SUCCESS`, commit `dce17cb110869f7a2fd3c492d4afb40d9b23fc8a`):
- 4 testy prawdziwego parsera Rust Tree-sitter.
- 22 testy Python pipeline: operator CLI token związany z hashem patcha, zgoda człowieka, SQLite HMAC chain, izolacja tenantów, replay, awarie/recovery, **genuine Docker patch test for Python and Rust** (Rust `cargo test --locked --offline` z poprawką).
- 3 testy Python AST / Rust CST semantycznego Security Critic.
- 13 testów kontraktów Rust (DAG, koszt, tenant, lease, epoch, stale replay), `cargo fmt` i `cargo clippy -D warnings`: PASS.

Status: **`PRODUCTION_ACCEPTED_SIMULATION`** — kompletny **ograniczony wariant symulacyjny**, zatwierdzony dyrektywą operatora bez zewnętrznej infrastruktury. Brak dowodu pełnej autonomicznej generacji kodu i pełnej semantyki Rust/type-checker, fizycznego KVM, rzeczywistych podpisów HSM oraz wdrożenia produkcyjnego. Nowy test negatywny sieci/read-only i poprawka długości komendy jeszcze bez zielonego wyniku w chwili wpisu. `production_approved=false`.
