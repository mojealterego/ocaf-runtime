# OCAF 0.39 — Firecracker/KVM + Aya/eBPF: rozpoczęcie implementacji

**Status: STARTED / PRE-ADMISSION ONLY / NO-GO.** Branch: `main`.
Rozpoczęcie developmentu 0.39 nastąpiło na wyraźną dyspozycję operatora.
**Nie oznacza ono zakończenia 0.38 ani autoryzacji produkcyjnej.**
Brakujące A01–A12 pozostają zarejestrowane w
[`STAGE038_ACCEPTANCE_BLOCKERS_2026-10-14.md`](STAGE038_ACCEPTANCE_BLOCKERS_2026-10-14.md).

## Docelowa granica wykonawcza

`OCAF TLS/RPC + signed tenant policy -> isolated privileged broker ->
Firecracker jailer -> KVM microVM -> approved immutable guest rootfs ->
bounded WASI execution -> receipt and teardown`.

Aya/eBPF musi egzekwować jawne polityki hostowego cgroup/network/device,
a dowód musi zawierać rzeczywisty attach/link ID, boot ID, jailer PID,
powiązanie z konkretną instancją VM oraz hash kodu programu BPF.
Sam plik `.o` nie dowodzi, że eBPF chroni jakikolwiek workload.

## Zrealizowane w 0.39.1

1. `scripts/stage039_microvm_gate.py` — read-only preflight Linux x86_64,
   `systemd` PID1, cgroup v2 cpu/memory/pids, `/dev/kvm` char 10:232 i
   uprawnienia R/W, BTF oraz bpffs. Obserwacje są faktyczne dla badanego
   hosta, **bez** uruchamiania KVM/guest.
2. Statyczny manifest default-deny z dokładną listą pól, typowaniem,
   ograniczonymi zasobami (1–4 vCPU, 128–2048 MiB), `jailer=true`,
   `network=none`, `guest_egress=false`, `rootfs_readonly=true`,
   `mmds_enabled=false`, `seccomp=default`.
3. SHA-256 pinuje cztery artefakty: binarium `firecracker`, kernel,
   rootfs i program Aya. Odrzucamy obcy katalog, symlink, hardlink,
   pliki group/world-writable, błędny digest, nieznane i zdublowane
   klucze JSON, NaN, boolean w miejscu integera oraz nadmierne rozmiary.
4. Raport tworzymy jednokrotnie z mode 0600; nie publikujemy sekretów
   ani ścieżek do prywatnych artefaktów. `microvm_launched=false`,
   `ebpf_attached=false`, `stage_039_complete=false`,
   `production_approved=false`.
5. Dwanaście testów regresyjnych oraz osobny workflow
   `.github/workflows/stage039-microvm-preflight.yml` uruchamiany na
   `main`; prawdziwy stan runnera Linux zapisany w artefakcie
   `stage039-linux-kvm-ebpf-readonly-preflight`.

## Reprodukcja

```bash
python3 -m unittest discover -s scripts/tests -p test_stage039_microvm_gate.py -v
python3 scripts/stage039_microvm_gate.py --host-only --report /secure/stage039-host.json
python3 scripts/stage039_microvm_gate.py --manifest /secure/stage039-request.json \
  --approved-root /opt/ocaf/stage039/approved \
  --report /secure/stage039-static.json
```

`--manifest` wymaga `schema_version=1`, `workload_id`, `tenant_id`,
`agent_id`, `isolation` oraz `assets`. Każdy zasób ma `path` i
`sha256` zatwierdzone przez operatora. Sam poprawny digest **nie chroni
przyszłego uruchomienia przed TOCTOU** — runtime musi trwale powiązać
zweryfikowany inode z uruchamianą VM.

## Otwarte kryteria 0.39.2–0.39.6

- Broker z rzeczywistym odseparowanym API, powiązaniem
  `tenant:agent:workload`, kontrolą konkurencji i leader-fencing.
