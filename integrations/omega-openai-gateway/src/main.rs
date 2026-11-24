use axum::{
    body::Body, extract::{DefaultBodyLimit, State}, http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response}, routing::{get, post}, Json, Router,
};
use futures_util::StreamExt;
use omega_openai_gateway::{admit, LocalChatRequest, Request};
use omega_openai_gateway::advanced::{AdvancedRequest, AdvancedTool, ToolPolicy, execute_function, function_continuation, reserve_budget, valid_id};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, path::PathBuf, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use omega_openai_gateway::voice::{self, TranscribeRequest, VoiceRequest};

const CLOUD: &str = "https://api.openai.com/v1/responses";
const LOCAL: &str = "http://127.0.0.1:8082/v1/chat/completions";
const MAX_RESPONSE: usize = 2 * 1024 * 1024;

use omega_openai_gateway::rag::{RagStore, RagError, Snapshot, Query};
use omega_openai_gateway::embedding::{EmbedRequest,parse_embedding};
use omega_openai_gateway::private_db::prepare_private_database;

use omega_openai_gateway::mcp::{Profiles,ApprovalStore,StartRequest,ApprovalRequest,ApprovalDecision};

struct App {
    client: reqwest::Client,
    token_hash: [u8; 32],
    api_key: Option<String>,
    cloud_cap: u32,
    daily_budget: u64,
    reservation: u64,
    advanced: ToolPolicy,
    voice: bool,
    rag: Option<RagStore>,
    embeddings: bool,
    mcp_profiles: Profiles,
    mcp_approvals: std::sync::Mutex<ApprovalStore>,
    tools: bool,
    db: Option<PathBuf>,
    local_model: String,
    slots: Arc<Semaphore>,
}

fn error(status: StatusCode, code: &'static str) -> Response {
    (status, Json(json!({"error":code}))).into_response()
}

fn authenticated(headers: &HeaderMap, expected: &[u8; 32]) -> bool {
    let Some(value) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(token) = value.strip_prefix("Bearer ") else { return false; };
    if token.len() > 512 { return false; }
    let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    bool::from(digest.ct_eq(expected))
}

async fn generate(State(app): State<Arc<App>>, headers: HeaderMap, Json(req): Json<Request>) -> Response {
    if !authenticated(&headers, &app.token_hash) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let cloud = req.model.cloud();
    if let Err(code) = admit(&req, app.cloud_cap > 0 && app.api_key.is_some(), app.tools) {
        let status = if code == "invalid_input" { StatusCode::BAD_REQUEST } else { StatusCode::FORBIDDEN };
        return error(status, code);
    }
    let payload = req.payload(&app.local_model);
    forward(app, cloud, req.stream, payload).await
}

async fn advanced_generate(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(req): Json<AdvancedRequest>,
) -> Response {
    if !authenticated(&headers, &app.token_hash) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let payload = match req.payload(app.cloud_cap > 0 && app.api_key.is_some(), &app.advanced) {
        Ok(p) => p,
        Err(code) => return error(if code == "invalid_input" { StatusCode::BAD_REQUEST } else { StatusCode::FORBIDDEN }, code),
    };
    if req.tool == AdvancedTool::Sha256 {
        return function_turn(app, payload).await;
    }
    forward(app, true, req.stream, payload).await
}

async fn local_function(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(call): Json<serde_json::Value>,
) -> Response {
    if !authenticated(&headers, &app.token_hash) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    match execute_function(&call, &app.advanced) {
        Ok(result) => Json(result).into_response(),
        Err(code) => error(if code == "tool_denied" { StatusCode::FORBIDDEN } else { StatusCode::BAD_REQUEST }, code),
    }
}

