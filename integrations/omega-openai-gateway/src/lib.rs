use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    #[default]
    Local,
    Astra,
    Sol,
    Terra,
    Luna,
}

impl Model {
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Astra => "gpt-6.2-astra",
            Self::Sol => "gpt-6.2-sol",
            Self::Terra => "gpt-6.1-terra",
            Self::Luna => "gpt-6-luna",
        }
    }
    pub fn cloud(self) -> bool {
        self != Self::Local
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    #[default]
    None,
    Docs,
    Web,
}

fn default_tokens() -> u32 { 512 }

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub input: String,
    #[serde(default)]
    pub history: Vec<Message>,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub allow_cloud: bool,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub tool: Tool,
    #[serde(default = "default_tokens")]
    pub max_output_tokens: u32,
}

pub fn admit(req: &Request, cloud_enabled: bool, tools_enabled: bool) -> Result<(), &'static str> {
    if req.input.trim().is_empty() || req.input.len() > 16_384 ||
       !(16..=4096).contains(&req.max_output_tokens) {
        return Err("invalid_input");
    }
    if req.history.len() > 32 || req.history.len() % 2 != 0 {
        return Err("invalid_input");
    }
    let mut total = req.input.len();
    for (index, message) in req.history.iter().enumerate() {
        let role = if index % 2 == 0 { Role::User } else { Role::Assistant };
        if message.role != role || message.content.trim().is_empty() {
            return Err("invalid_input");
        }
        total = total.checked_add(message.content.len()).ok_or("invalid_input")?;
        if total > 16_384 {
            return Err("invalid_input");
        }
    }
    if req.model.cloud() && !(cloud_enabled && req.allow_cloud) {
        return Err("cloud_denied");
    }
    if req.tool != Tool::None && !(req.model.cloud() && tools_enabled) {
        return Err("tool_denied");
    }
    Ok(())
}

impl Request {
    pub fn payload(&self, local_model: &str) -> Value {
        let mut messages: Vec<Value> = self.history.iter().map(|message| json!(message)).collect();
        messages.push(json!({"role":"user","content":self.input}));
        if !self.model.cloud() {
            return json!({"model":local_model,"messages":messages,
                "max_tokens":self.max_output_tokens,"stream":self.stream});
        }
        let input = if self.history.is_empty() { json!(self.input) } else { json!(messages) };
        let mut value = json!({"model":self.model.id(),"input":input,
            "max_output_tokens":self.max_output_tokens,"stream":self.stream,"store":false});
        match self.tool {
            Tool::None => (),
            Tool::Docs => {
                value["tools"] = json!([{"type":"mcp","server_label":"openai_docs",
                    "server_description":"Search public OpenAI documentation",
                    "server_url":"https://developers.openai.com/mcp",
                    "allowed_tools":["search_openai_docs"],"require_approval":"never"}]);
                value["max_tool_calls"] = json!(1);
            }
            Tool::Web => {
                value["tools"] = json!([{"type":"web_search"}]);
                value["max_tool_calls"] = json!(1);
            }
        }
        value
    }
}

/// Persistent UTC-day request quota. Reservations are never refunded, even on
/// timeout: an uncertain upstream request may have consumed money.
pub fn reserve(path: &Path, cap: u32) -> rusqlite::Result<bool> {
    if cap == 0 { return Ok(false); }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS gateway_quota (
        day TEXT PRIMARY KEY, calls INTEGER NOT NULL CHECK(calls >= 0)
    )")?;
    let changed = conn.execute(
        "INSERT INTO gateway_quota(day,calls) VALUES(date('now'),1)
         ON CONFLICT(day) DO UPDATE SET calls=calls+1 WHERE calls < ?1",
        params![cap],
    )?;
    Ok(changed == 1)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Deliberately small, local-only subset of Chat Completions.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub n: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub seed: Option<u32>,
}

impl LocalChatRequest {
    pub fn payload(&self, configured_model: &str) -> Result<Value, &'static str> {
        if self.model != "omega-local"
            || self.messages.is_empty()
            || self.messages.len() > 34
            || self.n.is_some_and(|n| n != 1)
            || (self.max_tokens.is_some() && self.max_completion_tokens.is_some())
            || self.temperature.is_some_and(|v| !(0.0..=2.0).contains(&v))
            || self.top_p.is_some_and(|v| !(0.0..=1.0).contains(&v))
            || self.seed.is_some_and(|v| v > i32::MAX as u32)
        {
            return Err("invalid_input");
        }
        let max_tokens = self.max_tokens.or(self.max_completion_tokens).unwrap_or(512);
        if !(16..=4096).contains(&max_tokens) {
            return Err("invalid_input");
        }
        let mut expected = ChatRole::User;
        let mut total = 0usize;
        for (index, message) in self.messages.iter().enumerate() {
            total = total.checked_add(message.content.len()).ok_or("invalid_input")?;
            if total > 16_384 {
                return Err("invalid_input");
            }
            if index == 0 && message.role == ChatRole::System {
                continue;
            }
            if message.role != expected || message.content.trim().is_empty() {
                return Err("invalid_input");
            }
            expected = if expected == ChatRole::User { ChatRole::Assistant } else { ChatRole::User };
        }
        if expected != ChatRole::Assistant {
            return Err("invalid_input");
        }
        let mut payload = json!({
            "model":configured_model,"messages":self.messages,
            "stream":self.stream,"max_tokens":max_tokens,"n":1
        });
        if let Some(temperature) = self.temperature {
            payload["temperature"] = json!(temperature);
        }
        if let Some(top_p) = self.top_p {
            payload["top_p"] = json!(top_p);
        }
        if let Some(seed) = self.seed {
            payload["seed"] = json!(seed);
        }
        Ok(payload)
    }
}

pub mod advanced;

pub mod voice;

// Compile the same OCAF retrieval implementation, without importing its runtime.
#[path = "../../../src/retrieval_quality.rs"]
pub mod retrieval_quality;
#[path = "../../../src/hybrid_retrieval.rs"]
pub mod hybrid_retrieval;
#[path = "../../../src/gateway_graph_rag.rs"]
pub mod gateway_graph_rag;
pub mod rag;
