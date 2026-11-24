//! Operator-bound MCP profiles and one-shot human approval. No client URLs.
use crate::Model;
use serde::Deserialize;
use serde_json::{json,Value};
use std::{collections::{BTreeMap,BTreeSet},time::{Instant,Duration}};
use reqwest::Url;

fn identifier(s:&str)->bool{
    !s.is_empty() && s.len()<=64 && s.bytes().all(|b|b.is_ascii_alphanumeric()||b"_-.".contains(&b))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile { id:String,url:String,tools:Vec<String> }
#[derive(Default)]
pub struct Profiles(BTreeMap<String,Profile>);
impl Profiles {
    pub fn parse(raw:&str)->Result<Self,&'static str>{
        if raw.len()>32768{return Err("invalid_mcp_config");}
        let rows:Vec<Profile>=serde_json::from_str(raw).map_err(|_|"invalid_mcp_config")?;
        if rows.len()>16{return Err("invalid_mcp_config");}
        let mut map=BTreeMap::new();
        for p in rows{
            let url=Url::parse(&p.url).map_err(|_|"invalid_mcp_config")?;
            let host=url.domain().ok_or("invalid_mcp_config")?;
            let tools:BTreeSet<_>=p.tools.iter().collect();
            if !identifier(&p.id)||p.url.len()>2048||url.scheme()!="https"
                ||url.port_or_known_default()!=Some(443)||!url.username().is_empty()||url.password().is_some()
                ||url.query().is_some()||url.fragment().is_some()||!host.contains('.')
                ||[".local",".localhost",".internal",".test",".invalid",".onion"].iter().any(|s|host.ends_with(s))
                ||p.tools.is_empty()||p.tools.len()>32||tools.len()!=p.tools.len()
                ||p.tools.iter().any(|t|!identifier(t))||map.contains_key(&p.id){
                return Err("invalid_mcp_config");
            }
            map.insert(p.id.clone(),p);
        }
        Ok(Self(map))
    }
}
fn default_tokens()->u32{512}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRequest{
    pub profile:String,
    pub model:Model,
    pub input:String,
    #[serde(default)]
    pub allow_cloud:bool,
    #[serde(default="default_tokens")]
    pub max_output_tokens:u32,
}
impl StartRequest{
    pub fn payload(&self,profiles:&Profiles,enabled:bool)->Result<Value,&'static str>{
        if !enabled||!self.allow_cloud||!self.model.cloud(){return Err("cloud_denied");}
        if self.input.trim().is_empty()||self.input.len()>16384||!(16..=4096).contains(&self.max_output_tokens){
            return Err("invalid_input");
        }
        let p=profiles.0.get(&self.profile).ok_or("tool_denied")?;
        Ok(json!({"model":self.model.id(),"input":[{"role":"user","content":self.input}],
            "store":false,"stream":false,"max_output_tokens":self.max_output_tokens,
            "include":["reasoning.encrypted_content"],"max_tool_calls":1,
            "tools":[{"type":"mcp","server_label":p.id,"server_url":p.url,
                "allowed_tools":p.tools,"require_approval":"always"}]}))
    }
}
struct Pending { created:Instant,payload:Value,request_id:String }
#[derive(Default)]
pub struct ApprovalStore(BTreeMap<String,Pending>);
pub enum ApprovalDecision{Denied,Approved(Value)}
impl ApprovalStore{
    pub fn issue(&mut self,profiles:&Profiles,request:&Value,response:&Value,now:Instant)
        ->Result<Option<Value>,&'static str>{
        self.0.retain(|_,v|now.saturating_duration_since(v.created)<Duration::from_secs(120));
        if response["status"]!="completed"{return Err("upstream_incomplete");}
        let output=response["output"].as_array().filter(|x|!x.is_empty()&&x.len()<=64).ok_or("upstream_protocol")?;
        if response.to_string().len()>131072{return Err("upstream_body_rejected");}
        let label=request["tools"][0]["server_label"].as_str().ok_or("upstream_protocol")?;
        let profile=profiles.0.get(label).ok_or("tool_denied")?;
        let mut approval=None;
        for item in output{
            match item["type"].as_str(){
                Some("reasoning"|"message"|"mcp_list_tools")=>(),
                Some("mcp_approval_request")=>{
                    if approval.is_some()||item["server_label"]!=label{return Err("upstream_protocol");}
                    let id=item["id"].as_str().filter(|s|crate::advanced::valid_id(s,"mcpr_")).ok_or("upstream_protocol")?;
                    let name=item["name"].as_str().filter(|s|profile.tools.iter().any(|t|t==s)).ok_or("tool_denied")?;
                    let args=item["arguments"].as_str().filter(|s|s.len()<=16384).ok_or("upstream_protocol")?;
                    let parsed:Value=serde_json::from_str(args).map_err(|_|"upstream_protocol")?;
                    if !parsed.is_object(){return Err("upstream_protocol");}
                    approval=Some((id.to_owned(),name.to_owned(),parsed));
                }
                _=>return Err("upstream_protocol"),
            }
        }
        let Some((id,name,args))=approval else{return Ok(None);};
        if self.0.len()>=8{return Err("approval_capacity");}
        let mut bytes=[0u8;32];
        getrandom::fill(&mut bytes).map_err(|_|"approval_unavailable")?;
        let ticket=bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
        if self.0.contains_key(&ticket){return Err("approval_unavailable");}
        let mut next=request.clone();
        next["input"].as_array_mut().ok_or("upstream_protocol")?.extend(output.iter().cloned());
        self.0.insert(ticket.clone(),Pending{created:now,payload:next,request_id:id});
        Ok(Some(json!({"ticket":ticket,"server_label":label,"name":name,"arguments":args,"expires_in_seconds":120})))
    }
    pub fn consume(&mut self,ticket:&str,approve:bool,now:Instant)->Result<ApprovalDecision,&'static str>{
        if ticket.len()!=64||!ticket.bytes().all(|b|b.is_ascii_hexdigit()){return Err("invalid_approval");}
        let mut pending=self.0.remove(ticket).ok_or("invalid_approval")?;
        if now.saturating_duration_since(pending.created)>=Duration::from_secs(120){return Err("invalid_approval");}
        if !approve{return Ok(ApprovalDecision::Denied);}
        pending.payload["input"].as_array_mut().ok_or("invalid_approval")?.push(
            json!({"type":"mcp_approval_response","approval_request_id":pending.request_id,"approve":true}));
        Ok(ApprovalDecision::Approved(pending.payload))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequest{pub ticket:String,pub approve:bool}
