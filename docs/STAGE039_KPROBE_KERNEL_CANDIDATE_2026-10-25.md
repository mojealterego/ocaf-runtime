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

Następnym technicznym krokiem jest osobny, powtarzalny build jądra z
przypiętego źródła i konfiguracji Firecracker, uzupełnionej o
`CONFIG_KPROBES=y` i `CONFIG_KPROBE_EVENTS=y` przy zachowaniu BTF,
`CONFIG_BPF_EVENTS=y`, `CONFIG_PERF_EVENTS=y` i `CONFIG_BPF_SYSCALL=y`.
Źródło, łatki, konfigurację wynikową i binarny `vmlinux` trzeba przypiąć
SHA-256; wynik `make olddefconfig` należy sprawdzić, bo może odrzucić
żądane opcje. Następnie obraz musi uruchomić istniejący guest Aya/WASI
na realnym jailed Firecracker. Pełna bramka 0.39.22 wymaga **udanego
załadowania i attach programu kprobe wewnątrz gościa** z dodatnim kernel
program ID i zdarzeniem `openat`; sama obecność pliku `kprobe_events`
lub flaga `.config` nie jest wystarczającym dowodem. Do takiego testu
status 0.39.22 pozostaje `PARTIAL`.