async fn local_chat(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Json(req): Json<LocalChatRequest>,
) -> Response {
    if !authenticated(&headers, &app.token_hash) {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let payload = match req.payload(&app.local_model) {
        Ok(payload) => payload,
        Err(code) => return error(StatusCode::BAD_REQUEST, code),
    };
    forward(app, false, req.stream, payload).await
}


async fn cloud_json(app: &App, payload: &serde_json::Value) -> Result<serde_json::Value,(StatusCode,&'static str)> {
    let (_permit,key) = cloud_lease(app).await?;
    let upstream = app.client.post(CLOUD).bearer_auth(key).json(payload).send().await
        .map_err(|_| (StatusCode::BAD_GATEWAY,"upstream_unavailable"))?;
    if !upstream.status().is_success() {
        return Err((if upstream.status() == reqwest::StatusCode::TOO_MANY_REQUESTS { StatusCode::TOO_MANY_REQUESTS } else { StatusCode::BAD_GATEWAY },"upstream_rejected"));
    }
    let actual = upstream.headers().get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()).unwrap_or("");
    if actual.split(';').next().map(str::trim) != Some("application/json") {
        return Err((StatusCode::BAD_GATEWAY,"upstream_content_type"));
    }
    let mut stream = upstream.bytes_stream();
    let mut data = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(b) if b.len() <= MAX_RESPONSE.saturating_sub(data.len()) => data.extend_from_slice(&b),
            _ => return Err((StatusCode::BAD_GATEWAY,"upstream_body_rejected")),
        }
    }
    serde_json::from_slice(&data).map_err(|_| (StatusCode::BAD_GATEWAY,"upstream_invalid_json"))
}

async fn function_turn(app: Arc<App>, payload: serde_json::Value) -> Response {
    let first = match cloud_json(&app,&payload).await {
        Ok(v) => v, Err((status,code)) => return error(status,code),
    };
    let next = match function_continuation(&payload,&first,&app.advanced) {
        Ok(Some(v)) => v,
        Ok(None) => return ([(header::CACHE_CONTROL,"no-store"),(header::HeaderName::from_static("x-omega-provider"),"openai")],Json(first)).into_response(),
        Err(code) => return error(StatusCode::BAD_GATEWAY,code),
    };
    let result = match cloud_json(&app,&next).await {
        Ok(v) => v, Err((status,code)) => return error(status,code),
    };
    let valid = result["status"] == "completed" && result["output"].as_array().is_some_and(|items|
        !items.is_empty() && items.iter().all(|item| matches!(item["type"].as_str(),Some("reasoning" | "message"))));
    if !valid { return error(StatusCode::BAD_GATEWAY,"upstream_incomplete"); }
    ([(header::CACHE_CONTROL,"no-store"),(header::HeaderName::from_static("x-omega-provider"),"openai")],Json(result)).into_response()
}

async fn cloud_lease(app: &App) -> Result<(OwnedSemaphorePermit, &str), (StatusCode, &'static str)> {
    let permit = app.slots.clone().try_acquire_owned().map_err(|_| (StatusCode::TOO_MANY_REQUESTS, "busy"))?;
    let (Some(db), Some(key)) = (app.db.clone(), app.api_key.as_deref()) else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "cloud_unconfigured"));
    };
    let (cap, daily, amount) = (app.cloud_cap, app.daily_budget, app.reservation);
    match tokio::task::spawn_blocking(move || reserve_budget(&db, cap, daily, amount)).await {
        Ok(Ok(true)) => Ok((permit,key)),
        Ok(Ok(false)) => Err((StatusCode::TOO_MANY_REQUESTS,"quota_exhausted")),
        _ => Err((StatusCode::SERVICE_UNAVAILABLE,"quota_unavailable")),
    }
}

async fn transcribe(State(app): State<Arc<App>>, headers: HeaderMap, Json(req): Json<TranscribeRequest>) -> Response {
    if !authenticated(&headers, &app.token_hash) { return error(StatusCode::UNAUTHORIZED,"unauthorized"); }
    let wav = match req.wav(app.voice && app.cloud_cap > 0) {
        Ok(wav) => wav,
        Err(code) => return error(if code == "invalid_input" { StatusCode::BAD_REQUEST } else { StatusCode::FORBIDDEN },code),
    };
    let (_permit,key) = match cloud_lease(&app).await { Ok(x) => x, Err((status,code)) => return error(status,code) };
    let part = match reqwest::multipart::Part::bytes(wav).file_name("audio.wav").mime_str("audio/wav") {
        Ok(part) => part, Err(_) => return error(StatusCode::BAD_REQUEST,"invalid_input"),
    };
    let form = reqwest::multipart::Form::new().text("model","gpt-transcribe").part("file",part);
    let upstream = match app.client.post("https://api.openai.com/v1/audio/transcriptions").bearer_auth(key).multipart(form).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return error(StatusCode::BAD_GATEWAY,"upstream_rejected"),
    };
    let mut stream = upstream.bytes_stream();
    let mut data = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(b) if b.len() <= MAX_RESPONSE.saturating_sub(data.len()) => data.extend_from_slice(&b),
            _ => return error(StatusCode::BAD_GATEWAY,"upstream_body_rejected"),
        }
    }
    match serde_json::from_slice::<serde_json::Value>(&data) {
        Ok(v) if v["text"].is_string() => Json(v).into_response(),
        _ => error(StatusCode::BAD_GATEWAY,"upstream_protocol"),
    }
}

