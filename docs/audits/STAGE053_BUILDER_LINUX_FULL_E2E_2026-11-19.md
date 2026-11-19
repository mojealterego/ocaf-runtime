# Stage 0.53 — Linux native functional E2E acceptance

- Source commit on main: `353f92ed11b76e04f26947b5066e34932f2da487`
- Tested local source commit: `7b428f7cde1cfce88c790456ca47aca75cfcb780`
- Identical complete Git tree: `35974dcafa8def0ef685dfbecfadbbd09c68f201`
- Prior base: `248bc194347973c23682e83dfc63e9ba30a027c6`
- Runner: existing isolated Linux x86_64 lab ocaf-stage038-host-b; Rust 1.99.0, Python 3.13.7, Node 24.18.0, WebKitGTK 2.52.6, tauri-driver 2.1.0.
- Distribution: native Linux ELF, copied to private isolated installation directory. This is not a Kali installation or proof of compatibility across all Linux distributions.

## Acceptance

| Scope | Result |
|---|---|
| Local Code / NoCode: create, edit, validate, canonical Rust compile, file import/export | PASS |
| Cloud Code / NoCode: same operations, real mTLS and OAuth PKCE / SQLite CAS | PASS |
| Foreign token/tenant and foreign import denial | PASS |
| Local workspace isolation, no-overwrite private export and restart recovery | PASS |
| Locked release build, executable format and shared libraries | PASS |
| Rust native unit tests | 4/4 |
| JavaScript regressions | 67/67 |
| Source bundle coverage | 3/3 |
| Full-stack Rust integration with real GUI | 1/1 |
| Lockfile SBOM | 451 records (not license review or full system SBOM) |
| Manifest and 12 constituent file digests | PASS; rechecked after test run |
| Ephemeral lab Ed25519 verification and tamper rejection | PASS |
| Operator-trusted production signature, full GATE-10 | UNVERIFIED / NOT_ACCEPTED |

Command: `bash scripts/verify_builder_linux_e2e.sh 7b428f7cde1cfce88c790456ca47aca75cfcb780 /absolute/new-evidence /path/to/tauri-driver` executed on source checkout with that original commit; public commit reproduces the exact tree. A fresh run must specify its own exact current source SHA and new evidence directory.

RED: native transport tests failed before implementation; extended native GUI failed at local Rust compile before implementation. GREEN: final committed source ran both installed-ELF GUI suites and full stack. Real fixture uses actual existing Rust authorization and persistence, ephemeral credentials, no remote production effects; no mocked gateway. Lab signer is NOT authorized operator signer. No manually dispatched paid CI.

## Evidence hashes

- ELF: `67e050d478cefb288fd315cd6211df7f3144257bc1f0c4ea80536dcc17a00f77`
- Lockfile SBOM: `a76cf27f6caba1affb6c26b2525913070f036c3448a3629225b9bfdc3e96f5f7`
- Signed engineering manifest: `19aacee2c0b6826ff2aa08661db40304dd24f39f9b73a1f7a9aa13ded027c7e4`

Published exact [manifest](evidence/linux-builder-2026-11-19/LINUX_ENGINEERING_EVIDENCE.json), [local GUI receipt](evidence/linux-builder-2026-11-19/local-gui.json), [Cloud GUI receipt](evidence/linux-builder-2026-11-19/cloud-gui.json), [lab public key](evidence/linux-builder-2026-11-19/lab-public.pem), [lab signature](evidence/linux-builder-2026-11-19/lab-signature.bin), [signature verification log](evidence/linux-builder-2026-11-19/signature-test.log), [environment](evidence/linux-builder-2026-11-19/environment.txt), [Rust tests](evidence/linux-builder-2026-11-19/desktop-tests.log), [full stack](evidence/linux-builder-2026-11-19/cloud-stack.log), [JS tests](evidence/linux-builder-2026-11-19/web-tests.log). Binary, full lockfile SBOM and remaining logs retained in private lab evidence directory `/tmp/ocaf-linux-e2e.BAdwNd/evidence-7b428f7`; not committed as release assets. The manifest names original tested commit, not the later publishing commit.

## Remaining gates

Functional engineering acceptance for Linux x86_64 on this runner only. Production release admission requires operator-signed provenance and final aggregate release verification; no trusted key supplied for signing. Neither production approval nor complete ecosystem release granted. Proceed sequentially to Kali Linux; don't count the earlier extracted Ubuntu package test as Kali native evidence.