- Izolowane uruchomienie Firecracker przez jailer i zweryfikowany KVM,
  secure lifecycle, izolacja dysku/urządzeń, teardown po kill/crash.
- Aya/eBPF: kompilacja i rzeczywiste attach/egzekwowanie cgroup/network
  policy oraz negatywne próby obejścia.
- Udokumentowana instancja VM, attestation kernel/rootfs/Firecracker,
  bootstrap secret boundary, link-ID cgroup oraz pomiary PID/RSS/I/O.
- Integracja RPC→broker→WASI→receipt, negatywne testy ucieczki,
  rotacja/revocation, odtwarzanie i niezależny test KVM na hostach.
- Produkcyjny GO wyłącznie po rzeczywistych testach i audycie,
  osobno od otwartego odbioru 0.38.

**Status końcowy tej inkrementacji: 0.39 STARTED, NIEUKOŃCZONY,
0.38 PARTIAL / NO-GO, produkcja NO-GO.**

## Realizacja 0.39.2 — lokalna transakcyjna bramka brokera (2026-10-15)

Wprowadzono `scripts/stage039_broker_ledger.py`, importowalny moduł
kontroli przyjmowania zadań z lokalną bazą SQLite WAL (`synchronous=FULL`,
`BEGIN IMMEDIATE`). Ten moduł **nie uruchamia maszyny wirtualnej**,
nie przyjmuje sieciowych poleceń i nie staje się automatycznie usługą.

Przed przyjęciem rezerwacji kontrolowane są:
- ten sam `tenant_id`, `agent_id`, `workload_id` co w ścisłym manifeście,
  SHA-256 całego manifestu, odmowa duplikatów JSON i replay `request_id`;
- przedział ważności żądania (maks. 300 sekund), monotoniczny `leader_epoch`,
  niezmienność właściciela `workload_id` i zakaz zastępowania żywej rezerwacji;
- wymagania rzeczywistego hosta z `microvm.preflight()`, poprawność
  hashy czterech artefaktów w zatwierdzonym prywatnym katalogu;
- pojemność brokera (64 równoczesne rezerwacje), rozmiar pliku
  bazy i ograniczenie rejestru historii; brak domyślnego fallbacku
  z trwałego SQLite na cache pamięciowy;
- chroniony katalog bazy (0700), create-exclusive pliku bazy
  z uprawnieniami 0600 **przed** otwarciem przez SQLite.

Odpowiedź przy pozytywnej rezerwacji brzmi
`RESERVED_NOT_EXECUTED` z jawnie fałszywymi flagami
`microvm_launched`, `ebpf_attached`, `production_approved` i
`stage_039_complete`. Rezerwacje i zwolnienia są zapisywane
transakcyjnie; błędny lub powtórzony `request_id` jest odrzucany również
po zwolnieniu.

**Granica zaufania:** aktualny moduł nie uwierzytelnia klienta. Ostateczny
broker musi oddzielnie ustalać rzeczywistą tożsamość przez kernelowe
`SO_PEERCRED` lokalnego AF_UNIX i wiązać uprawnienia z RPC principal.
Nie wystawiać metod `reserve`/`release` na niezabezpieczonym TCP/HTTP.
Nie używać samej rezerwacji jako poświadczenia, że kernel KVM został
uruchomiony lub program Aya/eBPF był faktycznie podpięty.

Regresje: `scripts/tests/test_stage039_broker_ledger.py`.
Potwierdzono 17 testów, w tym osobny cykl RED/GREEN weryfikujący, że
plik SQLite musi mieć 0600 jeszcze przed `sqlite3.connect`.
Workflow: `.github/workflows/stage039-microvm-preflight.yml`.

## Dyspozycja operatora względem 0.38

Rozdzielenie decyzji administracyjnej i weryfikowalnego odbioru opisuje
[`STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md`](STAGE038_OPERATOR_DIRECTIVE_2026-10-15.md).
Prace 0.39 nie są już wstrzymywane przez kolejność backlogu, lecz bramki
wdrożeniowe i bezpieczeństwa nadal nie mogą zostać oznaczone PASS bez dowodów.

