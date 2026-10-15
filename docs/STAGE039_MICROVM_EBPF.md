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
