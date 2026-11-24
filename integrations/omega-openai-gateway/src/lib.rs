use rusqlite::{params, Connection};
use serde::Deserialize;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub input: String,
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
        if !self.model.cloud() {
            return json!({"model":local_model,"messages":[{"role":"user","content":self.input}],
                "max_tokens":self.max_output_tokens,"stream":self.stream});
        }
        let mut value = json!({"model":self.model.id(),"input":self.input,
            "max_output_tokens":self.max_output_tokens,"stream":self.stream,"store":false});
        match self.tool {
            Tool::None => (),
            Tool::Docs => {
                value["tools"] = json!([{"type":"mcp","server_label":"openai_docs",
                    "server_description":"Search public OpenAI documentation",
                    "server_url":"https://developers.openai.com/mcp",
                    "allowed_tools":["search_openai_docs"],"require_approval":"never"}]);
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
