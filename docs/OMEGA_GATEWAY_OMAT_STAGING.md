# OMEGA Gateway OMAs/OMAt contract (staged)
The root crate exposes omega_gateway_omat, a typed local-only tool manifest,
resource digest, request builder and signed Zenoh staging probe using existing
stage043 functions. Registration defaults disabled and discovery excludes it.
The existing OSTc gate is not weakened. The staging probe deliberately carries
ostc_enabled=false, and MUST be rejected by the real server; it is not an
executor or a working inference transport. An authorized executor, trusted
OSTc admission and authenticated tenant routing must be implemented/verified
before changing this state. Existing FinOpsLedger, model_router and secret
broker remain authoritative for tenant/cloud integration.
Scope tests: cargo test --locked --test omega_gateway_omat
Status: source written, native build and tests unverified. No release approval.