async fn realtime_voice(State(app): State<Arc<App>>, headers: HeaderMap, Json(req): Json<VoiceRequest>) -> Response {
    if !authenticated(&headers, &app.token_hash) { return error(StatusCode::UNAUTHORIZED,"unauthorized"); }
    let item = match req.item(app.voice && app.cloud_cap > 0) {
        Ok(v) => v,
        Err(code) => return error(if code == "invalid_input" { StatusCode::BAD_REQUEST } else { StatusCode::FORBIDDEN },code),
    };
    let (_permit,key) = match cloud_lease(&app).await { Ok(x) => x, Err((status,code)) => return error(status,code) };
    match voice::realtime(item,key).await {
        Ok(v) => Json(v).into_response(),
        Err(code) => error(StatusCode::BAD_GATEWAY,code),
    }
}

async fn forward(
    app: Arc<App>, cloud: bool, streaming: bool, payload: serde_json::Value,
) -> Response {
    let (permit,key) = if cloud {
        match cloud_lease(&app).await { Ok((p,k)) => (p,Some(k)), Err((status,code)) => return error(status,code) }
    } else {
        match app.slots.clone().try_acquire_owned() {
            Ok(p) => (p,None), Err(_) => return error(StatusCode::TOO_MANY_REQUESTS,"busy"),
        }
    };
    let mut outbound = app.client.post(if cloud { CLOUD } else { LOCAL }).json(&payload);
    if let Some(key) = key { outbound = outbound.bearer_auth(key); }
    let upstream = match outbound.send().await {
        Ok(r) => r,
        Err(_) => return error(StatusCode::BAD_GATEWAY, "upstream_unavailable"),
    };
    if !upstream.status().is_success() {
        let status = if upstream.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            StatusCode::TOO_MANY_REQUESTS
        } else { StatusCode::BAD_GATEWAY };
        return error(status, "upstream_rejected");
    }
    let expected = if streaming { "text/event-stream" } else { "application/json" };
    let actual = upstream.headers().get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()).unwrap_or("");
    if actual.split(';').next().map(str::trim) != Some(expected) {
        return error(StatusCode::BAD_GATEWAY, "upstream_content_type");
    }
    let mut bytes = upstream.bytes_stream();
    if !streaming {
        let mut data = Vec::new();
        while let Some(chunk) = bytes.next().await {
            match chunk {
                Ok(chunk) if chunk.len() <= MAX_RESPONSE.saturating_sub(data.len()) => data.extend_from_slice(&chunk),
                _ => return error(StatusCode::BAD_GATEWAY, "upstream_body_rejected"),
            }
        }
        drop(permit);
        if serde_json::from_slice::<serde_json::Value>(&data).is_err() {
            return error(StatusCode::BAD_GATEWAY, "upstream_invalid_json");
        }
        return ([(header::CONTENT_TYPE, expected), (header::CACHE_CONTROL, "no-store"),
            (header::HeaderName::from_static("x-omega-provider"), if cloud { "openai" } else { "local" })], data).into_response();
    }
    let stream = async_stream::stream! {
        let _permit = permit;
        let mut total = 0usize;
        while let Some(item) = bytes.next().await {
            match item {
                Ok(chunk) if chunk.len() <= MAX_RESPONSE.saturating_sub(total) => {
                    total += chunk.len();
                    yield Ok::<_, std::io::Error>(chunk);
                }
                _ => {
                    yield Err(std::io::Error::other("upstream stream interrupted or oversized"));
                    break;
                }
            }
        }
    };
    ([(header::CONTENT_TYPE, expected), (header::CACHE_CONTROL, "no-store"),
      (header::HeaderName::from_static("x-omega-provider"), if cloud { "openai" } else { "local" })],
      Body::from_stream(stream)).into_response()
}

