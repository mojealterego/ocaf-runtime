//! Audio-only WebRTC signalling. Media travels directly to OpenAI.
//! A durable singleton lease fences admissions until hangup is confirmed.
use rusqlite::{params,Connection,OptionalExtension};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{path::{Path,PathBuf},time::Duration};

pub const CALLS:&str="https://api.openai.com/v1/realtime/calls";
const MAX_SDP:usize=32768;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest{pub sdp:String,#[serde(default)]pub allow_cloud:bool}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopRequest{pub ticket:String}

pub fn validate_sdp(sdp:&str)->Result<(),&'static str>{
    if sdp.len()>MAX_SDP||!sdp.is_ascii()||!sdp.starts_with("v=0")
        ||sdp.bytes().any(|b|b<32&&!matches!(b,b'\r'|b'\n'|b'\t')){
        return Err("invalid_sdp");
    }
    let mut media=0;
    let(mut fingerprint,mut ice_user,mut ice_pwd)=(false,false,false);
    for (i,line) in sdp.lines().enumerate(){
        if i>=512||line.len()>2048{return Err("invalid_sdp");}
        if let Some(m)=line.strip_prefix("m="){
            let parts:Vec<&str>=m.split_ascii_whitespace().collect();
            if parts.len()<4||parts[0]!="audio"||parts[1].parse::<u16>().ok().filter(|n|*n>0).is_none()
                ||parts[2]!="UDP/TLS/RTP/SAVPF"{return Err("invalid_sdp");}
            media+=1;
        }
        if line.starts_with("a=fingerprint:sha-256 "){fingerprint=true;}
        if line.starts_with("a=ice-ufrag:"){ice_user=true;}
        if line.starts_with("a=ice-pwd:"){ice_pwd=true;}
        if line.starts_with("a=sctp-port:")||line.starts_with("a=sctpmap:"){return Err("invalid_sdp");}
    }
    if media!=1||!fingerprint||!ice_user||!ice_pwd{return Err("invalid_sdp");}
    Ok(())
}
fn safe_id(s:&str)->bool{
    !s.is_empty()&&s.len()<=128&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b"_-".contains(&b))
}
pub fn call_id(location:&str)->Result<&str,&'static str>{
    let id=location.strip_prefix("https://api.openai.com/v1/realtime/calls/")
        .or_else(||location.strip_prefix("/v1/realtime/calls/"))
        .or_else(||location.strip_prefix("/realtime/calls/")).ok_or("upstream_location")?;
    if !safe_id(id){return Err("upstream_location");}
    Ok(id)
}
pub fn session()->Value{
    json!({"type":"realtime","model":"gpt-realtime-2.1-mini",
        "output_modalities":["audio"],"max_output_tokens":1024,"tools":[],"tool_choice":"none",
        "audio":{"input":{"turn_detection":{"type":"server_vad"}},"output":{"voice":"marin"}}})
}
#[derive(Clone)]
pub struct CallStore{path:PathBuf}
pub struct CallRecord{pub ticket:String,pub call_id:Option<String>}
impl CallStore{
    pub fn open(path:&Path)->Result<Self,&'static str>{
        let s=Self{path:path.to_owned()};
        s.conn()?.execute_batch("CREATE TABLE IF NOT EXISTS omega_webrtc_lease(
            slot INTEGER PRIMARY KEY CHECK(slot=1), ticket TEXT NOT NULL, call_id TEXT
        )").map_err(|_|"webrtc_storage")?;
        Ok(s)
    }
    fn conn(&self)->Result<Connection,&'static str>{
        let c=Connection::open(&self.path).map_err(|_|"webrtc_storage")?;
        c.busy_timeout(Duration::from_secs(2)).map_err(|_|"webrtc_storage")?;
        Ok(c)
    }
    pub fn begin(&self)->Result<String,&'static str>{
        let mut random=[0u8;32];
        getrandom::fill(&mut random).map_err(|_|"webrtc_storage")?;
        let ticket=random.iter().map(|b|format!("{b:02x}")).collect::<String>();
        let n=self.conn()?.execute("INSERT OR IGNORE INTO omega_webrtc_lease(slot,ticket,call_id)VALUES(1,?1,NULL)",
            [&ticket]).map_err(|_|"webrtc_storage")?;
        if n!=1{return Err("webrtc_lease_unresolved");}
        Ok(ticket)
    }
    pub fn bind(&self,ticket:&str,id:&str)->Result<(),&'static str>{
        if !safe_id(id){return Err("upstream_location");}
        let n=self.conn()?.execute("UPDATE omega_webrtc_lease SET call_id=?1 WHERE slot=1 AND ticket=?2 AND call_id IS NULL",
            params![id,ticket]).map_err(|_|"webrtc_storage")?;
        if n!=1{return Err("webrtc_lease_unresolved");}
        Ok(())
    }
    pub fn active(&self)->Result<Option<CallRecord>,&'static str>{
        self.conn()?.query_row("SELECT ticket,call_id FROM omega_webrtc_lease WHERE slot=1",[],
            |r|Ok(CallRecord{ticket:r.get(0)?,call_id:r.get(1)?})).optional().map_err(|_|"webrtc_storage")
    }
    pub fn clear(&self,ticket:&str)->Result<bool,&'static str>{
        self.conn()?.execute("DELETE FROM omega_webrtc_lease WHERE slot=1 AND ticket=?1",[ticket])
            .map(|n|n==1).map_err(|_|"webrtc_storage")
    }
}

pub async fn hangup(client:&reqwest::Client,key:&str,id:&str)->Result<(),&'static str>{
    if !safe_id(id){return Err("upstream_location");}
    let url=format!("{CALLS}/{id}/hangup");
    for attempt in 0..3{
        match client.post(&url).bearer_auth(key).timeout(Duration::from_secs(5)).send().await{
            Ok(r) if r.status().is_success()||r.status()==reqwest::StatusCode::NOT_FOUND=>return Ok(()),
            _=>(),
        }
        if attempt<2{tokio::time::sleep(Duration::from_millis(250*(attempt+1))).await;}
    }
    Err("webrtc_hangup_unconfirmed")
}

/// A missed or failed hangup is not interpreted as a closed call.
pub async fn close_record(client:&reqwest::Client,key:&str,store:&CallStore,ticket:&str)->Result<(),&'static str>{
    let Some(record)=store.active()? else{return Ok(());};
    if record.ticket!=ticket{return Err("webrtc_lease_mismatch");}
    let id=record.call_id.ok_or("webrtc_creation_unresolved")?;
    hangup(client,key,&id).await?;
    store.clear(ticket)?;
    Ok(())
}
