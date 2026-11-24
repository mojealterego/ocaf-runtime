//! Explicit cloud-only multimodal and tool admission; no caller-selected URLs.
use crate::Model;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

#[derive(Default)]
pub struct ToolPolicy {
    pub files: Vec<String>,
    pub vector_store: Option<String>,
    pub code: bool,
    pub function: bool,
}

pub fn valid_id(id: &str, prefix: &str) -> bool {
    id.starts_with(prefix) && id.len() > prefix.len() && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedTool {
    #[default]
    None,
    FileSearch,
    CodeInterpreter,
    Sha256,
}

fn default_tokens() -> u32 { 512 }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedRequest {
    pub model: Model,
    pub input: String,
    #[serde(default)]
    pub allow_cloud: bool,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_tokens")]
    pub max_output_tokens: u32,
    pub image: Option<String>,
    pub file_id: Option<String>,
    #[serde(default)]
    pub tool: AdvancedTool,
}

fn image_is_valid(image: &str) -> bool {
    if image.len() > 2_800_000 { return false; }
    let (encoded, png) = if let Some(s) = image.strip_prefix("data:image/png;base64,") {
        (s, true)
    } else if let Some(s) = image.strip_prefix("data:image/jpeg;base64,") {
        (s, false)
    } else { return false; };
    let Ok(decoded) = STANDARD.decode(encoded) else { return false; };
    decoded.len() <= 2 * 1024 * 1024 && if png {
        decoded.starts_with(b"\x89PNG\r\n\x1a\n")
    } else {
        decoded.starts_with(b"\xff\xd8\xff")
    }
}

impl AdvancedRequest {
    pub fn payload(&self, cloud_enabled: bool, policy: &ToolPolicy) -> Result<Value, &'static str> {
        if !self.model.cloud() || !self.allow_cloud || !cloud_enabled { return Err("cloud_denied"); }
        if self.input.trim().is_empty() || self.input.len() > 16_384
            || !(16..=4096).contains(&self.max_output_tokens) { return Err("invalid_input"); }
        let mut content = vec![json!({"type":"input_text","text":self.input})];
        if let Some(image) = &self.image {
            if !image_is_valid(image) { return Err("invalid_input"); }
            content.push(json!({"type":"input_image","image_url":image,"detail":"auto"}));
        }
        if let Some(id) = &self.file_id {
            if !valid_id(id, "file-") || !policy.files.contains(id) { return Err("tool_denied"); }
            content.push(json!({"type":"input_file","file_id":id}));
        }
        let mut payload = json!({"model":self.model.id(),"input":[{"role":"user","content":content}],
            "store":false,"stream":self.stream,"max_output_tokens":self.max_output_tokens});
        let tool = match self.tool {
            AdvancedTool::None => return Ok(payload),
            AdvancedTool::FileSearch => {
                let id = policy.vector_store.as_ref().filter(|id| valid_id(id,"vs_")).ok_or("tool_denied")?;
                json!({"type":"file_search","vector_store_ids":[id],"max_num_results":5})
            }
            AdvancedTool::CodeInterpreter => {
                if !policy.code { return Err("tool_denied"); }
                json!({"type":"code_interpreter","container":{"type":"auto"}})
            }
            AdvancedTool::Sha256 => {
                if !policy.function { return Err("tool_denied"); }
                if self.stream { return Err("invalid_input"); }
                payload["include"] = json!(["reasoning.encrypted_content"]);
                payload["parallel_tool_calls"] = json!(false);
                json!({"type":"function","name":"sha256_text","description":"Return SHA-256 of UTF-8 text.",
                    "strict":true,"parameters":{"type":"object","properties":{"text":{"type":"string"}},
                    "required":["text"],"additionalProperties":false}})
            }
        };
        payload["tools"] = json!([tool]);
        // max_tool_calls limits hosted tools, not custom function calls.
        payload["max_tool_calls"] = json!(1);
        Ok(payload)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionCall { name: String, arguments: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HashArguments { text: String }

/// Pure operation. No filesystem, shell, network or model-controlled dispatch.
pub fn execute_function(call: &Value, policy: &ToolPolicy) -> Result<Value, &'static str> {
    if !policy.function { return Err("tool_denied"); }
    let call: FunctionCall = serde_json::from_value(call.clone()).map_err(|_| "invalid_input")?;
    if call.name != "sha256_text" || call.arguments.len() > 20_000 { return Err("invalid_input"); }
    let args: HashArguments = serde_json::from_str(&call.arguments).map_err(|_| "invalid_input")?;
    if args.text.len() > 16_384 { return Err("invalid_input"); }
    Ok(json!({"sha256":format!("{:x}",Sha256::digest(args.text.as_bytes()))}))
}

/// Operator-priced, durable admission reservation, not an invoice or a
/// provider-enforced spending ceiling. Never refund ambiguous attempts.
pub fn reserve_budget(path: &Path, calls_cap: u32, daily: u64, amount: u64) -> rusqlite::Result<bool> {
    if calls_cap == 0 || amount == 0 || amount > daily || daily > i64::MAX as u64 { return Ok(false); }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS gateway_quota (
        day TEXT PRIMARY KEY, calls INTEGER NOT NULL CHECK(calls >= 0)
    ); CREATE TABLE IF NOT EXISTS gateway_budget (
        day TEXT PRIMARY KEY, calls INTEGER NOT NULL CHECK(calls >= 0),
        reserved INTEGER NOT NULL CHECK(reserved >= 0)
    );")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let day: String = tx.query_row("SELECT date('now')", [], |r| r.get(0))?;
    let calls: i64 = tx.query_row("SELECT calls FROM gateway_quota WHERE day=?1", [&day], |r| r.get(0)).optional()?.unwrap_or(0);
    let (priced_calls, reserved): (i64, i64) = tx.query_row("SELECT calls,reserved FROM gateway_budget WHERE day=?1", [&day], |r| Ok((r.get(0)?,r.get(1)?))).optional()?.unwrap_or((0,0));
    if calls != priced_calls || calls < 0 || reserved < 0 || calls >= i64::from(calls_cap)
        || (reserved as u64) > daily - amount { return Ok(false); }
    tx.execute("INSERT INTO gateway_quota(day,calls) VALUES(?1,?2) ON CONFLICT(day) DO UPDATE SET calls=excluded.calls", params![day,calls+1])?;
    tx.execute("INSERT INTO gateway_budget(day,calls,reserved) VALUES(?1,?2,?3) ON CONFLICT(day) DO UPDATE SET calls=excluded.calls,reserved=excluded.reserved", params![day,calls+1,reserved+(amount as i64)])?;
    tx.commit()?;
    Ok(true)
}

/// Stateless, one-step function continuation. No caller-supplied output items:
/// first is obtained by the server from its own authenticated API response.
pub fn function_continuation(payload: &Value, first: &Value, policy: &ToolPolicy)
    -> Result<Option<Value>, &'static str>
{
    if !policy.function || first["status"] != "completed" { return Err("upstream_incomplete"); }
    let output = first["output"].as_array().ok_or("upstream_protocol")?;
    if output.is_empty() || output.len() > 64 { return Err("upstream_protocol"); }
    let mut function_output = None;
    for item in output {
        match item["type"].as_str() {
            Some("reasoning" | "message") => (),
            Some("function_call") => {
                if function_output.is_some() { return Err("upstream_protocol"); }
                let call_id = item["call_id"].as_str().filter(|id| valid_id(id, "call_")).ok_or("upstream_protocol")?;
                let result = execute_function(&json!({"name":item["name"],"arguments":item["arguments"]}), policy)?;
                function_output = Some(json!({"type":"function_call_output","call_id":call_id,"output":result.to_string()}));
            }
            _ => return Err("upstream_protocol"),
        }
    }
    let Some(result) = function_output else { return Ok(None); };
    let mut next = payload.clone();
    let input = next["input"].as_array_mut().ok_or("invalid_input")?;
    input.extend(output.iter().cloned());
    input.push(result);
    next["tool_choice"] = json!("none");
    next["stream"] = json!(false);
    next["store"] = json!(false);
    Ok(Some(next))
}
