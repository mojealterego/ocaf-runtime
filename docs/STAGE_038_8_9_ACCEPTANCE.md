# OCAF 0.38.8–0.38.9 — Linux WASIp1 subprocess hardening / acceptance

**Branch:** `main` only. **Decision:** engineering security work in progress;
**production: NO-GO**. This document records executable checks and what they
**do not** establish. The final GitHub Actions status must be read for the
final `main` SHA; a successful prior revision is not release evidence.

## Security boundaries

Core OCAF authenticates tenant/agent and enforces ACL, budgets, replay and
fencing. The restricted WASIp1 adapter executes an operator-approved module
using a separately pinned Wasmtime CLI. A signed manifest and input validation
are admission controls; neither is an OS kernel isolation primitive.

### 0.38.8 — lifecycle and process hardening

| Control | Implementation / evidence | Guarantee boundary |
| --- | --- | --- |
| Cancellation/timeout | `WasiProcessGroup` in `src/executor/wasi.rs`; process is started in a dedicated Linux process group using `process_group(0)` and group is signalled with `SIGKILL` on Drop. `kill_on_drop(true)` is retained. | Kills processes still in that group. A compromised native engine that invokes `setsid`, escapes the group, or is able to interfere with the supervisor is **not** contained by this check. |
| No new privileges | Child-only `PR_SET_NO_NEW_PRIVS=1` via `pre_exec`, spawn fails closed if the syscall fails. | Limits privilege gain via `execve`; not an alternative to seccomp, namespaces, cgroups or microVM. |
| Core files | Child-only `RLIMIT_CORE` soft/hard = 0; spawn fails closed on error. | Limits guest core-dump disclosure, not arbitrary host reads from a compromised native engine. |
| Bounded subprocess I/O | Existing independent stdout/stderr limits and guarded stdin, wall deadline, fuel, Wasm memory/stack limits; per-executor concurrency. | Wasmtime compiler/process RSS and Linux CPU consumption are not fully bounded by Wasm fuel. |
| Private workspace | `EphemeralModule` staged under `/tmp/ocaf-wasi-UUID`, directory `0700`, module `0600`, cleaned on Rust Drop. | Abrupt host termination (SIGKILL/power loss) bypasses Rust destructors; startup residue management remains a deployment concern. |

### 0.38.9 — repeatable CI acceptance

| Check | File / test | Expected behavior |
| --- | --- | --- |
| Cancellation of child and background descendant | `tests/wasi_sandbox.rs::cancelling_guest_kills_background_descendants` | Descendant is not running after cancellation; Linux `/proc` supports state check. |
| Prior cancellation and cleanup | `cancellation_reaps_guest_and_removes_private_workspace` | Child terminates and temporary module is removed. |
| Native process privileges/core limit | `wasi_guest_inherits_no_new_privileges_and_core_limit` | Child sees `NoNewPrivs: 1`, `ulimit -c = 0`. |
| stdout and stderr bombs | `stdout_bomb_is_rejected_without_host_log_leak`, `stderr_bomb_is_bounded_and_not_returned_to_callers` | Limit violation; no guest stderr returned to requestor. |
| Wasm parser mutations | `mutated_wasm_binaries_do_not_panic_import_validation` | Bounded deterministic input-mutation smoke test, not coverage-guided fuzzing. |
| Signed policy/tenant isolation | `publisher_signature_binds_module_tool_policy_and_principal` | Reject wrong signer, tenant, agent, manifest, digest. |
| Real Wasmtime 49.0.2 | `scripts/wasmtime_acceptance.sh` | SHA-256 checked before extraction; run the **entire** `wasi_sandbox` integration suite with the real pinned engine; bounded execution repetition, fuel trap, memory rejection. |

