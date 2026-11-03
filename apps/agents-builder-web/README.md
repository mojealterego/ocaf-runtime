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

## Multilingual · API credentials · GGUF model profiles (2026-11-03)

The existing single Local/Cloud Code/NoCode UI now includes:

- Native UI catalog: `pl-PL` fallback, `en-US`, `cs-CZ`, `sk-SK`,
  `de-DE`, `uk-UA`, `ar-SA`. Direction is LTR/RTL; `Code DSL`
  remains LTR. Additional BCP 47 translations can be imported from a user
  catalog JSON (`{"locale":"it-IT","messages":{"title":"Crea agenti."}}`).
  Missing translations fall back to Polish; this is **not** a claim of
  completed translations for every language or all legacy status messages.
- Session-only credentials for OpenAI, Anthropic, Gemini, OpenRouter, Groq,
  Mistral, DeepSeek, Cohere, Hugging Face. The secrets are held only in
  JavaScript memory; they are cleared from the input immediately, never sent
  with exported DSL, never stored in localStorage/SQLite, and not uploaded to
  a server. A **production** credential vault or delegated OAuth/Keychain
  broker still requires a separate, tenant-authenticated implementation.
  Merely entering a key does not trigger provider calls, charges or inference.
- Public Hugging Face model lookup from `/api/models/{repo}?files_metadata=true`,
  filtering GGUF filenames and sizes. Download links target an exact verified
  repository SHA instead of mutable `main`. Downloads happen only following
  a direct operator click and are handled by the browser. Gated/private model
  access is not bypassed, and model rights and licenses must be respected.
- Local GGUF file picker with bounded binary-header validation (magic,
  GGUF version 2/3, tensor/metadata counts). Only a small header is read;
  no model weights are cached by the UI. Large tokenizer metadata may
  be reported as partial. The original file is **not** retained after
  closing the tab, so it must be reselected for subsequent execution.
- Versioned, bounded inference profile: context size, CPU threads,
  batch size, GPU layers, temperature, top-p, top-k, min-p, repetition,
  presence/frequency penalty, seed, maximum output, flash attention, mmap.
  Profiles are stored locally under `tenant/project_id` **outside** the
  typed Rust JSON DSL schema and are exportable without API secrets.
  They are not automatically synchronized with Cloud workspaces.
- These are configuration and download/inspection features. **No actual
  GGUF inference**, remote provider invocation, API-key server vault,
  multi-device synchronization, or production model load is claimed.
  A compatible inference adapter must be integrated and independently tested
  before advertising an operational agent.

Verification:
```bash
node --test apps/agents-builder-web/tests/*.test.mjs
node apps/agents-builder-web/tests/browser-smoke.mjs
```