fn rag_error(e: RagError) -> Response {
    match e {
        RagError::Invalid => error(StatusCode::BAD_REQUEST,"invalid_graph"),
        RagError::Conflict => error(StatusCode::CONFLICT,"stale_revision"),
        RagError::Storage => error(StatusCode::SERVICE_UNAVAILABLE,"rag_unavailable"),
    }
}
async fn rag_replace(State(app): State<Arc<App>>, headers: HeaderMap, Json(snapshot): Json<Snapshot>) -> Response {
    if !authenticated(&headers,&app.token_hash) {return error(StatusCode::UNAUTHORIZED,"unauthorized");}
    let Some(store)=app.rag.clone() else {return error(StatusCode::FORBIDDEN,"rag_disabled");};
    let Ok(permit)=app.slots.clone().try_acquire_owned() else {return error(StatusCode::TOO_MANY_REQUESTS,"busy");};
    match tokio::task::spawn_blocking(move||{let _permit=permit;store.replace(snapshot)}).await {
        Ok(Ok(revision))=>Json(json!({"revision":revision})).into_response(),
        Ok(Err(e))=>rag_error(e),
        Err(_)=>error(StatusCode::SERVICE_UNAVAILABLE,"rag_unavailable"),
    }
}
async fn rag_retrieve(State(app): State<Arc<App>>, headers: HeaderMap, Json(query): Json<Query>) -> Response {
    if !authenticated(&headers,&app.token_hash) {return error(StatusCode::UNAUTHORIZED,"unauthorized");}
    let Some(store)=app.rag.clone() else {return error(StatusCode::FORBIDDEN,"rag_disabled");};
    let Ok(permit)=app.slots.clone().try_acquire_owned() else {return error(StatusCode::TOO_MANY_REQUESTS,"busy");};
    match tokio::task::spawn_blocking(move||{let _permit=permit;store.retrieve(query)}).await {
        Ok(Ok(result))=>([(header::CACHE_CONTROL,"no-store")],Json(result)).into_response(),
        Ok(Err(e))=>rag_error(e),
        Err(_)=>error(StatusCode::SERVICE_UNAVAILABLE,"rag_unavailable"),
    }
}

async fn rag_embed(State(app):State<Arc<App>>,headers:HeaderMap,Json(req):Json<EmbedRequest>)->Response{
    if !authenticated(&headers,&app.token_hash){return error(StatusCode::UNAUTHORIZED,"unauthorized");}
    let Some(store)=app.rag.as_ref().filter(|_|app.embeddings) else{return error(StatusCode::FORBIDDEN,"embeddings_disabled");};
    let payload=match req.payload(store.model()){Ok(v)=>v,Err(code)=>return error(StatusCode::BAD_REQUEST,code)};
    let Ok(_permit)=app.slots.clone().try_acquire_owned() else{return error(StatusCode::TOO_MANY_REQUESTS,"busy");};
    let response=match app.client.post("http://127.0.0.1:8083/v1/embeddings").json(&payload).send().await{
        Ok(r) if r.status().is_success()=>r,
        _=>return error(StatusCode::BAD_GATEWAY,"upstream_unavailable"),
    };
    let mut chunks=response.bytes_stream();
    let mut data=Vec::new();
    while let Some(chunk)=chunks.next().await{
        match chunk{
            Ok(b) if b.len()<=65536usize.saturating_sub(data.len())=>data.extend_from_slice(&b),
            _=>return error(StatusCode::BAD_GATEWAY,"upstream_body_rejected"),
        }
    }
    let v:serde_json::Value=match serde_json::from_slice(&data){
        Ok(v)=>v,Err(_)=>return error(StatusCode::BAD_GATEWAY,"upstream_invalid_json"),
    };
    match parse_embedding(&v,store.model(),store.dimension()){
        Ok(embedding)=>([(header::CACHE_CONTROL,"no-store")],Json(json!({"model":store.model(),"embedding":embedding}))).into_response(),
        Err(code)=>error(StatusCode::BAD_GATEWAY,code),
    }
}