## 0.39.3 — rzeczywiste IPC AF_UNIX z uwierzytelnianiem jądra (2026-10-15)

Zaimplementowano `scripts/stage039_broker_ipc.py`. Jest to wyłącznie
lokalna, kontrolowana bramka **admission**, bez wykonywania mikroVM:

- rzeczywiste gniazdo `AF_UNIX` z odczytem `SO_PEERCRED`
  (kernelowo poświadczone `pid,uid,gid`); odmowa wszystkich UID
  nieobecnych na jawnej liście operatora;
- jeden `tenant:agent` na osobny socket, bez pola UID w JSON i bez
  możliwości nadpisania kernelowego principal danymi sieciowymi;
- katalog socketu `0700`, sam socket `0600`, odmowa podmiany
  istniejącej ścieżki, usuwanie tylko własnego inode;
- długość ramki 32-bit big-endian, ścisły JSON, skończone
  limity wiadomości i czasowy deadline; odmowa unknown fields,
  niespójnych typów, duplikatów JSON i prób `exec`;
- `reserve` → lokalna SQLite z anty-replay / fencing, `release`
  → powiązanie właściciela istniejącej rezerwacji z kernelowym UID;
- `health` i odpowiedzi jawnie deklarują `microvm_launched=false`,
  `ebpf_attached=false` i `production_approved=false`.

Regresje: `scripts/tests/test_stage039_broker_ipc.py` — 10 testów
obejmujących **rzeczywiste** gniazdo UNIX i prawa systemu plików,
uprawnienia kernel UID, politykę tenant/agent, odmowę >128 KiB,
odmowę sfałszowanego UID w JSON i realny zapis/replay/release w SQLite.
Workflow GitHub Actions `stage039-microvm-preflight.yml` uruchamia
łącznie **39 testów** (12 manifest/host, 17 ledger, 10 IPC).

**Ograniczenia:** brak rzeczywistego Firecracker/jailer start,
KVM VM/guest, attach Aya/eBPF, pomiaru isolation i pełnego
podpisanego RPC→host brokera. Peer UID identyfikuje proces lokalny,
a nie podpisanego zdalnego agenta: zaufany front-end RPC i kontrola
autoryzacji nadal pozostają obowiązkowe. Odczyt całych dużych hashów
odbywa się synchronicznie — przed udostępnieniem jako usługa
konieczne są ograniczenia pojemności/konkurencji i testy denial-of-service.


## 0.39.4 — sprawdzenie API KVM na żywym jądrze (2026-10-15)

`scripts/stage039_kvm_ioctl.py` wydaje prawdziwe IOCTL do `/dev/kvm`:
`KVM_GET_API_VERSION` (oczekiwane `12`) oraz `KVM_CREATE_VM`
(zwrócony VM fd jest natychmiast zamykany). Testy syntetycznego mock
i realny Linux probe są odseparowane, a report nigdy nie twierdzi, że
uruchomiono Firecracker lub guest. Osiem regresji przeszło.
**Na aktualnym GitHub-hosted runnerze: NO-GO**; brak używalnego KVM.

## 0.39.5 — konfiguracja Firecracker przygotowana, bez startu

`scripts/stage039_firecracker_spec.py` waliduje manifest, pinuje
SHA-256 czterech artefaktów i **ponownie hashuje** rzeczywiście
wyłożone w zatwierdzonym chroot `/guest/vmlinux.bin` oraz
`/guest/rootfs.ext4`. Tworzy Firecracker JSON:
`boot-source`, `drives` z `is_read_only=true`,
`machine-config` z bounded CPU/RAM, `network-interfaces=[]`.
Sześć testów (także zmiana staged kernel/rootfs, symlink, writable
katalogu, próba dodania sieci) PASS.

