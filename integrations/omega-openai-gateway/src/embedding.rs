//! Operator-bound local embedding adapter; no URL or model selection by clients.
use serde::Deserialize;
use serde_json::{json,Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbedRequest { pub input:String }
impl EmbedRequest {
    pub fn payload(&self,model:&str)->Result<Value,&'static str>{
        if self.input.trim().is_empty() || self.input.len()>16384 {return Err("invalid_input");}
        Ok(json!({"model":model,"input":self.input,"encoding_format":"float"}))
    }
}
pub fn parse_embedding(value:&Value,model:&str,dim:usize)->Result<Vec<f32>,&'static str>{
    if value["model"]!=model || !(1..=256).contains(&dim) {return Err("upstream_protocol");}
    let data=value["data"].as_array().filter(|v|v.len()==1).ok_or("upstream_protocol")?;
    if data[0]["index"].as_u64()!=Some(0){return Err("upstream_protocol");}
    let array=data[0]["embedding"].as_array().filter(|v|v.len()==dim).ok_or("upstream_protocol")?;
    let vector:Vec<f32>=array.iter().map(|v|{
        let n=v.as_f64().ok_or("upstream_protocol")? as f32;
        if !n.is_finite(){return Err("upstream_protocol");}
        Ok(n)
    }).collect::<Result<_,_>>()?;
    let norm:f64=vector.iter().map(|v|f64::from(*v).powi(2)).sum();
    if !norm.is_finite() || norm<=1e-24 {return Err("upstream_protocol");}
    Ok(vector)
}
