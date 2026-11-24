# OMEGA OpenAI Gateway Ω — 0.9.0

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
private directory (mode 0700). Also configure OMEGA_CLOUD_DAILY_BUDGET_MICRO_USD\nand OMEGA_CLOUD_RESERVE_MICRO_USD (both positive integer micro-USD units), then restart. Never commit keys. Tools separately
disabled unless OMEGA_CLOUD_TOOLS=1. No automatic fallback to cloud.

```sh
curl -fsS http://127.0.0.1:8788/v1/generate \
 -H "Authorization: Bearer $OMEGA_GATEWAY_TOKEN" \
 -H "Content-Type: application/json" \
 -d '{"input":"Odpowiedz po polsku","model":"local"}'
```
Cloud: model astra/sol/terra/luna and allow_cloud:true, optional tool docs/web.
Each cloud attempt consumes a persistent daily slot, including failed attempts.
Quota and operator-priced reservations are NOT a hard provider USD budget; provider pricing, reasoning, tool fees and other
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

## Local OpenAI-compatible subset
POST /v1/chat/completions accepts model omega-local, messages (optional initial
system message followed by alternating user/assistant, ending in user),
temperature, top_p, seed (0..=2147483647), n=1, stream and max_tokens or max_completion_tokens.
Authentication and all existing resource limits remain mandatory. The alias
always resolves to the operator's configured GGUF model; no cloud or tools are
reachable through this endpoint. Unsupported fields fail closed. This is not a
complete SDK-compatible API (no embeddings, model listing or structured output).
For Omega-X-Evolution the existing dynamic local_openai provider can be
configured with model identifier:
`local/omega-local@http://127.0.0.1:8788/v1?api_key_env=OMEGA_GATEWAY_TOKEN`.
This configuration has not been exercised end-to-end.

## Advanced requests
POST /v1/analyze is cloud-only, requires explicit allow_cloud:true and active
operator budget. Accepts text, an optional inline PNG/JPEG data URL (2 MiB decoded,
magic-byte and base64 validation only), optional previously uploaded file_id from
OMEGA_ALLOWED_FILE_IDS, and tool none/file_search/code_interpreter/sha256.
File search uses only OMEGA_VECTOR_STORE_ID, at most five results. Hosted
Code Interpreter requires OMEGA_CODE_INTERPRETER=1 and can incur extra charges.
No user-controlled URL, upload, arbitrary tool schema or container ID.
The sha256 tool requires OMEGA_LOCAL_FUNCTIONS=1. It advertises one strict
function schema; the trusted client may invoke POST /v1/functions/execute with
{"name":"sha256_text","arguments":"{\"text\":\"abc\"}"} to execute a pure hash.
The nonstreaming analyze route performs at most one automatic SHA-256 call, then
replays its own authenticated output and function_call_output in a second
stateless Responses call with tool_choice:none. Each request consumes an
independent budget reservation; if the second admission fails the operation
fails closed. Streaming SHA-256 orchestration is rejected. No arbitrary tool
names, URLs, file operations or shell execution are accepted.

Budget migration: both per-day call count and operator-priced reservation are
written atomically in SQLite before each cloud attempt. No refunds on errors.
If older unpriced calls exist for the current UTC day, cloud admission fails
closed until the next day. Operator prices are assumptions and do not bound
actual provider invoices; set conservative values and separate provider limits.
File search, images, containers, cached and reasoning tokens can change actual
charges. No cloud calls have been executed by this implementation session.

## Voice (explicit cloud opt-in)
Set OMEGA_CLOUD_VOICE=1 in addition to the cloud budget. POST /v1/audio/transcribe
accepts {"allow_cloud":true,"wav_base64":"..."} and uses gpt-transcribe.
POST /v1/audio/realtime-turn accepts exactly one of text (max 4096 UTF-8 bytes)
or pcm16_base64 (mono 24 kHz PCM16 LE, max 2 MiB) and allow_cloud:true.
It establishes a server-side WebSocket to gpt-realtime-2.1-mini, disables tools
and VAD, sends a single turn, collects bounded PCM and requires response.done
with completed status before returning JSON. A session is capped at 90 seconds,
1024 output tokens and 4 MiB audio. Output is AI-generated speech; disclose this
to listeners. This is not a persistent duplex WebRTC conversation. No API secret
is returned to clients. All audio attempts share the existing reservation and
concurrency controls. WAV validation is structural only; no local decoding.
Live API and hardware tests remain UNVERIFIED.

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

