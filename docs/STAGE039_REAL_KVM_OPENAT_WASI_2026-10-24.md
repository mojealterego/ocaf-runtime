# Etap 0.39 — rzeczywisty KVM, WASI i sonda `openat`

## Zweryfikowany przebieg

- Commit: `706d5f1b2631c734e3425ec3819726a0a2cb00bc`.
- [GitHub Actions run 40051055265](https://github.com/mojealterego/ocaf-runtime/actions/runs/40051055265): `success` na hoście z `/dev/kvm`.
- Artefakt `stage039-real-firecracker-jailer-linux-evidence`, ID `12544005275`; SHA-256 archiwum: `cb7f0664a58d379c7d259a2aee85b855ec3dfa8d88312a72124b32eb7241c883`.

| Dowód w artefakcie | Zaobserwowany wynik |
| --- | --- |
| `kvm.json` | Kernel utworzył prawdziwy VM fd przez `KVM_CREATE_VM`. |
| `broker-wasi-vm.json` | Dwa równoczesne jailed Firecracker VM związane z rezerwacjami SQLite i tożsamością `SO_PEERCRED`; lease zwolnione po kontrolowanym SIGKILL. |
| `real-guest-wasi-execution.json` | Różne moduły i 128-bitowe nonce tenantów A/B; oba moduły sprawdzone w gościu przez SHA-256, wykonane przez Wasmtime WASIp1 `fd_write`, kod wyjścia 0. |
| `real-guest-wasi-execution.json` | W obu *oddzielnych jądrach gości* program BPF tracepoint miał kernel ID `3` i zliczył po `7` rzeczywistych `openat` procesu Wasmtime. ID i TGID są liczbami lokalnymi dla każdego jądra, więc mogą się powtarzać między VM. |
| `jailed-guest-network.json` | Host TC/eBPF na TAP: odcięcie ruchu A, działanie B, przywrócenie po odłączeniu polityki. |
| `pre-admission-deny.json` | Odmowa przed startem VM po zmianie podpisanego payloadu, skrótu modułu, klucza zaufania lub bajtu obrazu rootfs. |

Poprzednie próby [40050113967](https://github.com/mojealterego/ocaf-runtime/actions/runs/40050113967)
i [40050480432](https://github.com/mojealterego/ocaf-runtime/actions/runs/40050480432)
zakończyły się odmową: stary obraz Linux 4.14 nie miał tracepointu.
Workflow przypina oficjalny obraz debug Linux 6.1.188 i jego konfigurację
osobnymi SHA-256; szczegóły źródła są w
[`OFFICIAL_KERNEL_CANDIDATE.md`](../stage039/guest-bpf/OFFICIAL_KERNEL_CANDIDATE.md).

Następny [run 40089951779](https://github.com/mojealterego/ocaf-runtime/actions/runs/40089951779)
na commicie `990bcc4b9cd15c6defb11a99588f948fdc262df2` także zakończył się
sukcesem. Artefakt ID `12559075837` ma SHA-256
`9cce4fa926fab06fecffd50464621c0c1e2ef02a16ea13e06dbf04055eb23492`.
Podpisany manifest przypina teraz *osobno* kernel, obraz rootfs, Wasmtime,
sondę guest eBPF i moduł WASI. W `pre-admission-deny.json` jest siedem
udanych testów odmowy, w tym zmiana pojedynczego bajtu każdej z ostatnich
czterech pozycji. Dwa goście ponownie zgłosiły po 7 zdarzeń `openat`.

[Run 40090227467](https://github.com/mojealterego/ocaf-runtime/actions/runs/40090227467)
na commicie `9e1518e90063599ac16e8c5da36d5ba6f78d6594` także przeszedł.
Artefakt ID `12558024315` ma SHA-256
`3d5048de3200335cb40cfb671efef89e37e5e8de97731b129582c0d8b300a9c6`.
Broker wyprowadził dwa różne W3C `traceparent` z uwierzytelnionych rezerwacji,
przekazał je przez parametry boot Firecracker, a PID1 odczytał je z `/proc/cmdline`.
Raporty obu gości i broker potwierdziły zgodność; każdy guest ponownie zgłosił
7 zdarzeń `openat`. Kolektor OTLP nie uczestniczył w tym przebiegu.

## Stan odbioru

To dowód laboratoryjny na jednym hoście KVM. Raporty zachowują
`stage_039_complete=false` i `production_approved=false`.
Sonda guest używa bezpośrednich syscalli BPF, a wymaganie 0.39.27 wymienia
Aya. Klucz podpisujący manifesty powstaje w tym samym przebiegu CI.
Zintegrowane egress broker, autoryzowana kwarantanna, wspólny test awarii,
niezależny klucz podpisujący, dwa trwałe niezależne hosty KVM oraz odbiór
operatora nadal wymagają osobnych dowodów. Osobne hashe artefaktów i ich
negatywne testy są dowodem laboratoryjnym, nie poświadczeniem dostawcy.
Również korelacja śladów w zewnętrznym kolektorze pozostaje otwarta.