**Pozostały blocker:** konfiguracja nie uruchamia procesu `jailer` ani
Firecracker. Dopiero start binariów tej samej wersji, prawdziwy
`InstanceInfo: Running` i dowód identyczności inode uruchomionych
artefaktów może poświadczyć boot. Kopiowanie i sprawdzanie digestów
przed startem nie zamyka samodzielnie problemu TOCTOU.

## 0.39.7 — prawdziwy loader Aya, jeszcze bez kernel attach

- `stage039/aya-ebpf-egress/src/main.rs`: program
  `aya_ebpf::cgroup_skb(egress)`, wynik `0` powoduje drop pakietu
  powiązanego z docelowym cgroup.
- `stage039/aya-guard/src/main.rs`: osobne narzędzie Rust z użyciem
  biblioteki Aya 0.14, statycznym SHA-256 ELF, `O_NOFOLLOW`, restrykcją
  `/sys/fs/cgroup/.../ocaf-*.service`, jawnie potwierdzanym
  `--ack-attach`, ograniczonym czasem życia attachment i odpięciem
  na końcu. Kompilacja i trzy testy logiki loadera PASS.
- **Nie** przeprowadzono jeszcze kompilacji eBPF `bpfel-unknown-none`,
  realnego `BPF_PROG_LOAD`/attach ani negatywnego packet-egress testu.
  Bez tych kroków `ebpf_attached=false` i etap 0.39 nadal NO-GO.

