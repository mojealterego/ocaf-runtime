# OCAF Stage 0.44 — deployment and acceptance

Status: source in main, **NOT PRODUCTION ACCEPTED**.

## Identity and consent
Use Keycloak as a real OIDC authorization server. It owns the browser login, MFA, consent screen, authorization-code and S256 PKCE exchange. The included realm template disables implicit/password grants, requires S256 and consent, restricts redirect URI, and maps the OMAt audience into access tokens. Both .invalid hosts must be replaced with exact operator-owned HTTPS origins. Review realm import with the deployed Keycloak version and configure LoA/ACR for real MFA. Never commit passwords or token values.

The Rust ocaf-oauth-gateway is a protected-resource authorization sidecar, not an authorization server. It binds only to 127.0.0.1. A separate hardened public TLS 1.3 reverse proxy must forward the original bearer token to the sidecar and require its HTTP 204 response before any OMAt dispatch. The OMAt mTLS/policy check remains independently mandatory. Do not expose OMAt or this plaintext sidecar directly.

## Runtime configuration
Required: OCAF_OIDC_ISSUER, OCAF_OIDC_CLIENT_ID, OCAF_OIDC_TENANT, OCAF_OIDC_KEY_ID, OCAF_OIDC_PUBLIC_KEY_FILE, OCAF_OIDC_MFA_ACR, OCAF_OIDC_RESOURCE, OCAF_OIDC_REQUIRED_SCOPES.
Optional: OCAF_OAUTH_BIND (IPv4 loopback only; default 127.0.0.1:8765).
The public PEM key must match a verified signing key from the configured Keycloak JWKS and pinned kid. Automating JWKS refresh with overlap is still open. Do not trust JWT-supplied key URLs or identity headers.

The sidecar implements /.well-known/oauth-protected-resource, /healthz and POST /omat/{tenant}/users/{user}/authorize. It only checks access and never executes OMAt. There is no deployed reverse proxy, consent callback or public login route in this repository.

## KMS and HSM
vault-policy.hcl is a least-privilege Vault Transit encrypt/decrypt/rewrap ACL template, NOT an active Vault integration. Provision a non-exportable derived AES-256-GCM key and operator-separated key rotation. The current Rust SecretBroker encrypts with a local key, not Vault. HSM-backed keys require a verified PKCS#11/managed-key deployment, supported edition, tested key lifecycle, attestations and HA. Neither Vault availability nor HSM integration has been verified.

## Real-network acceptance
On the target host, provision OCAF_E2E_ISSUER, OCAF_E2E_GATEWAY, OCAF_E2E_TENANT, OCAF_E2E_USER, OCAF_E2E_RESOURCE, and OCAF_E2E_TOKEN_FILE (owner-only, 0600). Run:
    python3 scripts/stage044_network_acceptance.py
This probes TLS 1.3, issuer metadata, protected resource metadata, positive authorization, wrong tenant/user, malformed bearer, and 48 concurrent negative requests. It fails if configuration is missing. It does not substitute for live browser PKCE+consent+MFA, external provider refresh, Vault KMS/HSM, certificate rotation, chaos/network partitions, or hybrid PQC downgrade tests. Those remain blocking acceptance criteria.

No production GO until all these tests run and are recorded on the actual infrastructure.
