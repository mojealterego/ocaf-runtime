# OCAF Agents Builder — shared Code/NoCode UI (stages 0.51–0.52 partial)

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

## Wspólny interfejs Local / Cloud (2026-11-03)

Ten sam edytor Code/NoCode ma dwa jawne tryby przechowywania projektu:
**Lokalnie** (offline localStorage) oraz **Chmura** (same-origin OAuth Bearer).
Przełącznik nie kopiuje projektu automatycznie między tenantami ani nie
wysyła danych bez wyraźnego polecenia operatora.

Tryb Cloud korzysta z istniejącego kontraktu Rust `BuilderHttpBoundary`:
`GET /v1/projects/{id}`, `PUT /v1/projects/{id}` (CAS `expected`)
i `POST /v1/projects/preview`. Wymaga rzeczywistego gateway HTTPS
uwierzytelniającego `AuthenticatedSubject`, aktywnego OAuth i SQLite.
Dopóki gateway nie jest wdrożony pod tym samym origin co aplikacja,
żądania Cloud zakończą się błędem i **nie wolno deklarować wdrożonej chmury**.
Klient nie ufa nagłówkom `x-tenant` ani `x-role`, nie używa cookies,
nie zapisuje tokenu OAuth na dysku, nie wykonuje narzędzi, nie publikuje
agentów i nie ponawia automatycznie zapisu po konflikcie HTTP 409.
Tenant wpisany w UI jest tylko oczekiwaną tożsamością; wiązanie
token–tenant–user–scope musi wykonać gateway i usługa Rust.

```sh
node --test apps/agents-builder-web/tests/*.test.mjs
CHROME_BIN=chromium node apps/agents-builder-web/tests/browser-smoke.mjs
```

Wariant Android może na tym etapie korzystać z responsywnego PWA w
przeglądarce; nie jest to natywny APK. Binarny Meta Agent Architekt,
natywne buildy Windows/macOS/iOS oraz produkcyjna chmura nie są
objęte tym przyrostem. Macierz 10/10 pozostaje otwarta.
