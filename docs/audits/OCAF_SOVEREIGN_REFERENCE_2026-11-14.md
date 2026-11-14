# OCAF Sovereign Runtime — przegląd 44-stronicowego źródła i moduł symulacyjny

Data źródła: 2026-11-14 (data analizy); PDF użytkownika: `Architektura i Kod OCAF Runtime.PDF`, 44 strony, 411594 bajtów, SHA-256 `c1caa3642308e23c4a4cb111d806fdc8b5124ba6ae637b91abd0f7778c13be85`.

**Klasyfikacja:** `SOURCE_REVIEWED / PARTIAL_CODE_ADDED / RUST_CI_UNVERIFIED`. To nie jest podpisany raport audytora, wynik testów sprzętowych ani deklaracja wdrożenia na 8 platformach.

## Zasada integracji

Głównym źródłem kanonicznych identyfikatorów pozostają `ROADMAP.md`, `docs/ROADMAP_FULL_ECOSYSTEM_2026.md` i `docs/roadmap/canonical_v11_inventory.json`: aktualny manifest deklaruje 334 rekordy etapów głównych, 64 techniczne i `production_approved=false`. PDF opisuje etapy 001–334 jako architekturę referencyjną; **nie jest dowodem ukończenia tych etapów**. Nie zastępować istniejącego workspace Cargo nowym, nie migrować bez testów całego root crate z edition 2021 na 2024 i nie importować beta wersji Zenoh/Wasmtime z PDF bez dowodu kompatybilności.

## Przegląd komponentów źródłowych

| Sekcja PDF | Repozytorium / obserwacja | Status w tym przyroście |
|---|---|---|
| s. 1–3: topologia granic zaufania, workspace `crates/*` | W repo istnieje root Cargo workspace, runtime 2021 i moduły wykonawcze; PDF proponuje alternatywny układ 2024 | `ANALYZED_NO_RESTRUCTURE` |
| s. 3–9: `TaskEnvelope`, `TaskAggregate`, CAS, lease, receipt | `src/task_cas.rs`, `src/task_cas_sqlite.rs` mają atomowe kontrolki; snippet PDF oblicza `action` z już zmienionego `self.state`, co gubi stan przed przejściem | `EXISTING_SOURCE`; nie kopiowano błędu |
| s. 10–11: dyspozytor etapów 001–207, 64 podetapy | Utrzymać historyczne i nowe ID; brak handlera nie oznacza ukończenia przez sam wpis w rejestrze | `OPEN_FULL_ACCEPTANCE` |
| s. 11–15: CRDT / PostgreSQL / fencing | PDF zawiera przykład, w którym `last_committed_lsn` rośnie **przed** trwałym zapisem, a `data` jest ignorowane; niedopuszczalne do produkcji. Zachować istniejące granice `src/postgres_ha_contract.rs` i realny protokół transakcyjny | `REQUIRES_SECURE_IMPLEMENTATION` |
| s. 15–17: `ZeroCopyBuffer` | Wzorzec `unsafe impl Send/Sync`, surowego wskaźnika bez pełnej synchronizacji oraz odczytu `header.len` bez ograniczenia pojemnością może prowadzić do UB/OOB. Nie przeniesiono do runtime | `REJECTED_UNSAFE_SNIPPET` |
| s. 18–20: TRS-SEAL | Referencja wyznacza nonce z `(epoch as u32)` i sekwencji, co może doprowadzić do ponownego użycia nonce po utracie wyższych bitów epoki; ponadto **zużywa okno replay przed uwierzytelnieniem**. Istniejące `src/trs_seal.rs` i `src/trs_seal_sqlite.rs` wymagają osobnej kompatybilnej migracji, a nie podmiany | `REJECTED_CRYPTO_SNIPPET` |
| s. 20–22: Ricci heuristic i hybrydowy PQC | `crates/ocaf-transport-crdt/src/ricci_routing.rs` już zawiera bezpieczniejszy, doradczy algorytm. Kod PDF nie oblicza rzeczywistego Wassersteina-1; propozycja PQC jest tylko funkcją HKDF nad **założonymi** shared secrets, nie implementacją ML-KEM | `PARTIAL` |
| s. 23–27: WASI, Firecracker, eBPF, SSRF | Istnieją kontrakty sandboxu; eBPF tracepoint zwracający `1` nie stanowi mechanizmu blokowania syscall w Linuksie. URL-filter hostowy z PDF nie obejmuje DNS rebinding, przekierowań i sieci prywatnych | `REQUIRES_RUNTIME_PROOF` |
| s. 28–31: GGUF, Merkle/FRI, HDC | Przykład GGUF odczytuje tylko nagłówek; sama weryfikacja gałęzi Merkle nie jest dowodem STARK/FRI. Brak hardware HIL w treści kodu | `SCOPED_RESEARCH` |
| s. 32–38: UI 8 × 4, UniFFI i platformy | Tabela jest matrycą wymagań, nie dowodem binariów AAB/APK/IPA/MSIX/HAP ani testów natywnych na wszystkich 8 systemach. Mobilny przykład podaje pusty delegation token | `UNVERIFIED_NATIVE_BUILDS` |
| s. 39–40: symulator zdarzeń i fault injection | Dodano `src/deterministic_fault_simulator.rs` w istniejącym Rust runtime: virtual clock, LCG, deterministic heap, partycje per tenant, utrata/duplikacja/mutacja, limity, nierozmywanie kolejki. Uzupełniono `src/lib.rs`, Rust CI | `SOURCE_IMPLEMENTED / RUST_TEST_UNVERIFIED` |
| s. 40–43: EvidenceReceipt, SLSA i statusy | Wzór SLSA z pustym digest SHA-256 i przykładowym linkiem do obcego repo nie poświadcza binariów. Tabela `PRODUCTION_APPROVED` dla symulatora w PDF nie jest dowodem operatora. Nowy `SimulationReceipt` ma tylko jawny `simulation_only=true` i nie udaje podpisu | `NO_PRODUCTION_GO` |

