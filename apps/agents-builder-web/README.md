# OCAF Agents Builder — browser Code/NoCode (stage 0.51 partial)

This is a working, dependency-free **offline web editor**, not a cloud service
or native Windows/macOS/Android/iOS application. It edits the restricted JSON
typed DAG DSL from Rust src/builder_core.rs. Code and NoCode are two views
of the same project, not arbitrary Rust-to-visual conversion.

## Run locally

From the repository root:

    python3 -m http.server 8765 --bind 127.0.0.1 --directory apps/agents-builder-web

Open http://127.0.0.1:8765/ . Browser service workers cache static editor
assets for offline reload after first successful load. Use HTTPS or localhost
for PWA installation. Browser localStorage persists at most 32 snapshots per
project, with optimistic revision checks; export JSON to keep an external copy.

## Features

- Code editor with strict JSON import, typed schema validation and apply.
- Interactive NoCode canvas, edge selection, palette drag/drop (or tap), typed
  Transform/Tool insertion, node removal and explicit tool grant/budget edits.
- Versioned browser-local projects, save/reopen, history and non-destructive revert.
- Static dry-run plan; **no execution**, network access, publication or signing.
- Bounded project/DSL sizes, rejected unsupported schema and arbitrary code.
- Shared Rust fixture checked in tests/builder_web_contract.rs.

## Security and limitations

The browser validator is UX-only; it does not authenticate a tenant. Tenant
local is an offline namespace, not a trusted identity. The origin's
localStorage is not transactional across tabs, cannot replace a server-side
durable database and is not tamper-resistant. Imported JSON is validated and
existing IDs cannot be silently overwritten. Exported JSON is unsigned.

Do not expose this static editor as a cloud multi-tenant service or call tool
APIs directly from it. Production Cloud needs authenticated ingress, Rust
compiler validation, durable tenant-scoped storage, sandbox, audit receipts,
signed artifacts and explicit user-approved deployment. Native desktop and
mobile targets still require independent platform projects, build artifacts,
GUI E2E and security acceptance.
