# Odzyskiwanie roadmapy i wielokrotna budowa

Roadmapa oraz manifesty floty są snapshotowane w trzech niezależnych kopiach.
Indeks snapshotów: [`recovery-builds/RECOVERY_INDEX.md`](recovery-builds/RECOVERY_INDEX.md).

Do wielokrotnej budowy służy:

```sh
OCAF_REBUILD_RUNS=3 ./scripts/rebuild_and_verify_fleet.sh
```

Skrypt buduje każdego agenta osobno, zapisuje digesty i zatrzymuje się przy
pierwszym braku toolchainu, manifestu albo artefaktu. Nie można oznaczyć
przebiegu jako ukończonego na podstawie samej dokumentacji.

Zmiany roadmapy muszą być dopisywane append-only i porównywane z manifestem
`snapshot-*.sha256`; przy rozbieżności wymagany jest jawny receipt migracji.
