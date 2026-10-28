# G01 local operations

Place the state directory on a filesystem that supports atomic rename, file sync, and advisory file locking. Keep it private to the operator account and back it up together with its `artifacts/` directory. Keep the operator private key outside the state directory. Preserve the original journal and artifacts when investigating a failure; `status` and `next` replay and verify them before showing state.

An interrupted `prepare` or rejected `handoff` does not complete a stage. A temporary file may remain after a crash and may be removed after confirming no process is active. Do not manually edit the journal or artifact files. If verification fails, restore a consistent backup or initialize a new store and repeat the evidence and operator handoff. No automatic recovery invents a completion.

The package intentionally has no live Zenoh, OSTc, source access, or active agent admission. The registry's Zenoh flag records future integration work. Local build and tests, a review of source/test evidence, and a signed receipt do not establish deployed service health or cross-host delivery.
