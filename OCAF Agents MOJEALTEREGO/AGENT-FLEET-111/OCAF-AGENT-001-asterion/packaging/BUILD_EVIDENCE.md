# Asterion · OCAF-AGENT-001 — Build evidence

Local development run, 2026-10-28 UTC:

- Source base: `8e6ee742210ec29312d60c56e3d2616dd1fbb101`, with Asterion
  changes in the delivery commit.
- `cargo test --locked`: **PASS**, 4 integration and 4 security tests.
- `packaging/build.sh`: **PASS**; reran tests and built the release binary.
- Archive: `dist/ocaf-agent-001-asterion-0.1.0.tar.gz` (471779 bytes).
- Archive SHA-256:
  `6a9c13c1559a109beda219f254910bbfb59f603c373a5cd3ae579f4c0662adda`.
- Archive installation, installed `status`/`plan`/`claim`, uninstall:
  **PASS** in an isolated temporary prefix.
- Installer rejected a forged archive whose checksum manifest omitted the
  binary: **PASS**.
- Installer rejected prefix `/` and symlinked `bin` and `share`
  destinations; a valid isolated prefix still installed: **PASS**.

The archive is a local development artifact and is ignored by Git. It has no
publisher signature. Certificate status remains **NOT ISSUED**. A SHA-256
digest identifies bytes only; it does not establish authenticity or
production readiness.

The `ocaf-agent-001-installer.yml` GitHub Actions workflow rebuilds the
archive from the pushed commit, checks its SHA-256, installs it in a disposable
runner prefix, verifies `DRAFT` and `ACTIVE=DENY`, then uploads the archive
and checksum for operator review. The workflow artifact is retained for seven
days; its run ID and result must be checked separately before citing it as
remote build evidence.
