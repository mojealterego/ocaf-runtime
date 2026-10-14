# Railway trial smoke probe (Stage 0.38 — NOT full acceptance)

An opt-in Railway build of `Dockerfile` produces a one-shot executable
`ocaf-railway-probe` that loads and executes the actual OCAF Rust library.
The probe validates local tool registration/execution, schema rejection,
unknown-tool rejection and invalid tenant namespace rejection.

The image binds **no public port**; requires **no secrets**, DB or service account;
and exits immediately after writing one bounded JSON verdict.

**Boundaries:** This is a disposable Railway container, not a persistent
systemd PID 1 virtual machine. It does not exercise signed WASI, cgroup
hardening, remote mTLS or two-tenant multi-host behavior. It MUST NOT be
used to grant Stage 0.38 production approval.

Execution is optional. Only a Railway Trial (up to its current free credits)
is authorized; do not upgrade plans or enable persistent workloads. The
Railway service should use restart policy NEVER (and no scheduled cron).
