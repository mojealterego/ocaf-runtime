# OCAF 0.39 — rzeczywisty boot jailed Firecracker/KVM (2026-10-15)

## Identyfikacja dowodu

- Repozytorium i gałąź: `mojealterego/ocaf-runtime`, `main`.
- Commit testowanego źródła: `677bd3e7eea055af9fe6a4b6259c706b5eddcb17`.
- [GitHub Actions: rzeczywisty jailed Firecracker guest boot](https://github.com/mojealterego/ocaf-runtime/actions/runs/38710048957), job `live-jailer-kvm-guest`, wynik `success`.
- Artefakt `stage039-real-firecracker-jailer-linux-evidence`, ID `11959858388`, [odnośnik do artefaktu](https://github.com/mojealterego/ocaf-runtime/actions/runs/38710048957/artifacts/11959858388). SHA-256 archiwum ZIP widoczny w logu uploadu: `2e3e7ba56ca642036db71f9b3e4f591f411462e21eff7b3596c4a7a33700b267`.
- Środowisko: efemeryczny, uprzywilejowany Linux x86_64 w GitHub Actions; **nie** docelowy serwer staging ani produkcja.

## Wyniki obserwowane w logach

| Próg odbioru próby | Obserwowany wynik | Zakres |
| --- | --- | --- |
| Kernel `KVM_CREATE_VM` | `kvm_vm_fd_created=true`; `STAGE039_REAL_KVM_KERNEL_CREATE_VM_PROVED` | Rzeczywiste ioctl jądra na runnerze |
| Binarna zgodność artefaktów | Firecracker 1.17.0 i obrazy `hello-vmlinux.bin`, `hello-rootfs.ext4` przeszły `sha256sum --check` | Sprawdzone SHA-256 wejściowych archiwów/obrazów |
| Uruchomienie jailera z realnym KVM | `JAILED_FIRECRACKER_GUEST_KERNEL_BOOT_OBSERVED` | Weryfikacja `Firecracker API Running`, początkowego komunikatu `Linux version`, członkostwa procesu w dedykowanym cgroup |
| Asercje workflow | `STAGE039_REAL_FIRECRACKER_JAILER_GUEST_LINUX_BOOT_PROVED` | `firecracker_api_running`, `guest_kernel_console_seen`, `process_in_expected_cgroup` są `true` |
| Zapis materiału dowodowego | Upload artefaktu ID `11959858388`, krok `success` | Raporty `kvm.json`, `guest-boot.json` zachowane w artefakcie CI |

### Poprawka weryfikatora i regresja

Na wcześniejszym `main` test `test_firecracker_api_response_uses_content_length_without_waiting_for_eof` wykazał, że parser odpowiedzi na `GET /` czekał na zamknięcie socketu mimo odebrania kompletnej ramki HTTP. Commit `677bd3e7eea055af9fe6a4b6259c706b5eddcb17` zmienił `scripts/stage039_live_firecracker.py::_api_state` na parsowanie nagłówka z limitem 8192 B i jedynego `Content-Length` z limitem 24576 B. Błędy formatu, niepełne body i zbyt duża odpowiedź kończą się odmową (`{}`). Workflow kontraktów `OCAF 0.39 Firecracker-KVM-Aya admission CI`: [PASS](https://github.com/mojealterego/ocaf-runtime/actions/runs/38710048907).

## Niezależne potwierdzenia i ograniczenia

[Rzeczywista próba Aya/eBPF z negatywnym testem A/B](STAGE039_REAL_KVM_AYA_PROOF_2026-10-15.md) była przeprowadzona w odrębnym workflow (38704086975). Oba zestawy dowodów **nie są** jeszcze jednym, równoczesnym testem izolacji egress uruchomionej mikroVM i nie potwierdzają pełnej izolacji dwóch agentów w produkcji.

Nadal wymagane są wspólny end-to-end Firecracker + Aya attach w faktycznym cgroup VM, co najmniej dwa niezależne trwałe hosty, awarie/partycje, długie soak/leak testy, rotacja/rewokacja certyfikatów, niezależny audyt i formalna akceptacja zmian. Powyższy PASS to weryfikowalna próba **guest kernel boot**, nie produkcyjny GO.

**Stan:** `stage_039_complete=false`, `production_approved=false`, `ebpf_attached=false` w raporcie boot. Wpis nie zmienia stanu odbioru 0.38.