## Local GraphRAG
Opt in with OMEGA_RAG_ENABLED=1, OMEGA_RAG_DB (absolute path in a private directory),
OMEGA_RAG_EMBEDDING_MODEL and OMEGA_RAG_DIMENSION (1–256). POST /v1/rag/index
replaces one snapshot with expected_revision, sources [{id,body,embedding,sha256:
[32 digest bytes]}] and edges [{from,to}]. POST /v1/rag/retrieve accepts query,
embedding,seeds,limit,body_bytes. Returns revision, source text, SHA-256 and graph
path. Uses identical OCAF BM25/cosine/RRF code via repository-relative modules.
No remote ingestion, automatic embedding generation, cloud export or model
execution. Single operator; not a multi-tenant database. Native tests unexecuted.

### Local embeddings
OMEGA_RAG_LOCAL_EMBEDDINGS=1 enables POST /v1/rag/embed with {"input":"text"}.
A separate local embedding server must be configured at 127.0.0.1:8083/v1/embeddings
with the exact configured model name and dimension (1–256). The gateway verifies
response shape and finite nonzero vectors; it does not attest loaded weights.
Cloud and RAG state paths require owned mode-0700 directories and mode-0600
regular files on Unix. Symlinks and hardlinks are rejected at startup; a trusted
operator must not replace paths during use. No Windows private-state backend yet.

## Operator MCP profiles
OMEGA_MCP_PROFILES_JSON is a JSON array of up to 16 {id,url,tools} entries.
Only operator-controlled HTTPS domain URLs and explicit tool names are accepted;
operator must verify server ownership, DNS, and tool semantics. Public server
URLs only; no OAuth token forwarding or caller-supplied endpoints. With cloud
and OMEGA_CLOUD_TOOLS enabled, POST /v1/mcp/start accepts
{profile,model,input,allow_cloud:true,max_output_tokens}. Pending calls return
HTTP 202 with exact tool name, arguments and a random one-shot ticket expiring
in 120 seconds. A human/operator must review them before POST /v1/mcp/approve
{ticket,approve:true}. Denial consumes the ticket without another API request.
Approvals are memory-only and invalidated by restart; second approval requests
fail closed. Both API requests are separately budgeted. The remote server may
receive data; trust is not established by its URL or tool name. This is a
single-operator path, not general multi-tenant MCP authorization. Live MCP
contract remains unverified.

## Bidirectional Realtime PCM
GET /v1/audio/realtime-stream upgrades to WebSocket. Native/trusted backend
clients must send Authorization: Bearer <gateway token>, X-Omega-Cloud-Consent:
yes, and no Origin header. Voice/cloud enablement and a reservation are required.
Binary input is 24 kHz mono signed PCM16 LE, max 32 KiB per frame; text "commit"
commits at least 100 ms of audio and requests one response. Receive binary PCM
and JSON ready/done/error messages. Audio may be buffered while a response is
streaming; committing while a response is active is rejected. Three turns,
90s, 2 MiB input, 4 MiB output, 8192 messages. Session consumes one operator-
priced reservation; price must account for all three turns. No tools, arbitrary
client JSON, URL tokens or direct browser origin. No playback truncation,
barge-in/cancellation, microphone capture, or WebRTC media transport. No native
or live provider verification yet.

## Local document ingestion (Termux / Python)
Run from clients/ with OMEGA_GATEWAY_TOKEN in a private environment:
```sh
python -m unittest -v test_omega_ingest.py
python omega_ingest.py ingest --revision 0 /path/to/document.txt
python omega_ingest.py query "question"
```
Ingest explicitly replaces the entire snapshot and requires its current revision.
Supports UTF-8 files only (not PDF parsing), 32 files / 1 MiB / 256 chunks,
SHA-256 provenance and directed links between adjacent chunks. All embeddings
and retrieval requests use the local gateway; no cloud call. The result maps
source IDs to local filenames; retain it with the snapshot revision. No
recursive scanning. Tests use deterministic mocked HTTP and embedding responses.

## Audio-only WebRTC signalling (experimental)
Set OMEGA_CLOUD_WEBRTC=1, OMEGA_CLOUD_VOICE=1 and OMEGA_WEBRTC_DB in a private
state directory. Cloud budget and an operator reservation for the complete
session are mandatory. POST /v1/audio/webrtc/start accepts {sdp,allow_cloud:true}
from a native or trusted backend client. Exactly one audio media section is
allowed; no data channel, video, arbitrary session settings or browser Origin.
Returns SDP answer, opaque lease ticket and 90-second watchdog target. The
native client negotiates and carries media directly with OpenAI. POST
/v1/audio/webrtc/stop accepts {ticket} and attempts provider hangup. SQLite
fences all further WebRTC admissions until hangup is confirmed. Startup tries
to close a previously bound call; an unknown creation outcome remains fenced
for operator reconciliation. Keep the WebRTC database/configuration available
until all calls are reconciled. Host loss can delay hangup until restart or
the provider's own timeout: 90 seconds is NOT a hard external spending limit.
Do not expose this route to untrusted clients. No live WebRTC test, device media
capture, provider-side hard cost limit or production authorization is claimed.
