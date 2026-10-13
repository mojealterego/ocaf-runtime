# Etap OCAF 0.38 — protokół odbioru operatora (do wykonania na staging)

**Status:** NIEPODPISANY / NO-GO. Formularz nie jest poświadczeniem, że
testy produkcyjne lub niezależny audyt zostały przeprowadzone.
Nie wprowadzaj danych prywatnych kluczy, haseł ani certyfikatów
zawierających sekrety do GitHub ani do rozmowy.

## Wymagana infrastruktura

- Linux x86_64, jądro z `memfd_create`, `F_ADD_SEALS`,
  `PR_SET_PDEATHSIG`, `/proc/self/fd`; administrator serwera.
- systemd + cgroups v2 z kontrolerami memory/cpu/pids; osobne
  konto `ocaf` i operator-installation approved Wasmtime.
- Zamknięte staging: minimum dwa logiczne tenanty, dwóch agentów,
  dwa endpointy TLS/mTLS, kontrolowane dane syntetyczne.
- Możliwość wyłączenia systemd, wymuszenia awarii i symulacji
  przerw sieciowych WYŁĄCZNIE w wydzielonym staging.
- Opiekun rotacji klucza publikatora, odrębny audytor i rejestr
  podpisanych dowodów; klucze prywatne nie opuszczają KMS/HSM.

## Przed wdrożeniem (operator)

1. Zweryfikuj rev GIT `git rev-parse HEAD`, działanie `cargo test --locked`,
   `scripts/wasmtime_acceptance.sh` i status dwóch workflow na tym SHA.
2. Zamknij instalację Wasmtime: root-owned katalogi bez dostępu
   zapisu dla innych użytkowników; dokumentuj SHA-256 binarium.
   Loader dodatkowo egzekwuje brak hardlinków i brak group/world-write.
3. Skopiuj **po przeglądzie** szablon
   `deploy/systemd/ocaf-coordinator-wasi.service.example` do docelowej
   jednostki systemd, ustaw właściwe ścieżki.
4. Zadbaj o owner-only `0600` i właściciela EUID konta usługi dla
   czterech plików WASI: modułu, manifestu, podpisu i PEM publikatora.
   Sprawdź katalog nadrzędny i reguły ACL. Klucza prywatnego **nie**
   zapisuj na hoście wykonującym niezaufany kod, o ile nie jest to
   konieczne do osobnego procesu podpisującego.
5. Zanotuj `systemctl show <unit> -p KillMode -p MainPID -p MemoryMax
   -p TasksMax -p CPUQuotaPerSecUSec -p FragmentPath`. Sprawdź
   `/sys/fs/cgroup` oraz brak trybów `delegated` obchodzących limity.

## Evidence / stress

```bash
export OCAF_TEST_WASMTIME=/opt/ocaf/wasmtime/wasmtime
export OCAF_TEST_WASMTIME_SHA256='<approved-SHA256>'
python3 scripts/wasi_host_acceptance.py \
  --cycles 100 --timeout 120 --max-rss-kib 1572864 \
  --max-fds 512 --max-processes 128 \
  --report /var/lib/ocaf/evidence/wasi-038-host.json
```

Podaj w raporcie oddzielnie pomiary z systemd/cgroup
(`memory.peak`, `cpu.stat`, `pids.peak`, `memory.events`),
ponieważ odczyty skryptu są tylko próbkami `/proc`.
Limity w przykładzie wymagają dopasowania do zatwierdzonego SLO.

## Chaos acceptance na izolowanym staging

- Uruchom moduły obu tenantów równolegle; sprawdź brak krzyżowania
  principal, payload, odpowiedzi i receipts w logach autorytatywnych.
- Zabij main PID procesu koordynatora `SIGKILL` przy aktywnym zadaniu;
  sprawdź, że systemd usuwa cały cgroup i prywatny `PrivateTmp`,
  nie tylko lidera. Zlicz pozostałe PID i workspace. Test wyłącznie
  przy wyłączonych rzeczywistych sinkach.
- Restart w środku niepewnego efektu zewnętrznego: bez automatycznego
  ponowienia skutku; logi i fencing/replay powinny umożliwić rozliczenie.
- Działanie przy zajęciu 100% CPU, blisko `MemoryMax`, ostrym FD
  pressure i timeoutach; monitoruj stabilność koordynatora, postęp
  drugiego tenant-a oraz `memory.events` (OOM).
- Kontrolowany zanik połączenia RPC/mTLS i rekoncyliacja po powrocie:
  brak ponownego przyjęcia przegranego lease i cross-tenant replay.

## Rotacja i odwołanie zaufania

1. Przygotuj nową parę kluczy Ed25519 w zatwierdzonym KMS/HSM,
   podpisz *nowy* manifest przypisany do modułu, agenta, tenant-a,
   polityki oraz daty wygaśnięcia.
2. Na każdym węźle wymień atomowo *operator-approved* plik PEM i
   manifest/podpis. Zrestartuj jednostkę w kontrolowanej kolejności
   węzłów (bez nierozliczonych efektów zewnętrznych).
3. Odmów startu przy niekompletnym pakiecie. Sprawdź sukces z
   nowym podpisem, odrzucenie starego klucza i błędnego agent-a,
   brak fallbacku do niezaufanego narzędzia.
4. Zachowaj niezależnie zapis kryptograficznie poświadczonego
   wycofania starego trust anchor; bez automatycznej obsługi CRL
   żaden już uruchomiony proces nie otrzyma zmian bez restartu.

## Formularz podpisania

| Pole | Wypełnia operator |
| --- | --- |
| SHA `main` | DO UZUPEŁNIENIA |
| Linux/kernel/systemd/cgroup | DO UZUPEŁNIENIA |
| Operator instalacji i daty testów | DO UZUPEŁNIENIA |
| Hash Wasmtime, podpis manifestu | DO UZUPEŁNIENIA |
| Artefakt soak JSON + podpis KMS | DO UZUPEŁNIENIA |
| Pomiary cgroup i FD + limit breach | DO UZUPEŁNIENIA |
| SIGKILL/restart/replay, cross-tenant | DO UZUPEŁNIENIA |
| Rotacja/revocation/KMS | DO UZUPEŁNIENIA |
| Raport niezależnego audytora | DO UZUPEŁNIENIA |
| Decyzja GO / NO-GO i podpis | **NO-GO — brak audytu i testów staging** |

**Nie wolno wpisać GO na podstawie samego CI lub niepodpisanego
raportu.** Testów destrukcyjnych nie wykonywać na hostach produkcyjnych
bez odrębnej zgody i planu odzyskiwania.
