# Asterion · OCAF-AGENT-001 — Build evidence

Local development run, 2026-10-28 UTC:

- Source base: `8e6ee742210ec29312d60c56e3d2616dd1fbb101`, with Asterion
  changes in the delivery commit.
- `cargo test --locked`: **PASS**, 4 integration and 4 security tests.
- `packaging/build.sh`: **PASS**; reran tests and built the release binary.
- Archive: `dist/ocaf-agent-001-asterion-0.1.0.tar.gz` (471556 bytes
  after deterministic tar metadata).
- Archive SHA-256:
  `b2a9fcf2d4897cd15c70ede2be45ac93e51895fb29fb1ea05e3f69d67304787f`.
- Two consecutive builds from the same source and binary produced that same
  archive hash after sorting entries, fixing tar metadata and omitting gzip
  timestamps. This is local reproducibility only.
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

[CI run 40790950576](https://github.com/mojealterego/ocaf-runtime/actions/runs/40790950576)
completed successfully for source commit
`d8e00cbe220b9c619b4f93485e2ce0fd76a59bbc` and uploaded artifact
`ocaf-agent-001-asterion-draft-d8e00cbe220b9c619b4f93485e2ce0fd76a59bbc`
(artifact ID `12860724609`). The remote artifact could not be downloaded from
this workspace because GitHub's backing blob URL returned HTTP 403, so the
local SHA-256 above is not asserted to be the CI archive SHA-256. The workflow
checks its own archive checksum before upload.
