# Stage 0.52 — executable Builder Cloud stack

**Scope:** shared hosted project workspace for all eight platform targets, not a ninth app.
**Base source revision:** cec25c8dbc1f4c30d9559a4ad1a6c7d773f4c0d2 (main).
**Evidence state:** IMPLEMENTED / TESTED / SCOPED NETWORK INTEGRATION VERIFIED.
No production approval, native-platform acceptance or independent release-auditor certificate.

## Architecture

HTTPS/mTLS (Python ingress) → same-UID private Unix socket → compiled Rust bridge →
existing BuilderHttpBoundary → OAuth scope/tenant/user authorization → compiled DAG
preview or SQLite project CAS/storage. The supervisor probes the framed protocol,
then starts TLS; failure of the bridge stops the listener. SIGTERM stops and reaps
the child. The existing editor is served at the same HTTPS origin.

API: GET /v1/projects/{id}, PUT /v1/projects/{id}, GET history, POST preview.
An allowlisted client-certificate DER SHA-256 fingerprint selects the tenant/user.
The opaque OAuth token is independently validated by Rust. The private UID and
canonical private directories are mandatory trust boundaries. Never run arbitrary
agent tools under that UID. The service does not issue OAuth tokens, claim MFA,
execute agent tools, publish agents or auto-deploy. Existing externally obtained
OAuth access tokens and operator-provisioned certificates are required.

Ingress performs bounded, exact HTTP framing and route/Host/Origin checks; no
cookies, forwarded or spoofed identity headers, Transfer-Encoding, duplicate
headers or arbitrary file paths. It enforces a 32-worker cap and socket deadlines.
The bridge limits frame/body size and concurrency, checks peer UID and subject
freshness. Both launch paths enforce private regular files for TLS keys and
identity registry; no symlink, hard link or public permissions. Stale sockets
are never automatically removed at startup. Trust material is fixed for the life
of the process; certificate revocation/rotation needs an operator-managed restart.

## Build and run locally (Linux/POSIX)

The repository pin is Rust 1.99.0. Use a dedicated service account and a trusted
non-root environment. No Actions, external paid runner or new service is needed.

    bash scripts/verify_builder_cloud_stack.sh "$(git rev-parse HEAD)"
    cargo build --locked --no-default-features --bin ocaf-builder-cloud-bridge

