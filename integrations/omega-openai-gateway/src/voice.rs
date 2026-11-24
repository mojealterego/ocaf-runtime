//! Bounded, explicit single-turn voice transport. No automatic cloud fallback.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async_with_config, tungstenite::{client::IntoClientRequest, Message, protocol::WebSocketConfig}};
use std::time::Duration;

pub const PCM_MAX: usize = 2 * 1024 * 1024;
const AUDIO_MAX: usize = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 4096;

fn decode(value: &str, max: usize) -> Result<Vec<u8>, &'static str> {
    if value.len() > max.div_ceil(3)*4 { return Err("invalid_input"); }
    let data = STANDARD.decode(value).map_err(|_| "invalid_input")?;
    if data.is_empty() || data.len() > max { return Err("invalid_input"); }
    Ok(data)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscribeRequest {
    #[serde(default)]
    pub allow_cloud: bool,
    pub wav_base64: String,
}

impl TranscribeRequest {
    pub fn wav(&self, enabled: bool) -> Result<Vec<u8>, &'static str> {
        if !enabled || !self.allow_cloud { return Err("cloud_denied"); }
        let data = decode(&self.wav_base64, PCM_MAX)?;
        if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return Err("invalid_input");
        }
        let declared = u32::from_le_bytes(data[4..8].try_into().map_err(|_| "invalid_input")?) as usize;
        if declared.checked_add(8) != Some(data.len()) { return Err("invalid_input"); }
        Ok(data)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceRequest {
    #[serde(default)]
    pub allow_cloud: bool,
    pub text: Option<String>,
    pub pcm16_base64: Option<String>,
}

impl VoiceRequest {
    pub fn item(&self, enabled: bool) -> Result<Value, &'static str> {
        if !enabled || !self.allow_cloud { return Err("cloud_denied"); }
        let content = match (&self.text, &self.pcm16_base64) {
            (Some(text), None) if !text.trim().is_empty() && text.len() <= 4096 =>
                json!({"type":"input_text","text":text}),
            (None, Some(audio)) => {
                let bytes = decode(audio, PCM_MAX)?;
                if bytes.len() % 2 != 0 { return Err("invalid_input"); }
                json!({"type":"input_audio","audio":audio})
            }
            _ => return Err("invalid_input"),
        };
        Ok(json!({"type":"conversation.item.create","item":{
            "type":"message","role":"user","content":[content]}}))
    }
}

#[derive(Default)]
pub struct AudioCollector {
    audio: Vec<u8>,
    count: usize,
    completed: bool,
}

impl AudioCollector {
    pub fn push(&mut self, event: &Value) -> Result<bool, &'static str> {
        self.count += 1;
        if self.completed || self.count > MAX_EVENTS { return Err("upstream_protocol"); }
        match event["type"].as_str() {
            Some("error") => Err("upstream_rejected"),
            Some("response.output_audio.delta") => {
                let delta = event["delta"].as_str().ok_or("upstream_protocol")?;
                let data = decode(delta, AUDIO_MAX.saturating_sub(self.audio.len())).map_err(|_| "upstream_protocol")?;
                self.audio.extend_from_slice(&data);
                Ok(false)
            }
            Some("response.done") => {
                if event["response"]["status"] != "completed" || self.audio.is_empty() || self.audio.len() % 2 != 0 {
                    return Err("upstream_incomplete");
                }
                self.completed = true;
                Ok(true)
            }
            Some(_) => Ok(false),
            None => Err("upstream_protocol"),
        }
    }

    pub fn finish(&self) -> Result<&[u8], &'static str> {
        if !self.completed { return Err("upstream_incomplete"); }
        Ok(&self.audio)
    }
}

pub async fn realtime(item: Value, key: &str) -> Result<Value, &'static str> {
    tokio::time::timeout(Duration::from_secs(90), async move {
        let mut ws = connect_realtime(key).await?;
        ws.send(Message::Text(item.to_string().into())).await.map_err(|_| "upstream_unavailable")?;
        ws.send(Message::Text(json!({"type":"response.create","response":{
            "output_modalities":["audio"],"max_output_tokens":1024,
            "tool_choice":"none"}}).to_string().into())).await.map_err(|_| "upstream_unavailable")?;
        let mut collector = AudioCollector::default();
        while let Some(msg) = ws.next().await {
            match msg.map_err(|_| "upstream_unavailable")? {
                Message::Text(s) => {
                    let event: Value = serde_json::from_str(&s).map_err(|_| "upstream_protocol")?;
                    if collector.push(&event)? {
                        let result = json!({"pcm16_base64":STANDARD.encode(collector.finish()?),
                            "sample_rate":24000,"channels":1,"format":"pcm_s16le","generated_by_ai":true});
                        let _ = ws.close(None).await;
                        return Ok(result);
                    }
                }
                Message::Close(_) => return Err("upstream_incomplete"),
                _ => (),
            }
        }
        Err("upstream_incomplete")
    }).await.map_err(|_| "upstream_timeout")?
}

pub(crate) async fn connect_realtime(key:&str)->Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,&'static str>{
    let mut request = "wss://api.openai.com/v1/realtime?model=gpt-realtime-2.1-mini"
        .into_client_request().map_err(|_| "upstream_unavailable")?;
    request.headers_mut().insert("Authorization", format!("Bearer {key}").parse().map_err(|_| "upstream_unavailable")?);
    let config = WebSocketConfig::default().max_message_size(Some(256*1024)).max_frame_size(Some(256*1024));
    let (mut ws, _) = connect_async_with_config(request, Some(config), false).await.map_err(|_| "upstream_unavailable")?;
    let session = json!({"type":"session.update","session":{"type":"realtime",
        "output_modalities":["audio"],"tools":[],
        "audio":{"input":{"format":{"type":"audio/pcm","rate":24000},"turn_detection":null},
                 "output":{"format":{"type":"audio/pcm","rate":24000},"voice":"marin"}}}});
    ws.send(Message::Text(session.to_string().into())).await.map_err(|_| "upstream_unavailable")?;
    let mut ready = false;
    for _ in 0..64 {
        let msg = ws.next().await.ok_or("upstream_incomplete")?.map_err(|_| "upstream_unavailable")?;
        if let Message::Text(s) = msg {
            let event: Value = serde_json::from_str(&s).map_err(|_| "upstream_protocol")?;
            if event["type"] == "error" { return Err("upstream_rejected"); }
            if event["type"] == "session.updated" { ready = true; break; }
        }
    }
    if !ready { return Err("upstream_protocol"); }
    Ok(ws)
}
