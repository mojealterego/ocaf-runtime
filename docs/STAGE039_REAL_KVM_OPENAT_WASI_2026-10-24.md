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

[Run 40126628090](https://github.com/mojealterego/ocaf-runtime/actions/runs/40126628090)
na commicie `b8646955b6bc3bdf9a63fae02253c31855510cc7` zakończył się
sukcesem na hoście KVM. Obraz każdego gościa zawierał statyczny loader Aya
0.14.0 z osadzonym ELF Aya eBPF 0.2.1, którego SHA-256 loader sprawdza przed
załadowaniem. CI zbudowało oba z przypiętych lockfile, uruchomiło dwa VM i
wymagało w każdym dodatniego identyfikatora programu w jądrze gościa oraz
dodatniej liczby zdarzeń `openat` z procesu Wasmtime. Negatywne testy
zmienionych bajtów obrazu, loadera, runtime i modułu także przeszły.
Artefakt `stage039-real-firecracker-jailer-linux-evidence` ma ID `12573387836`
i SHA-256 archiwum `f91ef711944098fcd23b570eda94ebd24697a3128ff0ffe6cd6e97056b48bdab`.
Serwer artefaktów zwrócił 403 przy próbie pobrania archiwum do tego
środowiska; szczegółowe liczniki pozostają dostępne w artefakcie CI, a log
potwierdza przejście sprawdzającej je bramki. Dowód obejmuje jedno uruchomienie
na hoście GitHub Actions, bez odbioru produkcyjnego.

[Run 40126806363](https://github.com/mojealterego/ocaf-runtime/actions/runs/40126806363)
przeszedł pięć negatywnych przypadków na przypiętym Wasmtime v49.0.3 oraz
rzeczywistą odmowę socket przez seccomp na hoście. Artefakt ID `12573143234`
ma SHA-256 `f0c86251e5494282f372979bd2b0383914f3b118669889fc838b77ebed0a2be8`.
Ten przebieg dotyczył wyłącznie hosta.

[Run 40156218795](https://github.com/mojealterego/ocaf-runtime/actions/runs/40156218795)
na commicie `3fcb87aae4e47aea0c4ff5c413bfffa18c74368d` przeszedł w całości
na hoście KVM. Każdy z dwóch gości wykonał także obowiązkowe negatywne
przypadki Wasmtime (fuel, timeout, pamięć, sygnał CPU, SIGKILL dziecka) i
odmowę socket przez seccomp; bramka konsoli wymagała dokładnych markerów
oraz kodu wyjścia supervisora. Osobny jailed gość połączył się przez
AF_VSOCK z hostowym brokerem; po podpisanym pozwoleniu otrzymał odpowiedź
z lokalnego serwera TLS, a ponowne użycie pozwolenia zostało odrzucone.
Host sprawdził `SO_PEERCRED`, PID/start procesu VMM i cgroup. Następnie dwa
zweryfikowane raporty VM zostały wysłane przez OTLP/HTTP JSON do kolektora
na pętli zwrotnej hosta. Workflow utworzył również przed startem VM
CycloneDX 1.5 dla podpisanych zasobów i źródeł Aya. Artefakt
`stage039-real-firecracker-jailer-linux-evidence` ma ID `12584526385`
i SHA-256 `d025e852dd5e27f229b423cb8e8cc5761084b1b4197a0a240ebc25bcd3a865c0`.
Serwer blob ponownie zwrócił 403 przy pobraniu artefaktu do tego środowiska;
statusy poszczególnych bramek i archiwum są zapisane w GitHub Actions.
Test egress dotyczył jednego dodatkowego VM i lokalnego serwera TLS.
Kolektor OTLP był na tym samym hoście, a ślady są eksportem raportów po
zakończeniu VM, nie telemetrią wysyłaną przez aplikację w gościu.

## Stan odbioru

To dowód laboratoryjny na jednym hoście KVM. Raporty zachowują
`stage_039_complete=false` i `production_approved=false`.
Wymaganie 0.39.27 ma udany test laboratoryjny Aya w gościu. Klucz
podpisujący manifesty nadal powstaje w tym samym przebiegu CI.
Autoryzowana kwarantanna, wspólny test awarii,
niezależny klucz podpisujący, dwa trwałe niezależne hosty KVM oraz odbiór
operatora nadal wymagają osobnych dowodów. Osobne hashe artefaktów i ich
negatywne testy są dowodem laboratoryjnym, nie poświadczeniem dostawcy.
Korelacja śladów w niezależnym kolektorze pozostaje otwarta.
