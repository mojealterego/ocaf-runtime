# Stage 0.39.8–0.39.9 — realne limity cgroup v2 i SIGKILL jailed VM

**Dzień:** 2026-10-15. **Gałąź:** `main`.
**Testowany commit:** `c3caa25e89d3e3153119000c38a1d5b32b98dcb5`.
**Wynik:** prawdziwe próby na efemerycznym uprzywilejowanym runnerze
Ubuntu 24.04 x86_64, nie wdrożenie produkcyjne.

## Oryginalne źródła i integralność

- [GitHub Actions 38713156606](https://github.com/mojealterego/ocaf-runtime/actions/runs/38713156606) — job `live-jailer-kvm-guest`, `conclusion=success`.
- [Artefakt 11961542955](https://github.com/mojealterego/ocaf-runtime/actions/runs/38713156606/artifacts/11961542955) — `stage039-real-firecracker-jailer-linux-evidence`.
- SHA-256 archiwum ZIP z logu `actions/upload-artifact`:
  `b6500a9fd54bb7c14fc37692aeae614218423896b0317d681878bc573fffc6c3`.
- Wcześniejszy eksperyment z parserem `cpu.stat` [38712382654](https://github.com/mojealterego/ocaf-runtime/actions/runs/38712382654) zakończył się FAIL,
  ponieważ kernel udostępnił kropkowane nazwy liczników CPU;
  obsługę tych nazw dodano przy zachowaniu fail-closed walidacji.
- [CI kontraktów 38712596266](https://github.com/mojealterego/ocaf-runtime/actions/runs/38712596266) potwierdziło testy parsera i limitów.

## Potwierdzony przebieg

Kolejne wartości `STAGE039_*_PROVED` pochodzą z rzeczywistych logów
run 38713156606, a nie z mocków:

1. `STAGE039_REAL_KVM_KERNEL_CREATE_VM_PROVED`
2. `STAGE039_REAL_FIRECRACKER_JAILER_GUEST_LINUX_BOOT_PROVED`
3. `STAGE039_LIVE_VM_HOST_CGROUP_AYA_EGRESS_NEGATIVE_AND_RECOVERY_PROVED`
4. `STAGE039_LIVE_KERNEL_CGROUP_RESOURCE_LIMITS_AND_PEAKS_PROVED`
5. `STAGE039_REAL_VM_SIGKILL_CGROUP_REAP_PROVED`

### Hostowy resource envelope

| Egzekwowany plik kontrolny cgroup-v2 | Założona i sprawdzona wartość |
| --- | --- |
| `memory.max` | 805306368 bajtów dla manifestu `mem_mib=256` (768 MiB) |
| `memory.swap.max` | `0` |
| `memory.oom.group` | `1` |
| `pids.max` | `96` |
| `cpu.max` | `200000 100000` (do 2 CPU) |

Kod porównuje faktyczny odczyt limitów po zapisie z planem, a po
starcie VM audytuje `memory.current`, `memory.peak`,
`memory.events`, `pids.current`, `pids.peak`, `pids.events`
i `cpu.stat`, z odrzuceniem OOM, przegięcia limitem PID, zbyt dużych
danych oraz niepoprawnego formatu. Test CI wymaga
`observed_memory_peak_bytes>0` i `observed_pids_peak>0`, bez
publikowania wymyślonych szczegółowych liczb. W raportach przekazywany
jest inode konkretnie utworzonego cgroup.

### Wstrzyknięcie awarii

Po testach boot, Aya i resource envelope proces Firecracker został
zabity sygnałem `SIGKILL`; `waitpid` zwrócił `-9`.
Kontrola `cgroup.events populated=0` na tym samym inode potwierdziła
brak aktywnych procesów w całym drzewie dedykowanego cgroup.
Odrzucane są: fałszywy PID/cgroup, inny exit code, brak pola
`populated`, `populated=1`, podwójne lub fałszywe liczniki.
Zajęty cgroup uniemożliwia uznanie udanej utylizacji.

## Granice dowodu i kolejne wymagania

- To rzeczywisty test **jednego** efemerycznego hosta; nie obejmuje
  dwóch trwałych odrębnych hostów, awarii zasilania, restartu węzła
  ani dwuhostowego przełączenia i trwałości efektów.
- Aya obejmuje hostowy próbnik TCP w cgroup procesu VM.
  Firecracker uruchomiono z `network-interfaces=[]`; **nie**
  udowodniono ograniczania pakietów pochodzących z guest VM.
- Prawdziwe testy guest-initiated network egresu, dwie jednocześnie
  uruchomione VM należące do różnych tenantów, side-channel,
  wielogodzinny soak i niezależny audyt pozostają otwarte.
- Wyniki CI nie są podpisanym odbiorem operatora i nie zatwierdzają
  wdrożenia produkcyjnego.

**Stan zarządczy:** `0.38 COMPLETE / CLOSED_BY_OPERATOR`;
`0.39 IN_PROGRESS`. **Stan bezpieczeństwa:** `production_approved=false`,
`stage_039_complete=false`. 
