# OCAF — Etap 0.39.18 oraz 0.40.1: broker → prawdziwe VM, trzy niezależne kernele PG

**Data:** 15 października 2026. **Gałąź:** `main`. Dalsze prace rozwojowe nie są wstrzymywane przez osobne podpisy audytowe/operacyjne zgodnie z mandatem operatora. Weryfikacja techniczna i niezmyślone dowody pozostają obowiązkowe.

## 0.39.18: autentyczne zarezerwowanie przez broker przed uruchomieniem dwóch Firecracker

- [**GitHub Actions 38739812121: SUCCESS**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121). Poprzedni także zielony [38739569201](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739569201).
- [Artefakt 11975256452](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812121/artifacts/11975256452), uploader ZIP SHA-256 `887c21aaea9a6c5712bc0c118f90ad5cff0fef57c45a672137d533b845614d4e`.
- Osobne prywatne AF_UNIX sockets; odczyt SO_PEERCRED z **jądra Linux**, polityka jednego tenant/agent principal na socket; dwie właściwe rezerwacje zasobów w SQLite, odrzucanie replay i fencing epok.
- Każdy manifest wymaga `network=none`, `guest_egress=false`, `jailer=true`, SHA-256 `firecracker`, `guest_kernel` i `guest_rootfs`; rezerwacje są trwale zatwierdzone **PRZED** startem obu Firecracker.
- Potem rzeczywiście uruchomiono dwa równoczesne, różne jailer/UID/cgroup/VM/tenant guest Linux. Dodatkowa warstwa wiąże wskazania rezerwacji i manifestów z realnymi dowodami boot guest kernel, inodami cgroup i SIGKILL VM A, w tym dalszym działaniem B. Następnie obie rezerwacje zostały zwolnione.
- Log marker `STAGE039_REAL_AUTHENTICATED_BROKER_TWO_JAILED_VM_BOOT_AND_FENCING_BOUND`. Niezależny test nieprawidłowych JSON/principal/lease/digest/replay i nieautentycznego guest cgroup boot fail-closed: [preflight CI 38739812535 SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38739812535).

**Zakres graniczny**: `scripts/stage039_broker_vm_binding.py` używa broker AF_UNIX/SO_PEERCRED lokalnie, a nie niezależnego zdalnego auth; tworzy rezerwacje z zaufanego CI harnessu, nie z podpisanego zewnętrznego RPC. Dowiedziono **broker → kernel VM boot/fault**, ale NIE podpisanego modułu WASI wykonywanego wewnątrz Firecracker. Nie modyfikować tych dwóch znaczeń na potrzeby marketingu ani release.

## 0.40.1: trzy oddzielne systemy Linux/KVM realizują pełne lokalne PostgreSQL failover

- [**GitHub Actions 38740089797: SUCCESS**](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797) — trzy osobne jednoczesne matrix runner jobs Linux `a,b,c` i czwarty niezależny porównujący wyniki. **Cztery zadania PASS**.
- Każde środowisko rzeczywiście uruchamia **trzy odrębne postmaster PostgreSQL 16** (primary A, standbys B/C), streamuje WAL, wymusza `pg_ctl -m immediate stop` A, promuje B, przełącza C na B, weryfikuje `pg_stat_wal_receiver.status=streaming` i replay transakcji.
- **Łącznie dziewięć prawdziwych postmasterów** w trzech izolowanych runner host kernels, każdy rozpoznawalny osobnym `/proc/sys/kernel/random/boot_id`. Czwarty job odrzuca identyczne boot ID, brak dowodów i fałszywe deklaracje produkcyjne.
- [Źródłowy host a 11975511424](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797/artifacts/11975511424) ZIP SHA-256 `564105009a5349df0e9d408965c83c62e3dd659a718ca3deb58605f3de94d70f`.
- [Źródłowy host b 11976365885](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797/artifacts/11976365885) ZIP SHA-256 `b45c8fb185ca06ce631a254232713a7892041a43c57dbf802a55a8a21db56968`.
- [Źródłowy host c 11976001249](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797/artifacts/11976001249) ZIP SHA-256 `b9a1fcbbf6f13fff4291983af8059e71f3403b1607dfa1f18af18979b3204080`.
- [Porównanie hostów 11975856209](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740089797/artifacts/11975856209) ZIP SHA-256 `01360a9f20018a540d4548d2c7c35337d59e7a47ea6401a5e0224fd09d59b708`.
- Znacznik: `STAGE040_THREE_ACTUAL_DISTINCT_LINUX_KERNELS_PG_FAILOVER_PROVED_NOT_INTERHOST_HA`.