## Ulepszenie symulatora wobec źródła (s. 39–40)

Oryginalna pętla zdejmuje zdarzenie z `BinaryHeap`, zanim porówna jego czas z `max_virtual_ms`; to gubi pierwsze zdarzenie z przyszłości. Ponadto callback może planować zdarzenia o zerowym opóźnieniu bez twardego limitu kroków, a suma `virtual_clock_ms + delay_ms` i licznik `event_counter` mogą się przepełnić.

Wersja w `src/deterministic_fault_simulator.rs` sprawdza horyzont na `peek()` przed `pop()`, stosuje `checked_add`, egzekwuje limity zdarzeń, bajtów, payload, liczby linków i kroków; rozróżnia zasymulowane wywołania `Primary`, `InjectedDuplicate`, `InjectedCorruption`. Umieszcza wszystkie wyniki w domenowo rozdzielonym SHA-256 transkryptu, bez podpisu i bez uprawnień wykonawczych. Dodano 15 testów jednostkowych źródła Rust obejmujących zdarzenia w przyszłości, stabilną kolejność, izolację tenantów, partycje, callback loops, fault injection, przepełnienia, deterministyczny replay i brak skutków ubocznych.

**Ograniczenia:** nie potwierdzono kompilacji Rust, Clippy, rustfmt ani realnego CI na docelowym SHA; brak zegara sprzętowego, Zenoh transportu i prawdziwego HA. To model, nie test produkcyjny. Nie modyfikowano `Cargo.toml`, `Cargo.lock`, identyfikatorów roadmapy, certyfikatów ani statusów produkcyjnych.

## Minimalne bramki dowodowe

```bash
cargo fmt --all -- --check
cargo test --locked --no-default-features --lib deterministic_fault_simulator::tests
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features
```

Brak wyniku PASS jest blokadą statusu `TESTED_LOCAL` i `CI_VERIFIED`. Symulacja nie może zmienić `DEVICE_VERIFIED` ani `PRODUCTION_APPROVED`.
