# OCAF 0.39 — udokumentowane rzeczywiste testy KVM i Aya/eBPF (2026-10-15)

## Pierwotne źródła i integralność

Repozytorium: `mojealterego/ocaf-runtime`, wyłącznie `main`.
**Rzeczywisty, uprzywilejowany Linux x86_64 na GitHub-hosted
Ubuntu 24.04**, nie symulacja. Testy zostały wykonane w jednym
workflow: [GitHub Actions 38704086975](https://github.com/mojealterego/ocaf-runtime/actions/runs/38704086975),
SHA `b195ae1525eeb81781dd4c552cff66b645d6cea2`.
Artefakt GitHub: `stage039-actual-github-root-kvm-evidence`,
ID `11956859080`.

Nie są to dwa trwałe serwery ani produkcyjna instalacja aplikacji.

| Artefakt | SHA-256 danych JSON |
|---|---|
| `actual-root-kvm-ioctl.json` | `b1baa88209c54f38e9ad890b63e26d479db3413b1ef7d88227779ad23de8393c` |
| `actual-root-aya-egress.json` | `4792d7d608fa84c4d79f778075eab51fee10284be9121bef268cac993cd71a63` |

## KVM na prawdziwym hoście — PASS

- Kernel: `6.17.0-1022-azure`, architektura `x86_64`, PID1 `systemd`.
- Linux `/dev/kvm` — prawdziwe urządzenie char 10:232, wymagany dostęp
  R/W po `sudo -n`. Zwykły nieuprzywilejowany runner raportował NO-GO
  z powodu permissions, dlatego wcześniejsze stwierdzenie „brak KVM”
  było nadmiernie kategoryczne.
- `KVM_GET_API_VERSION=12` — zweryfikowane, **PASS**.
- `KVM_CREATE_VM` zwróciło prawdziwy deskryptor jądra VM,
  natychmiast zamknięty; **PASS**.
- Host: `systemd` PID 1, dostępne cgroup v2 CPU/memory/pids,
  bpffs, BTF/CO-RE; **PASS**.

**Nie jest to dowód uruchomienia gościa Firecracker:** w raporcie
`guest_booted=false`, `firecracker_booted=false`.

## Aya/eBPF — rzeczywisty attach i negatywne testy — PASS

- ELF `cgroup_skb/egress` SHA-256:
  `992cd7a365067f52bce5afe5ca9758dad0e15ad9291b71991914040faaa55c95`.
- Faktyczny host Linux kernel BPF program ID: **93**.
- Faktyczny inode dedykowanego cgroup tenant A: **6764**.
- Host boot ID: `112e7592-89fe-46b9-9f3a-b8410263a342`.
- SHA-256 natywnego loadera Aya:
  `1f387e008c3195f7f9f1784c13004fab34f444f5700e67f3197afb1fa9309793`.
- **PASS:** przed attach tenant A i B łączyły się z `127.0.0.1`;
  po `CgroupSkb::attach` tenant A stracił połączenie, B zachował
  dostęp; po `detach` tenant A odzyskał połączenie.
- Ładunek nie wychodził do internetu, a cgroup były oddzielne
  i tymczasowe. Czas testowego attach: **15 sekund**.
- Pozytywny JSON status: `KERNEL_EBPF_EGRESS_ISOLATION_PASSED`.

Przed testem loader został skopiowany do dedykowanego katalogu
root-owned 0700 (binarium root-owned 0500), co naprawiło pierwszą
próbę odrzuconą przez politykę właściciela pliku. Pokazuje to,
że bramka naprawdę odrzuca niezgodne dane wejściowe.

## Granice odbioru, które pozostają

- W chwili wystawienia dowodów **nie wykonano jeszcze pozytywnego
  dowodu** `jailer` + `Firecracker API Running` + kernel console boot.
  Osobny [workflow](../.github/workflows/stage039-real-firecracker-jailer.yml)
  próbuje to zrobić, bez automatycznego GO.
- Aya A/B potwierdza izolację **próbnych lokalnych cgroup**, nie
  sieciową izolację produkcyjnych agentów w działających VM, nie
  zabezpieczenie przed escape.
- Runner jest **nietrwały/ephemeral**; nie spełnia wymogu
  długotrwałego hosta staging/produkcji.
- `stage_039_complete=false`, `production_approved=false`.
  Operator administracyjnie odblokował release, ale samo to nie
  zmienia technicznego zakresu potwierdzonych testów.

Ten dokument zastępuje stare „KVM/Aya NO-GO na wszystkich dostępnych
runnerach” dla tej konkretnej konfiguracji `sudo -n`. Nie zastępuje
bramki bezpieczeństwa ani osobnej akceptacji działania mikroVM.
