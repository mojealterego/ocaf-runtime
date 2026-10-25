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

[Run 40156485144](https://github.com/mojealterego/ocaf-runtime/actions/runs/40156485144)
na commicie `b5a97cbd192e954540fd9ae298619d4d8bd03341` zakończył się
sukcesem na hoście KVM. W obu przebiegach brokera hostowy Aya cgroup egress
został załadowany **przed** startem Firecracker, blokował próbę połączenia
procesu w cgroup A, przepuszczał oddzielną cgroup B, pozostał przypięty
podczas wykonania WASI w gościu i odłączył się po zakończeniu VM. Osobny
test uruchomił dwa jailed VM; sonda Aya w jądrze A obserwowała `openat`
Wasmtime, po czym gość otrzymał krótkotrwały podpisany permit przez vsock.
Pomocnik wstrzymał wskazany PID przez `PTRACE_INTERRUPT`, potwierdził
tożsamość procesu i zwolnił go przez `PTRACE_DETACH`, gdy B nadal działał.
Workflow wymagał przejścia wszystkich tych bramek oraz zapisu raportów.
Artefakt `stage039-real-firecracker-jailer-linux-evidence` ma ID `12585190719`
i SHA-256 archiwum `1e344f27bb40c358513bef74fada48fe4f0b0aee92c02dd8bd6c539e0f8c1c5e`.
Serwer blob odmawia pobrania archiwum w tym środowisku; dane liczbowe należy
odczytać bezpośrednio z artefaktu. Permit podpisano kluczem utworzonym w tym
samym teście laboratoryjnym. To nie dowodzi niezależnego operatora ani
odporności na wspólną awarię brokera i sond.

[Run 40179802694](https://github.com/mojealterego/ocaf-runtime/actions/runs/40179802694)
na commicie `10c4309c84e1281e7066557a88290ba5938f41c5` przeszedł pełny
workflow KVM, w tym dodatkowy test negatywny. W osobnym podpisanym obrazie
usunięto guest Aya; hostowy Aya nadal był aktywny, a drugi jailed VM działał.
Bramka wymagała dokładnej odmowy `OCAF039_Q_GUEST_DENIED=aya_ready`, braku
wydania permit, działania VM B podczas odmowy oraz odłączenia hostowego Aya
i przywrócenia łączności sondy hosta po zakończeniu VM. Artefakt ma ID
`12593702126` i SHA-256 archiwum
`1dbf91a2813475634fb30d5a161479781cef5461d88cdc27dcb7d3522be3a092`.
Poprzedni [run 40156821714](https://github.com/mojealterego/ocaf-runtime/actions/runs/40156821714)
zatrzymał się na błędnym parserze zakończonej nową linią ścieżki cgroup;
poprawka parsera ma lokalny test i przeszła w tym biegu. Test sprawdził
tylko przypadek brakującej sondy gościa; awaria brokera i partycja sieci
pozostają otwarte.

[Run 40182033376](https://github.com/mojealterego/ocaf-runtime/actions/runs/40182033376)
na commicie `860a9339f5dc5778f0b3a6347a6b57ef4f443f7a` przeszedł pełny
workflow KVM. Po pierwszej autoryzowanej odpowiedzi TLS dla gościa broker
egress zamknął i ponownie otworzył prywatny rejestr SQLite WAL oraz utworzył
nową instancję. Ponowne przedstawienie tego samego podpisanego permitu
przez vsock zostało odrzucone bez ponownego kontaktu z serwerem TLS.
Bramka CI wymaga `durable_replay_denied_after_broker_restart=true` i
oddzielnych decyzji `GRANTED`, `DENIED`. Lokalny test sprawdził też odmowę
po otwarciu rejestru w nowym procesie. Artefakt ma ID `12594508555` oraz
SHA-256 archiwum `cebbd88e6af5cc31e111ee3eb9dff8aecffbd6da063c494854bcdc1c816437bd`.
Ten test dotyczy odtworzenia instancji i trwałości replay; awaria całego
procesu brokera podczas pracy VM oraz partycja sieci pozostają otwarte.
Osobny lokalny test zabija `SIGKILL` proces brokera po zatwierdzonej
transakcji SQLite i potwierdza odmowę replay po ponownym uruchomieniu
procesu. Nie zastępuje to wspólnego testu awarii z działającym VM.

[Run 40200198836](https://github.com/mojealterego/ocaf-runtime/actions/runs/40200198836)
na commicie `af103ae3129fcf21bf064fd52b410e92827cdc28` przeszedł pełny
workflow KVM z osobnym procesem brokera egress. Po pierwszej odpowiedzi TLS
proces został zabity sygnałem `SIGKILL`, gdy jailed guest nadal działał.
Drugie żądanie gościa otrzymało odmowę podczas niedostępności brokera;
nowy proces odczytał trwały rejestr SQLite i odrzucił trzecie żądanie jako
replay. Bramka sprawdziła serial gościa, piętno kernelowe VMM, brak grant
podczas awarii oraz treść obu odmów. Artefakt ID `12601347265` ma SHA-256
archiwum `1ec1d9cba8db6564893e6d1207ecb6c9f3f678dce9cd369ac0f85d2aa4e82d7e`.

[Run 40200342138](https://github.com/mojealterego/ocaf-runtime/actions/runs/40200342138)
na commicie `9d048cfa0ebee696c990498d08241b8fab3003b1` przeszedł na dwóch
osobnych hostach CI. Na każdym wykonano 100 rzeczywistych cykli boot
jailed Firecracker/KVM, dowodu rootfs FD oraz SIGKILL z opróżnieniem cgroup;
porównanie odrzuca brak dowolnej serii. Artefakty: A ID `12601193534`,
SHA-256 `d3cbe501ce19197378497706a39dee3d345d479577d4751112c68996b887a0a2`;
B ID `12601593056`, SHA-256
`f0274bd94b87ccc8dd102d02fa04875fc3b3a340991a68513141e68cf905b7c9`;
porównanie ID `12601538179`, SHA-256
`98fa479bda2b7784b7a8f466cee09f2b542478a7ede84de45bd7b92def767189`.
Oba hosty CI były efemeryczne, a test nie dowodzi trwałego środowiska,
komunikacji między hostami ani partycji między nimi.

Właściciel 2026-10-25 zrezygnował z niezależnego audytu etapu 0.39.
Weryfikator dossier wymaga teraz jego własnego podpisu Ed25519 nad schematem
v2; [instrukcja odbioru](STAGE039_OWNER_SIGNOFF.md) opisuje dokładne pliki.
Podpis i trwałe hosty nie zostały jeszcze dostarczone.

[Run 40182499438](https://github.com/mojealterego/ocaf-runtime/actions/runs/40182499438)
na commicie `ada9056afb6dab8e8352fdfd1f6e2171ad5ca308` przeszedł pełny
workflow KVM z nową kontrolą inode rootfs. Po potwierdzeniu stanu `Running`
runner ponownie obliczał SHA-256 obrazu w chroot, sprawdzał jego tryb i
liczbę dowiązań, a następnie wymagał deskryptora otwartego przez żywy proces
Firecracker do tego samego `st_dev` i `st_ino`. Broker wiązał ten raport z
podpisanym digestem obrazu każdego z dwóch tenantów. Artefakt ID
`12594637328` ma SHA-256 archiwum
`7b0773d068efff08392fd89d3235490adc840d1e28b4b02f179c3a3c0b5447f7`.

[Run 40182621380](https://github.com/mojealterego/ocaf-runtime/actions/runs/40182621380)
na commicie `6c93143d6886603a0236d19f2fa3424dea5aa4c6` dodatkowo porównał
raporty dwóch osobnych efemerycznych runnerów KVM. Dla obu wymagano
odmiennego `boot_id`, działającego jailed VM, potwierdzenia otwartego inode
rootfs oraz opróżnienia cgroup po SIGKILL. Artefakty: tenant A ID
`12594931606` (SHA-256 `e53453c6396dbf538dbb83c7b1f261d4c055b4ceca6cce0d2c5a50865052e984`),
tenant B ID `12594936551` (SHA-256
`29142a6f912920c9e6b13e2d090e8dacf17c97c46d48a6a1528308462857a6b1`),
porównanie ID `12595001459` (SHA-256
`dc2d761778313e1eff9203093bb9a58eac344c518b07c161ecd8d901ab9a29aa`).
To są dwa niezależne jądra CI, ale nie trwałe hosty produkcyjne; nie było
połączenia między nimi, testu partycji ani 100 cykli na każdym.

## Stan odbioru

To dowody laboratoryjne na jednym i na dwóch efemerycznych hostach KVM. Raporty zachowują
`stage_039_complete=false` i `production_approved=false`.
Wymaganie 0.39.27 ma udany test laboratoryjny Aya w gościu. Klucz
podpisujący manifesty nadal powstaje w tym samym przebiegu CI.
Pozostałe przypadki wspólnej awarii, osobny klucz właściciela, dwa trwałe
hosty KVM oraz jego podpisany odbiór nadal wymagają osobnych dowodów.
Niezależny audyt został przez właściciela wyłączony z bramki 0.39.
Osobne hashe artefaktów i ich
negatywne testy są dowodem laboratoryjnym, nie poświadczeniem dostawcy.
Korelacja śladów w niezależnym kolektorze pozostaje otwarta.
