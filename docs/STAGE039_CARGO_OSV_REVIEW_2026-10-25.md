# 0.39 — przegląd zależności Cargo w dniu 2026-10-25

Odpytano publiczne OSV `POST https://api.osv.dev/v1/querybatch` dla
konkretnych par `name`/`version` z `Cargo.lock` głównego runtime i czterech
lockfile pod `stage039/`. Przed zmianą cztery pakiety głównego lockfile
miały dopasowane advisory; 55 unikalnych pakietów z lockfile 0.39 nie miało
dopasowania. API alertów Dependabot dla tego tokenu zwraca HTTP 403, więc
ośmiu alertów pokazywanych przez GitHub nie można przypisać do pakietów.

| Pakiet | Wersja z advisory | OSV | Wersja w nowym lockfile |
| --- | --- | --- | --- |
| `ctutils` | 0.4.3 | RUSTSEC-2026-0385 | 0.4.4 |
| `rustls-webpki` | 0.103.15 | RUSTSEC-2026-0382 | 0.103.16 |
| `tokio` | 1.53.2 | RUSTSEC-2026-0386 | 1.53.3 |
| `webpki-roots` | 1.0.9 | RUSTSEC-2026-0381 | 1.0.10 |

SHA-256 pakietów i `rust_version` pobrano z oficjalnych endpointów
`https://crates.io/api/v1/crates/<name>/<version>` dla obu wersji każdego
pakietu. Porównanie `.../<version>/dependencies` nie wykazało zmiany listy
zwykłych zależności i ich zakresów. Wpisano tylko cztery wersje i ich
registry checksum do `Cargo.lock`. Minimalne wersje Rust nowych pakietów
nie przekraczają deklarowanego w repozytorium toolchain 1.99.0.

Ponowne zapytanie OSV po zmianie objęło 441 unikalnych wersji z tych pięciu
lockfile i zwróciło **0 dopasowań w chwili zapytania**. To wynik publicznej
bazy na daną chwilę, nie dowód braku nieznanych podatności ani potwierdzenie
zamknięcia alertów GitHub. `scripts/supply_chain.py --repo .` przeszedł
lokalną kontrolę struktury lockfile. W lokalnym środowisku nie ma `cargo`,
więc `cargo test --locked` i pełny build na tym SHA wymagają CI. Do czasu
zielonego wyniku CI oraz rozstrzygnięcia alertów Dependabot przegląd
zależności 0.39.10 pozostaje częściowy.
