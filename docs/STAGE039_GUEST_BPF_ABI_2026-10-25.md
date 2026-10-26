# 0.39.22 — ABI BPF w jądrze gościa

`stage039/guest-wasi/guest_wasi_init.c` odczytuje z **gościa** nagłówek
`/sys/kernel/btf/vmlinux`, obecność `kprobe_events` w guest tracefs oraz
wynik rzeczywistego `bpf(BPF_PROG_LOAD)` z dodatnim ID programu. Emituje
jedną linię `OCAF039_GUEST_BPF_ABI_BTF=<0|1> KPROBE=<0|1> SYSCALL=1`.
`scripts/stage039_guest_bpf_abi.py` sprawdza tę linię w serialu dwóch
oddzielnych jailed Firecracker VM, wiąże ją z hashem serialu i pozytywnymi
zdarzeniami Aya `openat` w raportach brokerowanych zadań WASI. Osobno
porównuje SHA-256 pliku `.config` przypiętego jądra.

Obecny oficjalny debug kernel Firecracker `vmlinux-6.1.188` ma
`CONFIG_DEBUG_INFO_BTF=y` i `CONFIG_BPF_SYSCALL=y`, ale **nie ma**
`CONFIG_KPROBES=y` ani `CONFIG_KPROBE_EVENTS=y`. Zostało to sprawdzone na
pliku `.config` o SHA-256
`05238c0baa2e1266abadaf5c2d90f70a04f2f2a644381f5c03452e9b98a624a4`.
W [run 40202649826](https://github.com/mojealterego/ocaf-runtime/actions/runs/40202649826)
na commicie `553ce4c` wymagany krok dwóch brokerowanych, jailed gości KVM
zakończył się sukcesem. W obu serialach gościa odczytano `BTF=1`,
`KPROBE=0`, `SYSCALL=1`; zweryfikowano ich SHA-256, dodatnie ID programu
Aya i zdarzenia `openat` wykonywanego przez Wasmtime. Archiwum dowodów
ID `12601919898` ma SHA-256
`3f66db154e26bd3818a8c18b8884bb55a0c5d2d72b3b07e13562ab3c91325a5c`.
Raport ma status `GUEST_BPF_ABI_PARTIAL_KPROBE_OR_BTF_UNAVAILABLE`, nie
potwierdza gotowości kprobe i zachowuje `stage_039_22_complete=false`.

Późniejszy [test niestandardowego jądra](STAGE039_KPROBE_KERNEL_CANDIDATE_2026-10-25.md)
przeszedł najpierw w jednej VM w
[run 40260131539](https://github.com/mojealterego/ocaf-runtime/actions/runs/40260131539),
a następnie w dwóch brokerowanych jailed VM z odrębnymi zadaniami WASI,
guest Aya `openat` i host Aya w
[run 40262951835](https://github.com/mojealterego/ocaf-runtime/actions/runs/40262951835).
Każdy guest załadował i przyłączył program kprobe z dodatnim ID oraz
licznikiem `getpid`. Ten wynik nie zmienia odczytu **oficjalnego** obrazu
Firecracker z dwóch VM powyżej: nadal nie ma on KPROBES. Trwałe
przypięcie dystrybuowanego jądra, niezależna atestacja jego pochodzenia i
test wielohostowy pozostają otwarte; 0.39 jako całość jest nadal otwarty.
