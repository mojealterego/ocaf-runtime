# OCAF priority 0.32 — release integrity and provenance acceptance

Date: 2026-10-13. **No automatic tag, GitHub Release, or production deployment.**

## Evidence established at priority 0.31

Commit `77ddc0c9046faece6781a294dd59df7febe02feb` on `main` has
successful Rust CI (run `38373890158`) and supply-chain CI (run
`38373889928`). In the latter run the dependency-policy and attested-build
jobs completed successfully; artifact
`ocaf-linux-x86_64-77ddc0c9046faece6781a294dd59df7febe02feb`
(ID `11800825232`) was uploaded. The build log records subject
`ocaf_runtime-linux-x86_64.tar.gz` with SHA-256
`717edca0797e219d5ff20c46407abc3fad0a59598c0542c30b6e8302da5baed7`.
GitHub Actions reports successful provenance attestation `55347315`
and SBOM attestation `55347321`. These are records of successful signing,
**not yet independent consumer-side signature verification**.

## 0.32 controlled acceptance

For each `main` push, the downstream `release-evidence` job may run
**only after** dependency-policy and attested-build have succeeded. It
downloads the immutable artifact from the **same workflow run** using the
exact SHA-qualified artifact name.

The local release validator rejects:
- wrong or extra checksum entries, mismatched SHA-256 digests, truncated files,
  symlinks for evidence inputs and unbounded input sizes;
- SBOM files that are not CycloneDX 1.5 JSON, lack root `ocaf_runtime`
  metadata or contain no valid components;
- unsafe or unexpected TAR members, archive symlinks, missing executables,
  non-executable binaries or oversized members.

The verifier intentionally does **not** extract archive contents.

Signature verification is separate: GitHub CLI must verify both
`https://slsa.dev/provenance/v1` and `https://cyclonedx.org/bom`
attestations for the same archive, using the expected repository, signer
workflow, `refs/heads/main` and the **exact commit SHA**. The pipeline
fails closed when either check fails. No reusable personal access token
is stored; the job uses the ephemeral `GITHUB_TOKEN` for API access.

To reproduce locally after downloading and unpacking the CI artifact
into a trusted directory:

```bash
python3 -m unittest discover -s scripts/tests -p test_release_bundle.py -v
python3 scripts/verify_release_bundle.py --dir dist
(cd dist && sha256sum -c SHA256SUMS)
gh attestation verify dist/ocaf_runtime-linux-x86_64.tar.gz \
  --repo mojealterego/ocaf-runtime \
  --source-ref refs/heads/main \
  --source-digest 77ddc0c9046faece6781a294dd59df7febe02feb \
  --signer-workflow mojealterego/ocaf-runtime/.github/workflows/supply-chain.yml
gh attestation verify dist/ocaf_runtime-linux-x86_64.tar.gz \
  --repo mojealterego/ocaf-runtime \
  --predicate-type https://cyclonedx.org/bom \
  --source-ref refs/heads/main \
  --source-digest 77ddc0c9046faece6781a294dd59df7febe02feb \
  --signer-workflow mojealterego/ocaf-runtime/.github/workflows/supply-chain.yml
```

Replace the pinned SHA above with the commit from the **downloaded**
workflow run when checking a later candidate. Do not use current HEAD
to verify an older binary.

## Gate separation

Passing Rust CI, dependency policy, checksum/structure verification and
both cryptographic attestation checks establishes *release evidence*, not
production readiness or a reproducible bit-for-bit build. An operator
must separately approve the reviewed exact commit and binary digests
before any manual release promotion. Real multi-host failover, PKI
rollover, restore rehearsal, hardened tool sandboxing, soak tests and
independent security assessment remain required.

**Production verdict: NO-GO** until external acceptance evidence is supplied.