Odczyt dowodów:
[run 38663469207](https://github.com/mojealterego/ocaf-runtime/actions/runs/38663469207)
zawiera **68 zaliczonych testów (65 Python + 3 Rust Aya)**.
Test Linux-host i próba użycia KVM jawnie zwróciły NO-GO;
sukces workflow oznacza poprawne, fail-closed wykrycie braku sprzętu.

Zasady wykonywania na hoście: operator musi przedstawić autoryzowany
systemd/cgroup-v2 Linux KVM, zatwierdzony jailer/Firecracker tej samej
wersji, zakotwiczone digests kernel/rootfs, politykę eBPF cgroup
i logi uruchomienia i ograniczenia egress. Bez tych dowodów brak GO.


## 0.39.6 — realny Firecracker przez jailer: skrypt testu sprzętowego

Zaimplementowano `scripts/stage039_live_firecracker.py`, który jest
**wykonawczym** narzędziem do uruchomienia ograniczonego gościa
Firecracker na *rzeczywistym* autoryzowanym hoście Linux KVM.

Kryteria przedstartowe: root na dedykowanym hoście, systemd PID1,
dostępne `/dev/kvm`, realne `KVM_CREATE_VM`,
zatwierdzony manifest SHA-256 Firecracker/kernel/rootfs,
parowane `jailer`/Firecracker tej samej wersji, prywatny jailer base
(0700), odrębne UID/GID (nie root) i rzeczywisty cgroup v2.

Skrypt przygotowuje staging jailera w rzeczywistej ścieżce
`<jailer-base>/<firecracker-name>/<vm-id>/root`,
kopiuje guest kernel/rootfs z FD `O_NOFOLLOW` oraz ponownie
porównuje digest, generuje domyślnie bezsieciową konfigurację,
uruchamia **jailer bez --daemonize**, potwierdza Firecracker API
`state=Running`, komunikat `Linux version ...` na konsoli gościa
i membership PID w konkretnym cgroup.
Zamyka proces i podejmuje sprzątanie osobnego cgroup.
**Nie** jest to dowód uruchomienia aplikacji w gościu ani gwarancja
braku TOCTOU.

### Uruchomienie na host Linux/KVM (nie na GitHub-hosted runner)

```bash
sudo python3 scripts/stage039_live_firecracker.py \
  --manifest /opt/ocaf/approved/request.json \
  --approved-root /opt/ocaf/approved \
  --firecracker /opt/ocaf/approved/firecracker \
  --firecracker-sha256 PINNED_SHA256 \
  --jailer /opt/ocaf/approved/jailer \
  --jailer-sha256 PINNED_JAILER_SHA256 \
  --chroot-base /srv/ocaf-jailer \
  --uid 1002 --gid 1002 --timeout 60 \
  --report /var/lib/ocaf/evidence/firecracker-real.json \
  --ack-real-kvm-jailer-boot
```

## 0.39.7 — rzeczywisty test negatywny Aya/eBPF

Zaimplementowano `scripts/stage039_live_egress.py`:
tworzy **dwa nowe, dedykowane testowe cgroups A/B**, otwiera
jedynie lokalny serwer TCP `127.0.0.1`; oba kontrolnie działają.
Po realnym `BPF_PROG_LOAD` i `CgroupSkb::attach` Aya program
deny-all w cgroup A odrzuca połączenie A, podczas gdy niezależny
cgroup B nadal może połączyć się. Po `detach` A ponownie łączy
się. Każde z tych przejść musi mieć sukces, inaczej test kończy
się odmową. Nie ma zewnętrznego ruchu ani skanowania sieci.

Każdy proces testujący musi **rzeczywiście zapisać swój PID do
`cgroup.procs`**; błąd przyłączania do cgroup nie jest liczoną
blokadą pakietów. Raport wiąże program ID Aya, inode cgroup,
boot ID hosta i SHA-256 loadera oraz ELF. Skrypt nigdy nie
zatwierdza wdrożenia Firecracker.

```bash
sudo python3 scripts/stage039_live_egress.py \
  --loader /opt/ocaf/bin/ocaf-stage039-aya-guard \
  --loader-sha256 PINNED_LOADER_SHA256 \
  --program /opt/ocaf/approved/ocaf-stage039-deny-egress \
  --program-sha256 PINNED_BPF_ELF_SHA256 \
  --seconds 30 \
  --report /var/lib/ocaf/evidence/aya-egress-real.json \
  --ack-privileged-loopback-test
```

Dla przejścia na jednym rzeczywistym hoście przygotowano
**workflow manualny** `.github/workflows/stage039-live-kvm-aya.yml`,
uruchamiany **tylko** na autoryzowanym self-hosted runnerze
`[self-hosted, linux, x64, ocaf-kvm-ebpf]`. Wymaga
`vars.OCAF039_*` dla zweryfikowanych ścieżek i digestów i
`sudo -n` na sprzętowo kompatybilnym hoście. Nie używa
zwykłych runnerów GitHub i **nie** tworzy płatnych VM.

### Stan rzeczywistej infrastruktury

Sprawdzono aktualnie dostępne zaplecze:
- GitHub-hosted Linux (`38665774854`): `/dev/kvm` niedostępny,
  a host preflight i KVM ioctl **NO_GO**.
- Sprite `ocaf-stage038-linux`: PID1=`tini` i brak
  `/dev/kvm` — to środowisko kontenerowe, nie docelowy host.
- DigitalOcean: zero istniejących Dropletów.
- Kontenery Render/Railway nie są weryfikowanym hostem KVM.

**Wynik aktualny:** 10 testów kontraktu rzeczywistych
negatywnych prób eBPF + 6 testów kontraktu startu Firecracker
PASS w GitHub Actions; razem z wcześniejszymi: **84 testy**
(81 Python + 3 Rust), [run 38665774854](https://github.com/mojealterego/ocaf-runtime/actions/runs/38665774854).
**To nie są live BPF attach ani guest boot!** Brak dostępnego
autoryzowanego hosta KVM/BPF, więc całościowy test sprzętowy
i jego dowody nadal **NO-GO**.

**Warunek awansu:** rejestracja istniejącego (bez dodatkowych kosztów)
lub osobno zatwierdzonego przez operatora realnego hosta KVM/BPF.
Dopiero niezależne artefakty `kvm.json`, `boot.json`,
`egress.json` z jednego `boot_id` mogą potwierdzić zrealizowanie
prób (z zachowaniem dalszych osobnych kryteriów produkcyjnych).
