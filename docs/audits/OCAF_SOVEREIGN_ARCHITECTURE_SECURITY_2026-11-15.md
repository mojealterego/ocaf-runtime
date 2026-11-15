# OCAF Runtime — przegląd bezpieczeństwa architektury suwerennej (2026-11-15)

## Podstawa i zakres

Źródło: dostarczony dokument **Architektura i Kod OCAF Runtime.PDF**, sekcje „Globalna topologia”, „CRDT Offline-First”, „TRS-SEAL”, „EvidenceReceipt” oraz „Macierz 8 × 4”. Jest to **specyfikacja referencyjna**, a nie wynik potwierdzonych testów urządzeniowych. Porównano ją z bieżącym kodem repozytorium `mojealterego/ocaf-runtime`, bez zastępowania istniejących implementacji.

### Wdrożone w tym przyroście

- `crates/ocaf-transport-crdt/src/crdt.rs`: izolacja per tenant, ograniczone rozmiary, unikalność dot (actor,counter), zegary wektorowe, operacje usunięcia (tombstone), zachowanie historii przyczynowej, deterministyczne rozstrzyganie współbieżnych maksymów, atomowe scalanie i 12 testów źródłowych Rust.
- `src/evidence_gate.rs`: weryfikacja Ed25519 z wykorzystaniem już istniejącej zależności OpenSSL, podpisywany transkrypt z separacją domeny i prefiksami długości, oddzielne klucze publiczne dla czterech poziomów, walidacja struktury dowodów, domyślna odmowa awansu CI/DEVICE/PRODUCTION bez zewnętrznego weryfikatora i 6 testów źródłowych Rust.
- `scripts/tests/test_crdt_reference.py`: deterministyczny **niezależny model referencyjny Python**, 4 testy wykonane lokalnie, 1000 ziaren, 3000 porównań replik PASS. SHA-256 zbioru wyników: `e3a6e0ff8c11c4445d79c3ffad721c4f7190fc80d4ec68fb5352035db9e90b39`.
- `docs/evidence/OCAF_EVIDENCE_ED25519_VECTOR_2026-11-15.json`: niezależny wektor Ed25519 wygenerowany i zweryfikowany biblioteką Python cryptography; test Rust weryfikuje podpis i dokładne bajty transkryptu **dopiero po uruchomieniu cargo**.
- Python cryptography: 1000 deterministycznych mutacji podpisanego komunikatu odrzuconych; 4 lokalne testy modelu PASS. Nie jest to uruchomienie Rust.

### Znane luki bezpieczeństwa w kodzie referencyjnym źródła

| Fragment źródła | Problem | Wymagane postępowanie |
|---|---|---|
| TRS-SEAL AEAD | Nonce = seq + dolne 32 bity epoki; ponowne użycie po restarcie/zmianie epoki; replay konsumowany przed uwierzytelnieniem | Trwały licznik nonce, związanie pełnej epoki z kluczem, uwierzytelnienie przed replay |
| PostgreSQL fencing | Akceptacja `write_epoch > leader_epoch`, LSN aktualizowany przed zapisem; brak transakcji PostgreSQL | Fencing na autorytatywnym liderze, atomowy WAL + commit, brak uznawania przyszłych epok |
| ZeroCopyBuffer | `unsafe impl Sync` bez synchronizacji, brak sprawdzenia NULL/rozmiaru alokacji, niezweryfikowana pojemność odczytu | Bezpieczne API bufora, atomowy protokół publikacji, brak nieuzasadnionego `Sync` |
| Aya eBPF tracepoint | `return 1` z tracepoint nie blokuje syscall | Egzekwowanie przez seccomp/LSM, osobno telemetria eBPF |
| MCP URL | Filtr pomija wiele prywatnych adresów IPv4/IPv6 i DNS rebinding | Resolving+pinning adresów, pełne deny CIDR, ponowna walidacja redirectów |
| FRI | Sama ścieżka Merkle nie dowodzi poprawności STARK | Nie nazywać jej pełnym weryfikatorem STARK |
| GGUF | 24-bajtowy nagłówek nie weryfikuje wag/tensorów | Oznaczyć `HEADER_ONLY`; nie zatwierdzać modeli |
| NoCode → Code | Wstawienie komentarza nie jest transformacją AST | Odrzucenie transformacji bez semantycznej odwracalności |
| SLSA sample | SHA `e3b0c442...` to hash pustego pliku, a nie binarium wydawniczego | Wymaga realnego artefaktu i weryfikowalnej atestacji |

### Status dowodowy — bez zawyżania

- Nowe moduły Rust: **PARTIAL / IMPLEMENTED_UNVERIFIED**; kod i testy zapisane, ale kompilacja Rust i `cargo test` nie zostały wykonane.
- Testy Python: **TESTED_LOCAL wyłącznie dla niezależnych modeli referencyjnych**; nie potwierdzają zgodności wykonania Rust.
- `CI_VERIFIED`, `DEVICE_VERIFIED`, `PRODUCTION_APPROVED`: **NIEPOTWIERDZONE**.
- Dotychczasowy `StageLedger::complete` przyjmuje samodzielnie zadeklarowane `PASS`; nowy `EvidenceGate` jest oddzielnym mechanizmem i **nie jest jeszcze obowiązkową bramką** istniejącego rejestru etapów. Nie wolno traktować rejestru jako podpisanego źródła prawdy.
- CRDT wymaga uwierzytelnienia tożsamości nadawcy, autoryzacji tenant/key, trwałego magazynu operacji oraz walidacji wektorów przyczynowych w transporcie. Nie implementuje jeszcze Zenoh network adapter ani pełnej odporności na sfałszowane konteksty przyczynowe.
- Nie migrowano istniejącego `Cargo.toml` do 10 nowych crate'ów. Istniejący workspace i `Cargo.lock` zachowano, aby nie wywołać regresji lockfile oraz wersji Rust.

### Odbiór wymagany po przywróceniu narzędzi

```bash
python3 -m unittest discover -s scripts/tests -p test_crdt_reference.py -v
cargo fmt --all -- --check
cargo test --locked --no-default-features -p ocaf-transport-crdt crdt::tests
cargo test --locked --no-default-features --lib evidence_gate::tests
cargo clippy --locked --all-targets --no-default-features -- -D warnings
```

W środowisku realizacji nie ma `rustc`, `cargo` ani `rustfmt`, a DNS do dystrybucji Rust nie działa. Nie ma dowodu zaliczenia powyższych poleceń Rust. Weryfikacja dowodów zewnętrznych CI, TPM/TEE i SLSA wymaga rzeczywistych usług i odpowiednio przypiętych kluczy. Symulacja nie może zastąpić hardware attestation.
