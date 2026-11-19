# Stage 0.53 — Kali Linux installed package and four-mode engineering E2E

## Source and environment

- Published source commit: `18521af4d2ea65452c6d7c7c9ecb5afca4a0ca76`.
- Tested local commit: `a1dffcb28e7ff2d93047f142ea7a157c04d19bcb`; exact identical Git tree `5d792fb0618b06f2bf19c55372f196bb60f9aff3`.
- Base: `c80b8c654c3ef6d08f3adad644107b9d1d6ffa5c`.
- ELF source: Linux verified tree `35974dcafa8def0ef685dfbecfadbbd09c68f201`, published commit `353f92ed11b76e04f26947b5066e34932f2da487`; Kali changes packager/test harness, not binary code.
- Runner: existing isolated Linux x86_64 laboratory B. Separate PID/mount namespace with real Kali userland chroot, unprivileged GUI user, Xvfb, WebKitWebDriver and tauri-driver 2.1.0. Real host kernel is shared; not Kali bare metal, independent kernel or physical device.
- Kali GNU/Linux Rolling 2026.3 amd64, glibc 2.43-7, GTK 3.24.52-3, WebKitGTK 2.52.6-1, OpenSSL runtime 3.6.5-1. Apt obtained actual dependencies from signed Kali package repositories. No new cloud host or manually dispatched CI.
- Official OCI image `kalilinux/kali-rolling`: amd64 image manifest SHA-256 `12e581978942a815b35ab2406725c274a4f28825650595d2ec2b7fb7668b039d`; layer SHA-256 `a63fa61224687d45438ccafa87a6c0e3061157dd8c0e33828e49af3a8ae2ce51` (53,217,105 bytes). Both verified before extraction. [Raw OCI manifest](evidence/kali-builder-2026-11-19/kali-oci-manifest.json).

## Changes and execution

The packaging regression failed RED for the missing explicit native TLS runtime dependency. Package now declares libc6 >= 2.39 (actual ELF requirement), GTK/GLib t64 alternatives and libssl3t64 or libssl3. GREEN 6/6. Added isolated Kali namespace/test harness and a test-only Rust full-stack GUI adapter. Package coverage requires all new harness files, 3/3 PASS. No permissions granted to production application by the test harness. Test-only ephemeral fixture key/cert files are cleaned up; no production identity is used.

Repeatable verification on an operator-prepared disposable rootfs:

1. Provision official digest-verified Kali Rolling amd64 rootfs under private `/tmp/ocaf-kali-e2e.*/rootfs`; install actual dependencies from signed Kali archives, including `xvfb xauth webkitgtk-webdriver python3 dbus-x11 fonts-dejavu-core xdg-user-dirs`. Create unprivileged `ocaf` account matching invoking UID/GID and private home. Keep package service-start policy disabled in disposable rootfs.
2. Provide verified native ELF and compatible tauri-driver. Run on the lab host with authorized sudo for mount/PID namespace and chroot only:
   `bash scripts/verify_builder_kali_e2e.sh <exact-current-main-sha> <rootfs> <new-absolute-output-dir> <verified-elf> <tauri-driver>`
3. The actual run used local source SHA `a1dffcb28e7ff2d93047f142ea7a157c04d19bcb`; published source tree is identical. The harness creates and compares two packages, actually installs via dpkg, validates binary SHA and linked libraries, runs native GUI Local and authenticated Cloud, regenerates SBOM, signs and verifies the engineering manifest with a one-time lab key, rejects tampering.

Initial setup failed because the unprivileged test account was absent from the Kali password database; corrected lab setup. A first GUI run lost WebDriver connection during workspace switch; rerun and final unchanged-source harness passed. No native application fix is claimed for that transient failure. The source is not asserted to eliminate driver flakiness.

## Results

| Requirement | Result |
|---|---|
| Actual dpkg install in Kali userland | PASS |
| Package reproducibility (2 identical DEB builds) | PASS |
| Local Code / NoCode real GUI | PASS |
| Cloud Code / NoCode real GUI with actual mTLS/OAuth/SQLite | PASS |
| Create/edit/validate/Rust compile, import/export, persistent restart | PASS |
| Cross-tenant token/import denial, CAS, Local workspace separation | PASS |
| Installed ELF identity and dynamic libraries present | PASS |
| Package regression | 6/6 |
| Source bundle coverage | 3/3 |
| Full-stack Rust integration including Kali GUI | 1/1 |
| Cargo lockfile SBOM | 451 records |
| Installed system package inventory | 343 rows |
| All 17 signed engineering-manifest file digests | PASS, rechecked |
| Ephemeral Ed25519 signing and tamper rejection | PASS |
| Operator-trusted signing, independent kernel/device, aggregate GATE-10 | UNVERIFIED / NOT_ACCEPTED |

## SHA-256

- DEB: `5ce09577c6a1cdf280c07a0364ac270952416566e601927660dd21f4036e20c2`
- Installed ELF: `67e050d478cefb288fd315cd6211df7f3144257bc1f0c4ea80536dcc17a00f77`
- Cargo lockfile SBOM: `5cb97e090aa9412c617bb1ced886238d74f28ea4fc4b8c1eb685904b4daf2823`
- Signed engineering manifest: `58f25aba85ab780cfdab53d2eac7683736428d410c58629c8ffda8d800849cf7`

[Manifest](evidence/kali-builder-2026-11-19/KALI_ENGINEERING_EVIDENCE.json) · [Local GUI](evidence/kali-builder-2026-11-19/local-gui.log) · [Cloud GUI](evidence/kali-builder-2026-11-19/cloud-gui.log) · [dpkg install](evidence/kali-builder-2026-11-19/install.log) · [OS identification](evidence/kali-builder-2026-11-19/os-release.txt) · [package receipt](evidence/kali-builder-2026-11-19/package.json) · [lab key](evidence/kali-builder-2026-11-19/lab-public.pem) · [lab signature](evidence/kali-builder-2026-11-19/lab-signature.bin) · [signature test](evidence/kali-builder-2026-11-19/signature-test.log). Full artifact and remaining evidence retained at `/tmp/ocaf-kali-e2e.4h21rE/evidence-a1dffcb` on the existing lab host, not publicly released. Published evidence is engineering receipt, not operator release attestation.

Kali four-mode functional E2E completed on actual Kali userland. Device/kernel-specific verification and trusted operator release signature remain distinct open gates. Next platform Android; do not count this as verification of other platforms.
