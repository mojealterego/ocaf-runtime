# OCAF v0.30 — RustSec remediation of Zenoh transport (2026-10-13)

## Goal and release policy

The upstream current `zenoh 1.10.1` cannot resolve its
`lz4_flex 0.10.0` transitive crate to a secure patch release merely
by changing OCAF's root `Cargo.toml`. Its `zenoh-transport` manifest
requires `^0.10`, while `RUSTSEC-2026-0041` is fixed in
`lz4_flex >=0.11.6,<0.12` or `>=0.12.1`.

A deliberately **single-crate**, source-controlled backport is staged
through `scripts/vendor_zenoh_transport.py`. It copies the original
`zenoh-transport 1.10.1` package already downloaded and validated by
Cargo from crates.io (hash in the existing `Cargo.lock`) and changes
only its dependency constraint to `lz4_flex 0.11.6`. It adds an explicit
`[patch.crates-io]` path override so all other Zenoh crates continue
using upstream registry release packages. Cargo regenerates the lockfile.

## Transactional acceptance

This migration runs via a **path-scoped one-shot GitHub Actions job**
that stays on `main` and pushes the resulting source and lockfile
**only if** focused Zenoh tests, core tests, Clippy, feature controls,
new regression tests and source-integrity guards pass.

On any compilation or API compatibility failure, the job fails
and leaves the repository without the unverified patch. No branch
creation or silent force-push is permitted.

The repository dependency validator allows exactly one local
Zenoh transport crate with version `1.10.1`, a fixed relative path
and pinned `lz4_flex 0.11.6`; unrecognized patches and other local
crate sources still fail closed.

## Remaining blockers

This backport targets **only** RUSTSEC-2026-0041. Even after successful
migration, remaining warnings for `paste`, `rustls-pemfile` and
`unsafe-libyaml` continue to block `cargo-deny` and signed release
artifacts. They require separate reviewed upstream replacements rather
than blanket allowlists or pretending that a scanner failure is success.

Perform independent TLS/mTLS handshake, multi-host cluster failure,
backup/restore, upstream code review, artifact provenance and
operational signoff before approving any production deployment.
