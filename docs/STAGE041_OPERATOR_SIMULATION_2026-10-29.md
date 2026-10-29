# 0.41 — decyzja właściciela o odbiorze przez laboratoria i symulację

Właściciel 2026-10-29 zaakceptował laboratoria i symulację jako zastępczą
podstawę odbioru 0.41 z jawnymi ograniczeniami. Ta decyzja nie jest wynikiem
testu i nie stanowi kryptograficznego podpisu właściciela.

## Wykonane próby o potwierdzonym zakresie

- KVM Firecracker, podpisana kwarantanna gościa z przetrwaniem drugiej VM,
  awaria podpisującego brokera z Aya gościa i hosta: CI `40826747945` PASS.
- Fizyczny PostgreSQL PITR po awarii na drugim efemerycznym kernelu:
  CI `38738198412` PASS (transfer przez artefakt; brak offsite).
- Zaszyfrowana AES-256-GCM kopia PostgreSQL i odtworzenie na drugim
  efemerycznym kernelu: CI `40827727163` PASS. Tampering, zły klucz,
  odwołany klucz i cofnięta generacja są odrzucane.
- Krótka diagnostyka powtarzanego PITR po atomowej publikacji segmentów WAL:
  CI `40875639297` PASS. Ten przebieg trwał 30 sekund; nie zalicza 30 minut.

## Wyniki wymagane do administracyjnego zamknięcia

- CI `40875776594`: rzeczywiste osobne próby powtarzanego PITR przez
  30, 60 i 120 minut; RTO zmierzone od żądania zatrzymania PostgreSQL do
  weryfikacji stanu po odtworzeniu. Wynik w toku.
- CI `40876105501`: 60 minut rzeczywistej ograniczonej kolejki telemetrii
  Rust z awarią i wznowieniem odbiorcy. Wynik w toku. Jest to laboratorium
  procesu, nie OTLP ani pełna telemetria runtime.

Stan teraz: `SIMULATION_ACCEPTANCE_PENDING_RESULTS`.

## Trwałe granice techniczne

Powyższe laboratoria nie mają jednego wspólnego scenariusza awarii brokera,
VM i PostgreSQL. Weryfikator `stage041_fault_evidence.py` zachowuje
`broker_vm_postgres_integrated_fault_tested=false`. Nie udowodniono 60 minut
FPR/FNR kwarantanny gościa, rzeczywistego przeciążenia eksportu OTLP,
zewnętrznego S3 Object Lock, niezależnego KMS, trwałego obcego hosta,
przywrócenia pamięci Titans/CRDT ani rzeczywistego rozliczenia płatności.
Pozycje 0.41.5–0.41.14 pozostają w Roadmapie bez usuwania lub renumeracji.

Ukończenie administracyjne po zaakceptowanych próbach nie zmienia
`stage_041_complete=false` w raportach technicznych ani
`production_approved=false`. Decyzja produkcyjna nadal wymaga dowodów
odpowiadających tym otwartym wymaganiom.
