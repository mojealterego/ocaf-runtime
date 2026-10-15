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
