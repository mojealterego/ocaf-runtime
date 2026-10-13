# OCAF 0.28 — Locked dependencies, security policy and build evidence

**Status: supply-chain controls implemented; production signoff remains NO-GO.**

## Immutable dependency inventory

`Cargo.lock` is committed on `main` and no longer gitignored. It was generated
by Cargo in a short-lived, path-scoped bootstrap workflow on `main`; that
bootstrap is removed after completion. No package version or checksum was
manually fabricated. `scripts/supply_chain.py` validates the v4 lockfile
as an offline inventory: package identities, root package manifest identity,
checksums for crates.io registry entries, duplicate entries, missing
sources, Git dependencies, lockfile symlinks and unexpectedly large inputs.
It emits SHA-256 of the actual lockfile, not a declaration of package trust.

Run:
```bash
python3 -m unittest discover -s scripts/tests -p test_supply_chain.py -v
python3 scripts/supply_chain.py --repo .
cargo metadata --locked --format-version 1 --no-deps
cargo test --locked
```

Passing this validator **does not** verify crate archive bytes independently,
license compliance, source authenticity or vulnerability absence.

## CI: dependency policy

`.github/workflows/supply-chain.yml` performs:
1. Offline integrity tests and manifest-vs-lock check.
2. `cargo metadata --locked` so Cargo cannot silently rewrite its dependency resolution.
3. A pinned `cargo-deny` GitHub Action against the whole feature graph,
   including RustSec advisories, license allowlisting, dependency bans and
   crates.io source restrictions. `deny.toml` currently includes no
   advisory exemptions. Each failed check blocks the dependent build.

The registry allowlist covers crates.io only. Existing dependencies may
require update or explicit, individually justified license decisions to
pass the policy. Do not bypass findings by disabling CI or adding blanket
advisory ignores. A successful CI scan represents a point-in-time result.

## Artifacts and provenance

On a `main` push that passes dependency policy, the second CI job builds
release binaries from `Cargo.lock`, creates a CycloneDX 1.5 SBOM for all
targets, publishes an archived Linux x86_64 artifact plus SHA-256 sums,
and requests GitHub/Sigstore attestations for both build provenance and
the associated SBOM. Tags and actual GitHub Releases are **not**
created by this workflow. Downloadable CI artifacts are not a
production release and attestations do not prove runtime security.

Operator check following a successful attested CI run:
```bash
sha256sum -c SHA256SUMS
gh attestation verify ocaf_runtime-linux-x86_64.tar.gz --repo mojealterego/ocaf-runtime
```
Run checksum verification inside the extracted CI artifact directory. Verify
attestation subject digest, workflow identity, `main` commit SHA and the
reviewed Rust toolchain. Keep SBOM and attestations alongside the
approved release before deployment.

## Remaining production gates

The following remain external prerequisites: independent multi-host
PostgreSQL failover/fencing, real mTLS credential rollover, upstream
vulnerability remediation and license review, tool sandboxing, restore
rehearsal, independent security assessment and SLO/soak results.
A green GitHub Actions status alone is **not** production acceptance.
