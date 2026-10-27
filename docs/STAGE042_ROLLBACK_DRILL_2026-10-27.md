# Stage 0.42.4 — signed release rollback rehearsal

Run [40453820456](https://github.com/mojealterego/ocaf-runtime/actions/runs/40453820456)
completed on a disposable GitHub Actions Linux runner. It used the candidate
artifact from supply-chain run `40439264747`, commit
`e9a3dec77a43beaf8287cc01eb9471f35577e6c4`, and the prior artifact from
run `40412316181`, commit `d7e85204b851434c9ef233a0ce29dfdbc0065f66`.

The workflow checked that the prior commit is an ancestor, validated both
bundles' manifest SHA bindings and distinct archives, and ran GitHub's
cryptographic build and CycloneDX SBOM attestation verifier for **each**
archive with its exact commit and pinned signer workflow. Only after these
checks passed did it execute `ocaf-cluster-health` from candidate and prior
archives in separate temporary directories. Both returned exit code 1 and
the expected `unknown option` error for an intentionally invalid argument.
The executable SHA-256 values were different:

- Candidate: `0e96da3c0e01f186446258e73e76e2b087470b78639dc823bdda61ae909fa7bb`
- Prior: `9f049a237ecd7cd21f88270d45ea7bba54cd0448198f6223bd2adbac4b80c68f`

A second run, [40454393345](https://github.com/mojealterego/ocaf-runtime/actions/runs/40454393345),
repeated those checks and added real disposable PostgreSQL 16 state. The
candidate binary observed two pending jobs. The runner restored a custom
`pg_dump` snapshot captured before the second job was inserted; the prior
binary then observed exactly one pending job. This demonstrates one database
snapshot recovery and a prior-version read against restored state on an
ephemeral runner.

The standalone Python report deliberately says `signature_verified: false`:
the Python program does not verify GitHub signatures. The preceding workflow
step verified both attestations for both archives and is the signature
evidence. This rehearsal exercises one CLI error path and one PostgreSQL
read after restoring a small test snapshot. It does not roll back a real
schema migration, credentials, agent runtime, or production deployment.
It does not establish behavior after secret revocation or on a
durable host. Stage 0.42.4 remains technically open for those cases.
