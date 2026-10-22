# SBOM, provenance and license gate

Stage 0.42 now has a deterministic generator at
`scripts/generate_sbom.py`. It reads the committed `Cargo.lock`, emits a
CycloneDX 1.5 JSON document, includes each locked package checksum when Cargo
provides one, and binds the document to the Git commit SHA.

```sh
python3 scripts/generate_sbom.py --output dist/ocaf-runtime.sbom.json
python3 scripts/verify_sbom_provenance.py dist/ocaf-runtime.sbom.json \
  --expected-sha "$(git rev-parse HEAD)"
```

The generator deliberately emits `NOASSERTION` for every license. A lockfile
does not prove license compliance. The verifier therefore reports
`license_review: OPEN`; a release cannot claim license acceptance until a
separate review supplies SPDX identifiers and its evidence.

This is an implemented, testable gate. It is not a production certification,
an external audit, or proof that Rust binaries were built.
