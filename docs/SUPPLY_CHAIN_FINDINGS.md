# Security inventory — 2026-10-13 (supply-chain priority 0.28)

**Decision: production release NO-GO.** The first actual `cargo-deny`
GitHub Actions scan, run ID `38364859758`, found the findings below.
The scanner passed its **bans, licenses and sources** checks but failed
its **advisories** check. This document records observed results, not
an independent exploitability assessment of OCAF's deployed configuration.

| RustSec advisory | Dependency in observed graph | Severity category | Required action |
| --- | --- | --- | --- |
| RUSTSEC-2026-0041 | `lz4_flex 0.10.0`, via `zenoh-transport` | Vulnerability: decompression and information disclosure | Review Zenoh dependency upgrade to patched `lz4_flex`; the patched versions are in different 0.x minor lines and require an upstream dependency change. |
| RUSTSEC-2024-0436 | `paste 1.0.15`, via `token-cell` | Unmaintained | Track upstream replacement/removal and review maintained alternatives. |
| RUSTSEC-2023-0071 | `rsa 0.9.10`, previously via `zenoh-transport` | Vulnerability: Marvin timing side channel | **Mitigated at dependency-graph level** by disabling unused Zenoh auth_pubkey/default features. Revalidate after every feature or lockfile update. |
| RUSTSEC-2025-0134 | `rustls-pemfile 2.2.0`, via `zenoh-link-tls` | Unmaintained | Track upstream migration to `rustls-pki-types` parsing APIs. |
| RUSTSEC-2024-0449 | `unsafe-libyaml 0.2.11`, via `serde_yaml` / `zenoh-config` | Unmaintained | Coordinate replacement of YAML parser upstream; do not silently ignore. |

Changes introduced after the initial scan: `Cargo.toml` explicitly
uses `zenoh` with `default-features = false` and only `transport_tcp`
and `transport_tls`. Cargo-generated `Cargo.lock` shrank from **480**
to **432** package entries and no longer contains `rsa`. This is a
source-based dependency observation, not proof of runtime behavior.

### Evidence and review

- Baseline scanner result: GitHub Actions run `38364859758` on commit
  `055b09c08c6b079e55274c7dec7850845803b55e`.
- Bootstrap lockfile run `38364150506`, later transport minimization run
  `38365099441`.
- Current `main` must run `cargo test --locked` and the current supply
  chain policy on its own commit before accepting new evidence.
- A vulnerability scanner operating on the lockfile does not establish
  that a vulnerable function is reachable in the application. Equally,
  disabling a feature without rebuilding/retesting is not a deployment
  mitigation claim. Keep advisories fail-closed pending actual review.

## Priority 0.29 — enforce disabled compression while upstream remediation is pending

Zenoh `1.10.1` still locks `lz4_flex 0.10.0` through the
`zenoh-transport` dependency, even with `transport_compression` disabled.
The upstream dependency cannot be safely forced to patched
`lz4_flex >=0.11.6,<0.12.0` without migrating the Zenoh integration,
using a reviewed upstream patch, or maintaining a vetted security backport.
See [RustSec 2026-0041](https://rustsec.org/advisories/RUSTSEC-2026-0041.html).

Implemented safety gates in 0.29:
- A fail-closed policy in `validate_zenoh_config` denies
  `transport/{unicast,multicast}/compression/enabled` unless explicitly
  `false`; missing compression settings retain Zenoh's disabled default.
- An independently tested `cargo metadata --locked --all-features` guard
  checks both `zenoh` and `zenoh-transport` to reject enabled
  compression, public-key/password authentication, multilink or
  unreviewed transport features. It requires both TCP and TLS.
- Both standard Rust CI and supply-chain CI now enforce that graph
  before any production-facing build. Compilation cannot silently
  re-enable compression through Cargo feature unification without
  failing that guard.
- No advisory ignores, disabled scanners or deceptive substitutions
  were added. Existing unmaintained packages still require upstream
  migration and independent review.

This is a **defense-in-depth reachability mitigation**, not removal
of the vulnerable `lz4_flex` package and not a replacement for a
fixed upstream dependency. The `cargo-deny` advisory gate deliberately
remains NO-GO until dependencies are remediated.

**Production consequences:** the attested artifact job depends on the
advisory/license/source gate. Therefore no new attested release artifact
will be produced until the currently blocked security gate is resolved.
Do not insert blanket RustSec ignores merely to turn CI green.
