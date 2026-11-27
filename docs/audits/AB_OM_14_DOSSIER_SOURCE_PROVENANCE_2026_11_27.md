# AB-OM-14 — source dossier provenance and canonical selection

Date: 2026-11-27. Base: `a92a67f067b61bd3464a08f45fef90fec5e5af5e`. Status: TESTED_LOCAL_PARTIAL; requirement OPEN_UNVERIFIED.

Four original source/test files reconstructed in an isolated Linux directory and checked against Git blob SHA:
- agent-catalog-ui.mjs dc17f3c49c395adb947af2df2c4359dc249d48bf
- agent-catalog.mjs 25874cd2c8a85a03fcfed7d13a8f45d1575c95df
- strict-json.mjs 36a00ba9872cdc4d527a1a59c433af83959c92f6
- tests/agent-catalog-ui.test.mjs 05907f718a3e939528b0cfb60eadeb2371ab5ede

Change: select the canonical record from the admitted catalog by ID; ignore caller-supplied record claims; clear dossier and reject unknown ID. Show fleet, schema version (not agent version), declared source Git blobs, explicit lack of cryptographic authenticity verification, reference swarm scope/membership and artifact status for individual agents. No admission, execution or commercial authority.

Node v22.16.0. Command in apps/agents-builder-web: node --test tests/agent-catalog-ui.test.mjs.
RED: 3 PASS / 2 FAIL, exit 1; forged name accepted and string ID failed to resolve.
GREEN: 5 PASS / 0 FAIL, exit 0; all 144 synthetic roster dossiers exercised, provenance, scope, membership, invalid ID and forged fields checked. node --check agent-catalog-ui.mjs exit 0.
Verified modified blobs:
- UI 781ffa78c53276bc9fd05812c919e515a3ee4577
- tests deef9a0f45a3f065f566559b934e49fe4d460bb5

TAP SHA-256 RED 8b366c2b10d67bd52109899b7854402091419ab2452cbc82edd55357ac0eac90; GREEN a55ea95856234398180abf5a9b1f135c3e86da0bf7a855581ed996f1421512d6. Summary; logs not embedded.

Test scope: native Node Response, injected transport and DOM test doubles, not a real browser. Full project regression, Rust, and platform builds not executed. Actual documentation publication, agent version and producer attestations, signed certificates and revocation, prices, resource and compatibility data remain open. No release or production approval. No external audit required.
