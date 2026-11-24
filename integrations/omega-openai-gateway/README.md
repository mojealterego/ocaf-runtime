# OMEGA OpenAI Gateway Ω — 0.2.0

Status: source implementation / build and tests UNVERIFIED. Single trusted
operator, loopback only. Not an OCAF tenant ingress, OMAt tool, or production
service. Existing OCAF model_router, FinOpsLedger and secret broker remain
authoritative for future multi-tenant OCAF admission; this transport adapter
does not replace them.

## Run
```sh
cd integrations/omega-openai-gateway
cargo generate-lockfile
cargo test --locked
cargo build --locked --release
export OMEGA_GATEWAY_TOKEN="$(openssl rand -hex 32)"
export OMEGA_LOCAL_MODEL=local
cargo run --locked --release
```
Local llama-server must listen at 127.0.0.1:8082 and accept the configured model.
No OpenAI request is enabled by default. In a private terminal only, load an API
key from a secret manager, set OMEGA_CLOUD_DAILY_REQUESTS to an explicitly
approved nonzero quota and OMEGA_QUOTA_DB to an absolute persistent path in a
private directory (mode 0700), then restart. Never commit keys. Tools separately
disabled unless OMEGA_CLOUD_TOOLS=1. No automatic fallback to cloud.

```sh
curl -fsS http://127.0.0.1:8788/v1/generate \
 -H "Authorization: Bearer $OMEGA_GATEWAY_TOKEN" \
 -H "Content-Type: application/json" \
 -d '{"input":"Odpowiedz po polsku","model":"local"}'
```
Cloud: model astra/sol/terra/luna and allow_cloud:true, optional tool docs/web.
Each cloud attempt consumes a persistent daily slot, including failed attempts.
Quota is NOT a hard USD budget; provider pricing, reasoning, tool fees and other
clients may incur charges. No automatic retries. Stream:true returns native SSE:
OpenAI Responses events or llama-server Chat Completions chunks, distinguished by
X-Omega-Provider. Stream clients must verify terminal event and reject incomplete
streams. Nonstream returns native JSON; endpoint is not a drop-in OpenAI SDK base
URL. Input 16 KiB, HTTP request 64 KiB, output 16–4096 tokens, response 2 MiB,
four concurrent requests, 120s upstream deadline. Fixed upstream URLs,
no redirects or proxy, no caller-supplied tool schemas/URLs. Docs MCP only
search_openai_docs; web search separately operator-enabled through tool flag.
No prompt or API-key logging. TLS for cloud; local loopback HTTP is single-user.

## Multi-turn and content handling
Optional history contains up to 32 alternating user/assistant messages, ending
in an assistant message. The current input remains the next user message.
The aggregate text limit is 16 KiB. Original message text (including spaces,
Unicode and line breaks) is preserved; the gateway performs no semantic
rewriting, classification or content moderation of its own. Local model
behaviour depends on the installed model; cloud provider policies remain
unchanged. Selecting cloud explicitly authorizes sending the complete supplied
history to that provider. No history is automatically stored by the gateway
or sent to a fallback provider. Authentication, size limits, network and tool
permissions, and quotas remain enforced.

```json
{"model":"local","input":"Dalszy ciąg","history":[{"role":"user","content":"Pytanie"},{"role":"assistant","content":"Odpowiedź"}]}
```

## Integration boundaries
Locally-XXX: Tauri backend invokes gateway; never expose bearer to renderer.
Digital-Life / Omega-X-Evolution: trusted Python workers invoke gateway from
existing provider adapters, interpreting native response shape by provider.
OCAF: bind verified tenant/agent identity, capability policy, existing model
router, FinOps and secret broker before enabling this route for OMAs/OMAt or
Zenoh; never forward arbitrary agent-supplied URLs or bypass OSTc.
Termux: requires Rust toolchain, llama-server and build/runtime verification on
device. No deployment or real API calls performed.

## Verification
cargo test --locked; cargo fmt -- --check; cargo clippy --locked --all-targets
-- -D warnings. The lockfile must be generated and reviewed before release.
Tests cover policy and SQLite quota, not live upstream, native SSE completeness,
tenant isolation, replay across requests, end-to-end integration or supply-chain.
Do not expose outside loopback. No production approval.
