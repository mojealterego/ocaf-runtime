# 0.41 — granica dowodowa awarii brokera, VM i PostgreSQL

Istniejący workflow `stage039-real-firecracker-jailer.yml` uruchamia na
rzeczywistym hoście KVM dwie VM. Sprawdza podpisaną kwarantannę procesu
gościa przy działającym drugim tenancie, a także SIGKILL procesu brokera
podczas działania guest Aya i host Aya. Przebieg 40826747945 zakończył się
sukcesem w tym zakresie.

Osobny workflow `stage041-physical-pitr-cross-kernel.yml` sprawdza odtworzenie
PostgreSQL z fizycznego basebackup i WAL na drugim kernelu Linux. Te dowody
nie dotyczą wspólnego okna awarii. `stage041_fault_evidence.py` wymaga
konkretnych pól obu prób i wystawia tylko `SEPARATE_FAULT_LABS_VERIFIED`.
`broker_vm_postgres_integrated_fault_tested=false` i
`stage_041_complete=false` są celowe. Administracyjny odbiór symulacyjny
właściciela nie zmienia zakresu pomiaru. Aby uzyskać dowód integracyjny,
PostgreSQL musiałby działać w tym samym scenariuszu awarii co broker i VM,
a jego zapisy i odzysk musiałyby być zmierzone w jednym przebiegu.