**Zakres graniczny**: nie ma replikacji PostgreSQL między **różnymi** runnerami, bo każdy ma własny lokalny zestaw trzech procesów A/B/C. Nie przetestowano zewnętrznego transferu TCP/mTLS między hostami, host poweroff, nowego primary na innym fizycznym hoście ani rzeczywistego split-brain prevention.

## Następne techniczne wymagania

1. Rzeczywiste **podpisane wykonanie WASI wewnątrz jądra gościa Firecracker**: pinned Wasmtime/WASI VM image + podpisany manifest + wejście/wyjście IPC, poprawna weryfikacja digest/signature i guest exit-code/receipt. Test negatywny modułu z nieprawidłowym podpisem i innego tenant.
2. Prawdziwa **międzyhostowa** replikacja PostgreSQL, fencing starego primary, utrata łączności i zweryfikowane bounded takeover time; oddzielne hosty docelowe bez płatnej infrastruktury wymagają udostępnionego mechanizmu połączenia lub autoryzacji istniejących.
3. Długotrwałe równoległe obciążenie PostgreSQL/Firecracker i pomiar p95/p99, host/guest RAM, WAL backlog, RTO/RPO oraz automatyczne czyszczenie po błędach.

**Etapy 0.39 i 0.40 pozostają technicznie w toku. Bez podpisów jako warunku rozwoju — tak; bez rzeczywistych testów lub z fikcyjnymi wynikami — nie.**

## 0.40.2 — 30 sekund rzeczywistego obciążenia pgbench z dwiema replikami WAL

**Rzeczywisty CI run 38740936947 — SUCCESS**: [GitHub Actions](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947) [artefakt 11975657701](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947/artifacts/11975657701), ZIP SHA-256 `2e178799f91b297350fa9611c9d12ea0a4c28d860a299ced2ff2285fbb621472`.

1. Nowy, nieuprzywilejowany **prawdziwy PostgreSQL 16** primary A oraz dwa standby B/C na tym samym kernelu Linux; `pgbench -i -s 1` zakłada dane obciążeniowe.
2. W trakcie realnej replikacji WAL uruchomiono **osiem współbieżnych klientów pgbench** na **dwóch worker threads przez 30 sekund**; `pgbench` potwierdził zakończenie, liczbę rzeczywiście przetworzonych transakcji oraz dodatni TPS.
3. Po zakończeniu obciążenia serwer A, replika B oraz replika C zwróciły **dokładnie te same skumulowane salda** tabel `pgbench_accounts`, `pgbench_branches` i `pgbench_tellers`. Walidator wymaga odtworzenia transakcji przez obie działające repliki B/C.
4. Wymuszono **`pg_ctl stop -m immediate`** primary A. B został promowany do primary, C przepięto na nowy primary i potwierdzono dalszą replikację. Bez takiej pozytywnej obserwacji CI zgłasza błąd.
5. Skrypt ogranicza maksymalny czas obciążenia, liczbę klientów i zapisuje czas, rzeczywisty licznik transakcji i raportowany TPS do maszyny dowodowej. Nie zgłasza sztucznego RPO/RTO.

Dodatkowe testy jednostkowe: **4 PASS** — m.in. odrzucanie brakujących danych pgbench/TPS/duration, niedopasowane salda replik oraz błędne parametry fizycznego basebackup. Podczas pierwszego przebiegu **[38740603200 — FAILED](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740603200)** rzeczywisty `pg_basebackup` przekroczył dozwolony czas po przygotowaniu dużej bazy `pgbench`. Naprawiono konfigurację poprzez wymuszenie rzeczywistego checkpointu `--checkpoint=fast` oraz streaming `--wal-method=stream`, a pełny [ponowny test 38740936947 zakończył się SUCCESS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38740936947).

**Granice:** trzy procesy PostgreSQL znajdują się na **jednym kernelu**; to test WAL churn, replikacji i awarii procesu po obciążeniu, nie długotrwały test tygodniowy, fizyczny host poweroff ani prawdziwe interhost mTLS HA.