async fn mcp_start(State(app):State<Arc<App>>,headers:HeaderMap,Json(req):Json<StartRequest>)->Response{
    if !authenticated(&headers,&app.token_hash){return error(StatusCode::UNAUTHORIZED,"unauthorized");}
    let payload=match req.payload(&app.mcp_profiles,app.tools&&app.cloud_cap>0){
        Ok(v)=>v,Err(code)=>return error(if code=="invalid_input"{StatusCode::BAD_REQUEST}else{StatusCode::FORBIDDEN},code),
    };
    let first=match cloud_json(&app,&payload).await{Ok(v)=>v,Err((status,code))=>return error(status,code)};
    let approval=match app.mcp_approvals.lock(){
        Ok(mut store)=>store.issue(&app.mcp_profiles,&payload,&first,std::time::Instant::now()),
        Err(_)=>return error(StatusCode::SERVICE_UNAVAILABLE,"approval_unavailable"),
    };
    match approval{
        Ok(Some(ticket))=>(StatusCode::ACCEPTED,[(header::CACHE_CONTROL,"no-store")],Json(ticket)).into_response(),
        Ok(None)=>([(header::CACHE_CONTROL,"no-store")],Json(first)).into_response(),
        Err(code)=>error(StatusCode::BAD_GATEWAY,code),
    }
}
async fn mcp_approve(State(app):State<Arc<App>>,headers:HeaderMap,Json(req):Json<ApprovalRequest>)->Response{
    if !authenticated(&headers,&app.token_hash){return error(StatusCode::UNAUTHORIZED,"unauthorized");}
    if !app.tools||app.cloud_cap==0{return error(StatusCode::FORBIDDEN,"tool_denied");}
    let decision=match app.mcp_approvals.lock(){
        Ok(mut store)=>store.consume(&req.ticket,req.approve,std::time::Instant::now()),
        Err(_)=>return error(StatusCode::SERVICE_UNAVAILABLE,"approval_unavailable"),
    };
    let payload=match decision{
        Ok(ApprovalDecision::Approved(v))=>v,
        Ok(ApprovalDecision::Denied)=>return Json(json!({"status":"denied"})).into_response(),
        Err(code)=>return error(StatusCode::BAD_REQUEST,code),
    };
    match cloud_json(&app,&payload).await{
        Ok(v) if v["status"]=="completed"&&v["output"].as_array().is_some_and(|items|
            !items.is_empty()&&items.iter().all(|i|matches!(i["type"].as_str(),Some("reasoning"|"message"|"mcp_list_tools"|"mcp_call"))))=>
            ([(header::CACHE_CONTROL,"no-store")],Json(v)).into_response(),
        Ok(_)=>error(StatusCode::BAD_GATEWAY,"upstream_incomplete"),
        Err((status,code))=>error(status,code),
    }
}

