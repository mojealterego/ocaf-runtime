# Stage 0.39.21 negative WASIp1 budget fixtures

This increment executes three *real* Wasmtime v49.0.3 traps on the current
Linux host: fuel exhaustion, Wasm timeout, and bounded linear-memory growth.
It observes a kernel `RLIMIT_CPU` signal and kills a separate running Wasmtime
child process group with `SIGKILL`. A separate
statically linked C probe installs a kernel seccomp filter and proves that a
socket syscall returns `EPERM` while unrelated process completion still works.
The Wasmtime release archive is already pinned by the stage 0.39 guest image
builder; the negative harness additionally pins the executable SHA-256 to
`b213b0678c0afc807c53130e5b7ca357f0307c1f8deb18cac35fcc65480b9c85`.
Each WAT fixture imports WASIp1 `fd_write` and is pinned by an exact SHA-256.

Run the actual-host checks after downloading the approved Wasmtime archive:

```sh
OCAF039_WASMTIME=/absolute/path/to/wasmtime \
  python3 -m unittest discover -s scripts/tests \
  -p test_stage039_wasi_budget_lab.py -v
```

The standalone harness can write a bounded JSON report:

```sh
python3 scripts/stage039_wasi_budget_lab.py \
  --wasmtime /absolute/path/to/wasmtime \
  --sha256 b213b0678c0afc807c53130e5b7ca357f0307c1f8deb18cac35fcc65480b9c85 \
  --report /absolute/path/to/unused-report.json
```

## Guest KVM integration candidate

The image builder verifies exact WAT source hashes, compiles both modules to
WASM, builds a static guest negative supervisor, copies all three into each
tenant rootfs and compares the copied bytes through `debugfs` before the
rootfs digest is signed. The supervisor executes real Wasmtime fuel, timeout
and bounded memory traps, receives a kernel CPU signal, kills a separate
Wasmtime process group with `SIGKILL`, and installs a kernel seccomp filter
that denies `socket()` with `EPERM`. Every child is bounded by CPU, address
space, output and elapsed time. Guest PID1 emits the six receipts only after
the supervisor's complete exact output and exit zero; the host serial verifier
requires each marker once, in order, after the tenant nonce and before PASS.
The supervisor has been executed on a Linux host against pinned Wasmtime, but
its Firecracker guest behavior still requires a green real KVM workflow run.

Remaining 0.39.21 gates include broker revocation/outage/replay, combining
these negative receipts with guest TC deny/recovery, and signing the audit
receipt with an operator-provisioned execution authority. A full milestone
closure also requires the other open 0.39 gates.

The current JSON report deliberately says `guest_kvm_tested=false` and
`stage_039_complete=false`. Local host traps, cgroup configuration and static
seccomp code do not meet the full 0.39.21 guest, TC and signed receipt gate.