Create a canonical absolute directory accessible only by the service UID (0700),
two distinct regular SQLite files (0600), and private bridge.json (0600).
Use existing OAuth data without overwriting or generating fake credentials.
Example configuration shape (substitute the operator's actual paths/audience):

    {
      "origin":"https://builder.example:8443",
      "audience":"https://builder.example:8443",
      "socket":"/srv/ocaf-builder/private/bridge.sock",
      "oauth_database":"/srv/ocaf-builder/private/oauth.sqlite",
      "builder_database":"/srv/ocaf-builder/private/projects.sqlite",
      "clients":[{
        "client_id":"builder-local","tenant":"local",
        "redirect_uri":"https://client.example/callback",
        "resource":"https://builder.example:8443",
        "scopes":["builder:read","builder:write"],"step_up_scopes":[]
      }]
    }

Supply an operator-managed CA and TLS key/certificate, with an identity registry
mapping certified client SHA-256 DER fingerprints to {"tenant":"local","user":"alice"}.
The TLS key and registry must be private regular files in canonical private
directories; all config is operator controlled. Get the certificate fingerprint
with openssl x509 -in client.crt -outform DER | sha256sum.

    python3 -m scripts.builder_cloud_supervisor \
      --bridge-bin "$(realpath target/debug/ocaf-builder-cloud-bridge)" \
      --bridge-config /srv/ocaf-builder/private/bridge.json \
      --origin https://builder.example:8443 --bind 127.0.0.1 --port 8443 \
      --cert /srv/ocaf-builder/private/server.crt \
      --key /srv/ocaf-builder/private/server.key \
      --ca /srv/ocaf-builder/private/client-ca.crt \
      --registry /srv/ocaf-builder/private/identities.json \
      --assets apps/agents-builder-web

The origin must match browser URL/Host and deployment certificate. Exposing
the service beyond loopback is a separate reviewed operator deployment decision.
Do not expose the private bridge socket. The service must use a distinct UID from
agent sandboxes. No test CA, test users or test token should be used in production.

## Executed evidence, 2026-11-18

Earlier Python implementation packages were re-examined from scratch. Their source
was imported into a fresh checkout of the pinned main revision on existing
ocaf-stage038-host-b (Linux, x86_64), and new tests were executed there.
Python tests also passed in the local sandbox before the security fixes.

Final verification command:
    bash scripts/verify_builder_cloud_stack.sh cec25c8dbc1f4c30d9559a4ad1a6c7d773f4c0d2

The command was run against the candidate source tree at the stated base
revision. Source hashes below bind the working-tree inputs; the repository
commit containing this report binds all files and permits independent retesting.

- Rust 1.99.0 (b940084d7); Cargo 1.99.0; Python 3.13.7; Node 24.18.0;
  OpenSSL 3.5.5. Locked dependencies; Cargo.toml/Cargo.lock unchanged.
- Real Rust binary build: PASS. SHA-256 (debug binary in this environment):
  9f6676326939d3033d1b54b73e3367ddf7b94a04cd19479cfd299b5ec618ab2b.
- 37/37 Rust tests: bridge 2; Builder Cloud 14; full stack 1;
  core 13; store 6; web contract 1.
- Full stack mtls_oauth_sqlite_cas_and_restart_full_stack uses actual signed
  TLS client certs, PKCE-generated opaque tokens, compiled bridge process,
  real loopback sockets, real SQLite, preview, save, conflict (409), read-only
  denial, wrong subject/tenant/token/certificate denial, history, supervisor
  shutdown and a restart preserving two revisions. Test fixture PKI and
  tokens are temporary and never committed. **Not** browser/device GUI E2E.
- 22/22 Python ingress and supervisor tests; recording or executable fixture
  backend for these focused tests, separate from the actual Rust full-stack test.
- RED for TLS key and standalone registry permissions: 1 failed assertion
  + 1 timeout because insecure config was accepted; GREEN 22/22 after fix.
- Source ZIP coverage RED 17 failed subtests; GREEN 3/3 with 18 independent
  anchor omission cases and byte-identical fixture ZIP reproduction.
  Existing packager suite 16/16.
- JavaScript shared-editor tests: 65/65, no skip, no failures.
- rustfmt on both new Rust files: PASS; Bash syntax: PASS; git diff --check: PASS.
- Clippy scoped diagnostic count: 0 in new Rust source. Strict repo-wide
  Clippy -D warnings failed on 7 pre-existing library diagnostics
  (hybrid_retrieval.rs, trs_traffic.rs, trs_dtn.rs, trs_dtn_reliable.rs,
  trs_dtn_sqlite.rs). No suppressions or modifications to those files.
- No new paid resources or GitHub Actions invoked.

Source file SHA-256:
```
f4f331c12a1b93c162ac46c69ee6a8328d764d871495877c7922f79a60601fd6  scripts/builder_cloud_ingress.py
9d46264bc94271324de6ce457182b168aa1af2a6977b06e3b16863c27495c41d  scripts/builder_cloud_supervisor.py
5603f29e69e47b9c6b709d4a47bda7bca7e287edd78cc3f2727ca66ecbe6a2f7  scripts/tests/test_builder_cloud_ingress.py
c8a9daa1ce87cc9fc62c639ab0f884b3e03c1cd78e7b32fffd491c8a39f868dc  scripts/tests/test_builder_cloud_supervisor.py
cd58f91503274e7a65d601405750e6c0c0a39fa11befc77a3ab8c1ddce10cc02  src/bin/ocaf-builder-cloud-bridge.rs
1eda0d14c2f3edf9b8fadabd327782b220d15bdbd9330e60dc8750cc4e9eab58  tests/builder_cloud_full_stack.rs
eb9ddf265d20cef40e339b7e1c1a9579d34413b3b11f86b2a8fdc647a5fdbdb3  scripts/verify_builder_cloud_stack.sh
5a7ef3138317d28b8be340372823659beaa1dda55192736abe7085c109962dec  scripts/package_agents_builder.py
0e4e63aa7410971a6770335ab8751e294e23a03eeca981e27b22dc236255a258  scripts/tests/test_builder_bundle_platform_coverage.py
```

## Boundaries / gate disposition

This is independently repeated execution of the prior source in an isolated
environment, plus separate negative tests and code review. It is not an
independent human/agent release audit; GATE-08 remains open. Global lint,
browser mTLS navigation (local browser administrative policy blocked the
attempt), native client certificate/token acquisition on all eight platforms,
production deployment, full release signing, native GUI E2E, 32 platform
suites and GATE-10 are not complete. No product registry status is promoted.
Cloud execution/build/deployment and interactive consent/PKCE UI remain outside
this increment. All original roadmap requirements remain in force.
