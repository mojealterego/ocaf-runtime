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

## Verified findings and minimized transport surface (2026-10-13)

The initial full-dependency scan reported RustSec advisories in the Zenoh
transitive graph, including the RSA timing side channel. To reduce attack
surface, the project's Zenoh feature selection now disables Zenoh defaults
and allows only `transport_tcp` and `transport_tls`. Cargo re-resolved
and committed the lockfile from this restricted feature configuration on
`main`. This removed the `rsa` package from the locked dependency graph
(480 to 432 locked packages); this alone is not a proof of absence of
other vulnerabilities. The upstream-linked dependencies identified in
[SUPPLY_CHAIN_FINDINGS.md](SUPPLY_CHAIN_FINDINGS.md) remain blockers.

**Resolution (priority 0.30, 2026-10-13):** the five carefully scoped,
Cargo-sourced Zenoh-related local backports removed the four original
RustSec findings without adding exceptions to `deny.toml`. CI job
`38371010671` passed unit tests, selected real Zenoh/mTLS tests,
full Clippy and `cargo-deny check --all-features` before committing
the resulting vendor sources and `Cargo.lock` to `main`.
The current RustSec database must still be checked on **each new commit**.

Unlike a new upstream Zenoh release, these local patches require manual
maintenance, source review and security review; historical audit results
are not claims about new vulnerability discoveries. The advisory gate
remains **enforced**, and generated release artifacts must be
verified independently before use. Do not treat historical CI builds
as signed production releases.

## Remaining production gates

The following remain external prerequisites: independent multi-host
PostgreSQL failover/fencing, real mTLS credential rollover, vendored dependency provenance and ongoing advisory/license review,
tool sandboxing, restore
rehearsal, independent security assessment and SLO/soak results.
A green GitHub Actions status alone is **not** production acceptance.

## Priority 0.31 — SBOM filename regression and provenance recovery (2026-10-13)

The Rust CI and dependency-policy jobs passed at main commit
`0dfa34b8d4f575b1de39cf502290fe63d8b56889`, but the attested-build job
failed at CycloneDX output collection (run `38371980150`, job `115211163066`).
Inspection of the job logs and `cargo-cyclonedx 0.5.9` source identified
a CLI filename mismatch: `--override-filename bom.json` generates
`bom.json.json`, whereas the workflow expects `bom.json`.

The corrected workflow passes `--override-filename bom`, retaining the exact
`bom.json` output path. A dedicated regression test is included in both the
Python CI suite and the dependency-policy job. The SBOM must also parse as a
nonempty CycloneDX 1.5 JSON document before upload or attestation.

**Verification boundary:** this source correction is not proof that attestation
succeeded. Confirm a successful supply-chain run on the resulting commit,
inspect the published artifact, then verify the SHA-256 checksums and both
attestations. Production remains **NO-GO** pending the independent acceptance
gates listed above.

## Priority 0.32 — independently checked release evidence (2026-10-13)

The previously successful 0.31 run `38373889928` published the Linux archive,
CycloneDX SBOM and SHA-256 sums and recorded both signing operations.
Priority 0.32 adds a separate fail-closed consumer-side verification job
to validate the downloaded artifact, archive inventory, CycloneDX structure,
and matching SLSA + CycloneDX signatures for the exact `main` SHA.
See [RELEASE_EVIDENCE_032.md](RELEASE_EVIDENCE_032.md) for evidence IDs,
local verification commands and manual release signoff boundaries.
This is not an automatic release or production certification.
