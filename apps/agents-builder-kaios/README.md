# OCAF Agents Builder — KaiOS 2.5

Status: **source implementation committed; packaging CI and real device/simulator execution unverified**.

A side-effect-free canonical typed DAG validator is included in `canonical-graph.js` and tested separately. The v1 editor still uses its restricted node-only schema; canonical project editing and execution are not integrated.\n\nThis is a small, keypad-friendly, offline **subset** of OCAF Agents Builder, not a full agent compiler or runtime. It targets the KaiOS 2.5 packaged web-app model (`manifest.webapp`). KaiOS 3.x compatibility has not been established and must be tested separately.

## Implemented

- Code: edit a constrained JSON project (version 1, project_id, up to 32 unique node IDs).
- NoCode: add and list typed-identifier nodes; the same JSON document is displayed in Code mode.
- Local: persist and reopen validated projects in browser localStorage.
- Cloud: **disabled and fail-closed**. There is no authentication, API key handling, model inference, GGUF, tool execution, cross-device synchronization, or deployment. A separate static typed DAG validator is bundled, but it is not yet wired into the v1 node-only editor and is not a signed compiler.
- Keypad shortcuts: SoftLeft save, SoftRight open; controls are standard focusable HTML elements.

## Limitations and security

The JSON validator rejects unsupported project/node properties rather than silently stripping unknown graph semantics. This is a deliberately strict schema boundary.

The simplified KaiOS schema is **not** the canonical OCAF typed DAG and is not interoperable with the desktop project's complete schema. The UI must not claim semantic Code/NoCode round-trip with the canonical compiler. The app requests no privileged KaiOS permissions and does not evaluate project JSON as code. Local storage is not encrypted; do not enter secrets.

## Build and acceptance

`.github/workflows/agents-builder-kaios.yml` runs source contract tests and JavaScript syntax checks and creates a ZIP of `manifest.webapp`, `index.html`, `app.js`, `style.css`, `canonical-graph.js`. This is an **unsigned web-app package**, not proof of installation.

Acceptance still requires: pinned KaiOS SDK/device target, install and launch, keypad navigation E2E, persistence after restart, storage quota handling, accessibility, and independent negative tests. For KaiOS 3.x, separately verify manifest and runtime requirements before claiming compatibility.