Run the documented CI acceptance on Linux x86_64:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features --test wasi_sandbox -- --test-threads=1
bash scripts/wasmtime_acceptance.sh
cargo test --locked
```

Production GO requires green Rust CI **and** green Supply Chain / Provenance
on the final SHA; both are necessary but **not sufficient**. Release review
must additionally cover:

1. Independent multi-tenant concurrent execution and leak/soak testing,
   including measured RSS, file descriptors, CPU, PIDs and residual files.
2. Independently administered host-level hardening (unprivileged account,
   restricted writable mounts and immutable pinned-engine path),
   anti-TOCTOU review of engine `execve`, parent-directory ownership,
   and system-wide process limits.
3. Recovery after OCAF `SIGKILL`, process crash, host restart and partition;
   no automatic replay of uncertain external effects.
4. Publisher key rotation and revocation, tool provenance and upstream
   Wasmtime security advisories; artifact signature beyond a bare digest.
5. Full independent adversarial security assessment, signed evidence and
   documented operator acceptance.

**Open risk:** process-group cleanup cannot contain a malicious Wasmtime
binary escaping its group, arbitrary native syscalls, or host compromise.
Kernel-level seccomp/cgroups/namespaces and Firecracker/eBPF are scoped to
0.39. Tests in this stage are implementation evidence, not third-party
attestation and not a production security certification.

## Rozszerzenie 0.38 — anti-TOCTOU, awarie i dowody z hosta

### Niezmienność wykonywanego silnika

`PinnedWasmtime` otwiera plik binarny przez `O_NOFOLLOW`, sprawdza
`fstat` otwartego deskryptora (plik zwykły, jeden hardlink,
root/effective-UID owner, brak zapisu grupy/świata, executable, rozmiar),
kopiuje dokładne bajty do anonimowego Linux `memfd`, oblicza SHA-256
w trakcie kopiowania i zakłada `F_SEAL_WRITE/GROW/SHRINK/SEAL`.
Proces potomny wywołuje **uszczelnioną kopię** przez
`/proc/self/fd/N`, a nie ponownie otwartą ścieżkę. Deskryptor
jest `CLOEXEC` w koordynatorze; tryb jest odblokowany tylko
po `fork` w procesie potomnym. Brak `memfd`, brak dostępu
do `/proc/self/fd` lub błąd seal oznacza odmowę startu,
bez awaryjnego fallbacku do mutowalnej ścieżki.

Wymagania operacyjne: Linux z `memfd_create`, `F_ADD_SEALS`
i działającym `/proc/self/fd`, administrowana instalacja Wasmtime.
Nie zastępuje to ochrony konta usługi, kluczy i innych binariów.

### Restart koordynatora / SIGKILL

Dziecko otrzymuje `PR_SET_PDEATHSIG=SIGKILL`, z kontrolą PID
rodzica po `prctl`. Sygnał dotyczy *bezpośredniego* potomka;
nie jest dziedziczony przez dalsze procesy. Wdrożenie pod
`deploy/systemd/ocaf-coordinator-wasi.service.example` oferuje
`KillMode=control-group` oraz limity systemd `MemoryMax`,
`CPUQuota`, `TasksMax` i osobny `PrivateTmp`. **Jest to
szablon, a nie potwierdzona konfiguracja żadnego hosta.**
Wdrożenie, crash test i potwierdzenie rzeczywistego cgroup v2
wymagają docelowego serwera z operatorem systemd.

### Powtarzalny hostowy soak i artefakt dowodowy

`scripts/wasi_host_acceptance.py` uruchamia pełny zestaw
`wasi_sandbox` na rzeczywistym silniku wskazanym przez
`OCAF_TEST_WASMTIME`, wielokrotnie i z timeoutem.
Tworzy wersjonowany raport JSON (SHA-256 silnika, commit,
czas, próbki RSS/fd/PID, wynik testów, obecność nowych
katalogów tymczasowych). Dane `/proc` są **próbkowane**:
sumowanie RSS współdzieli strony; krótkich pików nie gwarantuje
uchwycić. Nie jest to dokładny miernik cgroup `memory.peak`.
Niczego nie usuwa z hosta i nie przesyła kluczy ani sekretów.

CI używa dwóch cykli jako smoke soak; artefakt
`wasi-host-acceptance` przechowuje raport dla audytu.
Powtórzenie na docelowym hoście z realnym cgroup,
przy obciążeniu oraz długotrwałym run, jest osobnym gate'em.

Przykład operatora (po instalacji zatwierdzonego Wasmtime):

```bash
export OCAF_TEST_WASMTIME=/opt/ocaf/wasmtime/wasmtime
export OCAF_TEST_WASMTIME_SHA256='<approved-real-binary-sha256>'
python3 scripts/wasi_host_acceptance.py \
  --cycles 100 --timeout 120 \
  --max-rss-kib 1572864 --max-fds 512 --max-processes 128 \
  --report /var/lib/ocaf/wasi-soak-acceptance.json
