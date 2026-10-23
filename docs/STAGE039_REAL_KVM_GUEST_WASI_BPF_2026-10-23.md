# Stage 0.39: rzeczywisty KVM, guest WASI i guest eBPF — przebieg 2026-10-23

## Źródło dowodu

- Commit: `e6ad6b2a4027c98e182afb073e43b70aeb33c727`.
- Workflow: [run 40015144582](https://github.com/mojealterego/ocaf-runtime/actions/runs/40015144582), wynik `success`.
- Artefakt: [stage039-real-firecracker-jailer-linux-evidence, ID 12528716158](https://github.com/mojealterego/ocaf-runtime/actions/runs/40015144582/artifacts/12528716158).
- SHA-256 archiwum artefaktu według GitHub Actions: `73f94a7047122b7004dae3c4f6fd62a0fc83ed37520055f7f3f0ee801e72018f`.
- Testy kontraktowe dla `test_stage039*.py`: 150/150 przed wypchnięciem commitu.

## Fakty potwierdzone przez artefakt

| Zakres | Dowód |
| --- | --- |
| Sprzętowy KVM | `kvm.json`: rzeczywisty `KVM_CREATE_VM`, `kvm_vm_fd_created=true`. |
| Broker i dwa VM | `broker-wasi-vm.json`: dwa różne identyfikatory jailed Firecracker VM, autoryzacja `SO_PEERCRED`, rezerwacje SQLite przed startem, zwolnienie lease po SIGKILL. |
| Podpisane manifesty | Oba manifesty Ed25519 sprawdzone przed rezerwacją względem zaufanego klucza laboratoryjnego; `signed_wasi_manifest_verified=true`. |
| WASIp1 wewnątrz gościa | `real-guest-wasi-execution.json`: oba goście zgłosiły `fd_write`, kod wyjścia 0 i różne SHA-256 surowego seriala. |
| eBPF wewnątrz gościa | Każdy guest PID 1 załadował i przyłączył program `BPF_PROG_TYPE_SOCKET_FILTER` przez syscall `bpf`, po czym odczytał dodatni kernel program ID (`1` w każdym odrębnym jądrze VM). |
| eBPF na hoście | `jailed-guest-network.json`: TC/eBPF na TAP gościa A, host kernel program ID `107`, odcięcie A, działanie B i odzyskanie ruchu. |

## Granice dowodu

To jest poprawny wynik **runnera laboratoryjnego**. Nie zamyka całego etapu 0.39 ani nie zatwierdza produkcji: raporty zachowują `stage_039_complete=false` i `production_approved=false`.

- Klucz podpisujący został wygenerowany w tym samym przebiegu CI. Produkcyjna kotwica zaufania i niezależne podpisanie modułu WASM nadal wymagają wdrożenia.
- Oba VM użyły tego samego zadania WASM; `different_wasm_task_per_tenant_tested=false`. Brakuje osobnych wejść i nonce związanych z tenantem oraz negatywnego testu podmienionego modułu wewnątrz VM.
- Guest eBPF potwierdza załadowanie filtra socketowego w jądrze gościa. Deskryptor filtra jest zamykany przed uruchomieniem Wasmtime. Nie jest to jeszcze sonda Aya `openat` ani egzekwowanie polityki dla procesu Wasmtime w gościu.
- Oba VM działały na jednym hoście KVM. Test nie dowodzi niezależnych hostów ani pełnego egress broker z testami SSRF/DNS rebinding.

Następne bramki 0.39 wynikają z powyższych luk; status Roadmapy pozostaje `ACTIVE_TECHNICAL_OPEN`.
