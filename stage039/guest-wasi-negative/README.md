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

## KVM integration contract

1. Copy and SHA-pin both WAT fixtures plus the static seccomp probe inside each
   tenant's read-only Firecracker guest image. Verify exact copied bytes before
   signing the rootfs manifest.
2. Guest PID1 must run the negative Wasmtime modules with the same `-W` flags,
   a bounded host process supervisor, no-new-privileges, and CPU/memory/time
   limits. Require the exact fuel, timeout, and memory trap from the guest's
   Wasmtime child, and a real `SIGKILL` status for the independent spin child.
3. Run the seccomp probe in the guest and require the kernel `EPERM` marker.
   Bind all negative receipts to the VM ID, tenant nonce, broker lease, signed
   module SHA and guest kernel identity. Tenant B must remain alive when A is
   killed or denied.
4. Reuse the existing live jailed TAP/TC kernel eBPF packet deny/recovery
   evidence and test broker revocation, outage and replay. Sign the combined
   audit receipt with an operator-provisioned host execution authority.

The current JSON report deliberately says `guest_kvm_tested=false` and
`stage_039_complete=false`. Local host traps, cgroup configuration and static
seccomp code do not meet the full 0.39.21 guest, TC and signed receipt gate.