```

Limity w przykładzie to **parametry do zatwierdzenia przez
operatora**, nie pomiary konkretnego hosta ani gwarancja.
Produkcja wymaga osobnej oceny peak RSS i cgroup `memory.peak`,
kontroli wspólnych tenantów, przeciążenia, restartu,
partition recovery i effect-fencing.

### Macierz dostępu do zamknięcia GO

| Dostęp / dowód | Potrzebny do | Stan |
| --- | --- | --- |
| GitHub push i Actions | Implementacja, jednostkowe testy, realny Wasmtime CI i artefakt smoke | Dostępny; końcowy SHA wymaga zielonych przebiegów |
| Operator Linux x86_64, systemd+cgroup v2, SSH/console z prawem wdrożenia | Faktyczne `MemoryMax`, `CPUQuota`, `TasksMax`, SIGKILL restart, chaos, pomiary | **Brak dostępu do hosta** |
| Rzeczywiste wielodzierżawne środowisko staging z TLS/mTLS | Próby krzyżowe, awarie RPC, obciążenie na wielu maszynach | **Brak potwierdzonego środowiska** |
| Opiekun podpisów Ed25519 / trust anchor (bez podawania kluczy prywatnych w czacie) | Wymuszenie rotacji i odebrania zaufania w każdym węźle | Procedura do wykonania przez operatora |
| Niezależny ekspert bezpieczeństwa | Threat modeling, audyt kernel escape/side channel, sign-off | **Nieprzeprowadzony** |

**Zasada odbioru:** nawet przy pełnym CI GREEN status produkcyjny
pozostaje **NO-GO**, dopóki dowody z realnej infrastruktury i
niezależny podpisany protokół akceptacji nie zostaną zatwierdzone.
Etap 0.39 nie jest automatycznie rozpoczynany.

## Formalny odbiór operatora

Procedury wdrożenia cgroup/systemd, chaos-staging, weryfikacji
wielodzierżawności i rotacji/odwołania zaufania: 
[STAGE_038_OPERATOR_SIGNOFF.md](STAGE_038_OPERATOR_SIGNOFF.md).
Dokument zawiera formularz **NO-GO** do wypełnienia przez
operatora oraz niezależnego audytora; żaden sukces CI nie
podmienia podpisu operatora.


## Dodatkowa automatyzacja odbioru hosta (2026-10-14)

- `scripts/stage038_host_gate.py`: read-only, fail-closed preflight
  Linux x86_64 / PID1 systemd / cgroup v2 / memfd seals / PDEATHSIG;
  opcjonalny rygorystyczny odczyt rzeczywiście uruchomionej jednostki
  `--unit ocaf-coordinator-wasi.service`, w tym limitów cgroup, licznika
  `memory.peak`, `cpu.stat`, `pids.current` i `memory.events`.
- `scripts/tests/test_stage038_host_gate.py`: walidacja bezpiecznej składni
  jednostki, surowych limitów zasobów, odmowy przy niekompletnych właściwościach,
  atomowego create-only report oraz niezmienności stanu NO-GO.
- `.github/workflows/stage038-host.yml`: cykliczny test na ephemerycznym
  runnerze Ubuntu x86_64, którego wynik nie jest fizycznym staging OCAF.
- `scripts/wasi_host_acceptance.py` odmawia uruchomienia, jeśli brakuje
  zatwierdzonego `OCAF_TEST_WASMTIME_SHA256` lub jest nieprawidłowy;
  raport JSON zapisywany 0600 i wyłącznie w nowej ścieżce.
- CI wykonuje 10 powtórzeń pełnego testu `wasi_sandbox` na prawdziwym
  pinned Wasmtime z próbnikami RSS/FD/PID, a nie 2. Próg RSS w CI
  jest smoke-budget dla efemerycznej VM, nie operacyjnym SLO hosta.

**Ograniczenie:** CI nie stanowi potwierdzenia rzeczywistego działającego
OCAF pod wdrożonym systemd na prywatnym staging, restartów hosta,
sieciowego partition, rozliczenia efektów ani niezależnego audytu.
Etap 0.39 pozostaje zablokowany do spełnienia protokołu odbioru.
