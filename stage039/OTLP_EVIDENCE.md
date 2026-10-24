# Stage 0.39: OTLP evidence bridge

The KVM workflow exports two verified guest receipts as OTLP/HTTP JSON spans
to a real HTTP collector bound to `127.0.0.1` on the CI host. It checks that
the collector received exactly the exported bytes, records their SHA-256, and
keeps both tenant trace IDs distinct. Each span's trace and span ID come from
the W3C `traceparent` derived from the authenticated broker lease, passed in
the Firecracker boot arguments, and observed by PID1 in `/proc/cmdline`.

The bridge consumes `broker-wasi-vm.json` and `real-guest-wasi-execution.json`
only after the existing KVM checks prove signed guest modules, kernel BPF
`openat` events, exact WASIp1 output, and the VM fault receipts. It rejects
changed tenant, VM, trace, module, serial or event values before transport.
The OTLP span timestamp marks **post-run export time**, not guest event time.

Run it after a successful real KVM workflow:

```sh
python3 scripts/stage039_otlp_evidence.py \
  --broker-report stage039-real-guest-evidence/broker-wasi-vm.json \
  --guest-report stage039-real-guest-evidence/real-guest-wasi-execution.json \
  --report stage039-real-guest-evidence/loopback-otlp-receipt.json
```

This proves loopback OTLP export and receipt continuity on one CI host. It
does not instrument the guest application, reach an independent collector or
prove two-host trace continuity. The receipt explicitly records those limits
and leaves `stage_039_complete=false` and `production_approved=false`.
