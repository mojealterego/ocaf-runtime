# 0.39.22 — wybór jądra gościa z kprobe

Sprawdzono sześć konfiguracji x86_64 z publicznego, przypiętego snapshotu
Firecracker CI `firecracker-ci/20261023-dd7c24b0d165-0` w bucket
`spec.ccfc.min`. W każdym przypadku porównano bajty pliku `.config`,
obliczono SHA-256 i sprawdzono `CONFIG_DEBUG_INFO_BTF`, `CONFIG_KPROBES`,
`CONFIG_KPROBE_EVENTS`, `CONFIG_BPF_EVENTS`, `CONFIG_PERF_EVENTS` oraz
`CONFIG_BPF_SYSCALL`.

| Obraz `.config` pod `x86_64/` | SHA-256 | BTF | KPROBES | KPROBE_EVENTS | BPF_EVENTS |
| --- | --- | --- | --- | --- | --- |
| `debug/vmlinux-5.10.271.config` | `6eb28292dc3759193d8d372244c7f9b3a17412010d9eff639a46e4a16e8963a4` | tak | nie | nie | tak |
| `debug/vmlinux-6.1.188.config` | `05238c0baa2e1266abadaf5c2d90f70a04f2f2a644381f5c03452e9b98a624a4` | tak | nie | nie | tak |
| `debug/vmlinux-6.18.54.config` | `dfd5753b3275af3ec6d50b63751221405e26021ad9ae4cd14c6a2f0e70d7c667` | tak | nie | nie | tak |
| `vmlinux-5.10.271.config` | `03a2a3fd5de1daaa0f5083cd0980de98cc86b8f4596d6f1e3db98837e267a9bf` | nie | nie | nie | nie |
| `vmlinux-6.1.188.config` | `32f1d98d81e63d86b7f90c42ab7495ebd234f0999a24682843c2f9e46f6f2704` | nie | nie | nie | nie |
| `vmlinux-6.18.54.config` | `7dbdc7445d01d3dd049b1e653534bde71266d59afaf9c1de50a8a6b8a8c138d0` | nie | nie | nie | nie |

Wszystkie sześć ma `CONFIG_PERF_EVENTS=y` i `CONFIG_BPF_SYSCALL=y`.
Nie ma w tym snapshotcie gotowego obrazu spełniającego bramkę kprobe.
Samo przejście na nowsze `debug/vmlinux-6.18.54` nie rozwiąże problemu.
Rzeczywisty przebieg KVM
[40202649826](https://github.com/mojealterego/ocaf-runtime/actions/runs/40202649826)
potwierdza w dwóch gościach BTF i BPF syscall, lecz brak kprobe na użytym
jądrze 6.1.188.

## Wynik ręcznego testu KVM z 2026-10-26

[Run 40260131539](https://github.com/mojealterego/ocaf-runtime/actions/runs/40260131539)
na commicie `071869835156da575813699dd8096192d1167daf` zakończył się
**SUCCESS**. Wszystkie kroki, w tym rzeczywisty KVM, budowa jądra oraz
uruchomienie guest przez Firecracker jailer, zakończyły się sukcesem.
Archiwum `stage039-experimental-guest-kprobe-evidence` ma ID
`12624219410`, rozmiar `239599477` bajtów i raportowany przez GitHub
SHA-256 archiwum
`6e4cd75468f3aa4c91b915a556077b413bb2729b3e8d1275ffe2ed5705e2a442`.
Archiwum zawiera wynikową `.config`, ELF `vmlinux`, ich SHA-256, raport
guest i skrót wyniku. Bieżące środowisko dostaje HTTP 403 z magazynu
artefaktów GitHub, więc zawartości archiwum i dokładnej wartości
`program_id` nie odczytano tu niezależnie; potwierdzone są metadane API
oraz sukces bramek kodu workflow.

Workflow buduje upstream Linux `6.1.188` ze źródła o SHA-256
`ed4d0acb1307c235230c89efc094e210e6290593f94a7e617f28b1001101a33a`
na wejściowej konfiguracji debug Firecracker o SHA z tabeli. Wymaga po
`make olddefconfig` wartości `CONFIG_MODULES=y`, `CONFIG_KPROBES=y`,
`CONFIG_KPROBE_EVENTS=y`, `CONFIG_DEBUG_INFO_BTF=y`, `CONFIG_BPF_EVENTS=y`,
`CONFIG_PERF_EVENTS=y` i `CONFIG_BPF_SYSCALL=y`. Pierwszy przebieg
[40230951701](https://github.com/mojealterego/ocaf-runtime/actions/runs/40230951701)
wykazał, że bez `MODULES` upstream odrzuca `KPROBES`; po poprawce jądro
zbudowano i uruchomiono. Guest PID1 sprawdza BTF, tworzy zdarzenie kprobe
`__x64_sys_getpid`, ładuje program BPF typu KPROBE, odczytuje dodatnie
kernel program ID, przyłącza program przez `PERF_EVENT_IOC_SET_BPF`,
wykonuje `getpid` i wymaga dodatniej liczby zdarzeń. Host wymaga dokładnie
jednego guest markera z dodatnimi ID i licznikiem, aktywnego API VM,
zgodności hashu otwartego rootfs FD, odmowy odczytu hostowego canary oraz
pustego cgroup po SIGKILL. Pojedyncza flaga `.config` nie może przejść tej
bramki.

To **jedna laboratoryjna VM z małym programem BPF w C**, z sondą `getpid`.
Nie jest to jeszcze połączony test dwóch tenantów, Aya `openat`, WASI i
tej samej nowej konfiguracji jądra; brak też trwałego przypiętego artefaktu
jądra i niezależnej atestacji jego pochodzenia. Dlatego status 0.39.22
pozostaje `PARTIAL_REAL_KVM_KPROBE_LAB_PASS`, a etap 0.39 jako całość
pozostaje otwarty. Workflow jest ręczny (`workflow_dispatch`), aby build
jądra nie obciążał każdego push.