fn config() -> Result<App, Box<dyn std::error::Error>> {
    let token = env::var("OMEGA_GATEWAY_TOKEN")?;
    if !(32..=512).contains(&token.len()) || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("OMEGA_GATEWAY_TOKEN must contain 32-512 visible ASCII characters".into());
    }
    let cloud_cap: u32 = match env::var("OMEGA_CLOUD_DAILY_REQUESTS") {
        Ok(v) => v.parse()?,
        Err(env::VarError::NotPresent) => 0,
        Err(e) => return Err(e.into()),
    };
    if cloud_cap > 1000 { return Err("cloud quota exceeds maximum".into()); }
    let (api_key, db) = if cloud_cap > 0 {
        let key = env::var("OPENAI_API_KEY")?;
        if key.trim().is_empty() || key.len() > 512 {
            return Err("invalid API key configuration".into());
        }
        let db = PathBuf::from(env::var("OMEGA_QUOTA_DB")?);
        if !db.is_absolute() || db.as_os_str().is_empty() {
            return Err("quota path must be absolute and in a private directory".into());
        }
        (Some(key), Some(prepare_private_database(&db)?))
    } else { (None, None) };
    let (daily_budget, reservation): (u64, u64) = if cloud_cap > 0 {
        let daily: u64 = env::var("OMEGA_CLOUD_DAILY_BUDGET_MICRO_USD")?.parse()?;
        let per_call: u64 = env::var("OMEGA_CLOUD_RESERVE_MICRO_USD")?.parse()?;
        if per_call == 0 || per_call > daily || daily > i64::MAX as u64 {
            return Err("invalid cloud reservation budget".into());
        }
        (daily, per_call)
    } else { (0, 0) };
    let files: Vec<String> = env::var("OMEGA_ALLOWED_FILE_IDS").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(str::to_owned).collect();
    let vector_store = env::var("OMEGA_VECTOR_STORE_ID").ok().filter(|s| !s.is_empty());
    if files.len() > 64 || files.iter().any(|id| !valid_id(id, "file-"))
        || vector_store.as_ref().is_some_and(|id| !valid_id(id, "vs_")) {
        return Err("invalid advanced tool configuration".into());
    }
    let advanced = ToolPolicy { files, vector_store,
        code: env::var("OMEGA_CODE_INTERPRETER").as_deref() == Ok("1"),
        function: env::var("OMEGA_LOCAL_FUNCTIONS").as_deref() == Ok("1") };
    let local_model = env::var("OMEGA_LOCAL_MODEL").unwrap_or_else(|_| "local".into());
    if local_model.is_empty() || local_model.len() > 256 {
        return Err("invalid local model configuration".into());
    }
    let rag=if env::var("OMEGA_RAG_ENABLED").as_deref()==Ok("1") {
        let path=PathBuf::from(env::var("OMEGA_RAG_DB")?);
        let model=env::var("OMEGA_RAG_EMBEDDING_MODEL")?;
        let dim:usize=env::var("OMEGA_RAG_DIMENSION")?.parse()?;
        Some(RagStore::open(&prepare_private_database(&path)?,&model,dim)?)
    }else{None};
    let mcp_profiles=Profiles::parse(&env::var("OMEGA_MCP_PROFILES_JSON").unwrap_or_else(|_|"[]".into()))?;
    let client = reqwest::Client::builder().no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120)).build()?;
    Ok(App {
        client, token_hash: Sha256::digest(token.as_bytes()).into(), api_key, cloud_cap, daily_budget, reservation, advanced, rag,
        mcp_profiles, mcp_approvals: std::sync::Mutex::new(ApprovalStore::default()),
        embeddings: env::var("OMEGA_RAG_LOCAL_EMBEDDINGS").as_deref()==Ok("1"),
        voice: env::var("OMEGA_CLOUD_VOICE").as_deref() == Ok("1"),
        tools: env::var("OMEGA_CLOUD_TOOLS").as_deref() == Ok("1"),
        db, local_model, slots: Arc::new(Semaphore::new(4)),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Arc::new(config()?);
    let router = Router::new()
        .route("/healthz", get(|| async { StatusCode::NO_CONTENT }))
        .route("/v1/generate", post(generate))
        .route("/v1/chat/completions", post(local_chat))
        .route("/v1/functions/execute", post(local_function))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .route("/v1/analyze", post(advanced_generate).layer(DefaultBodyLimit::max(3 * 1024 * 1024)))
        .route("/v1/audio/transcribe", post(transcribe).layer(DefaultBodyLimit::max(3 * 1024 * 1024)))
        .route("/v1/audio/realtime-turn", post(realtime_voice).layer(DefaultBodyLimit::max(3 * 1024 * 1024)))
        .route("/v1/rag/index", post(rag_replace).layer(DefaultBodyLimit::max(3*1024*1024)))
        .route("/v1/rag/retrieve", post(rag_retrieve))
        .route("/v1/rag/embed", post(rag_embed))
        .route("/v1/mcp/start", post(mcp_start))
        .route("/v1/mcp/approve", post(mcp_approve))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8788").await?;
    eprintln!("OMEGA gateway listening on loopback");
    axum::serve(listener, router).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    }).await?;
    Ok(())
}
