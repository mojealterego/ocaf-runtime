//! Bounded, multi-turn, bidirectional PCM WebSocket session.
//! Clients must disclose AI-generated speech; no tool execution or raw JSON relay.
use axum::extract::ws::{WebSocket,Message as ClientMessage};
use base64::{engine::general_purpose::STANDARD,Engine as _};
use futures_util::{SinkExt,StreamExt};
use serde_json::{json,Value};
use tokio_tungstenite::tungstenite::Message;
use std::time::Duration;

const MAX_INPUT:usize=2*1024*1024;
const MAX_OUTPUT:usize=4*1024*1024;
const MAX_CHUNK:usize=32768;
const MAX_EVENTS:usize=8192;
#[derive(Default)]
pub struct SessionBudget{
    input:usize,output:usize,pending:usize,turn_output:usize,turns:u8,busy:bool,
}
impl SessionBudget{
    pub fn append(&mut self,n:usize)->Result<(),&'static str>{
        if self.turns>=3||n==0||n%2!=0||n>MAX_CHUNK||n>MAX_INPUT.saturating_sub(self.input){
            return Err("audio_input_limit");
        }
        self.input+=n;self.pending+=n;Ok(())
    }
    pub fn commit(&mut self)->Result<(),&'static str>{
        if self.busy||self.turns>=3||self.pending<4800{return Err("invalid_turn");}
        self.pending=0;self.turn_output=0;self.turns+=1;self.busy=true;Ok(())
    }
    pub fn output(&mut self,n:usize)->Result<(),&'static str>{
        if !self.busy||n==0||n%2!=0||n>MAX_OUTPUT.saturating_sub(self.output){
            return Err("audio_output_limit");
        }
        self.output+=n;self.turn_output+=n;Ok(())
    }
    pub fn done(&mut self)->Result<(),&'static str>{
        if !self.busy||self.turn_output==0{return Err("upstream_incomplete");}
        self.busy=false;Ok(())
    }
}

async fn notice(client:&mut WebSocket,value:Value)->Result<(),&'static str>{
    client.send(ClientMessage::Text(value.to_string().into())).await.map_err(|_|"client_disconnected")
}

pub async fn serve(mut client:WebSocket,key:String){
    let result=tokio::time::timeout(Duration::from_secs(90),run(&mut client,&key)).await;
    match result{
        Ok(Ok(()))=>(),
        Ok(Err(code))=>{let _=tokio::time::timeout(Duration::from_secs(1),notice(&mut client,json!({"type":"error","code":code}))).await;}
        Err(_)=>{let _=tokio::time::timeout(Duration::from_secs(1),notice(&mut client,json!({"type":"error","code":"session_timeout"}))).await;}
    }
    let _=tokio::time::timeout(Duration::from_secs(1),client.send(ClientMessage::Close(None))).await;
}

async fn run(client:&mut WebSocket,key:&str)->Result<(),&'static str>{
    let mut ws=crate::voice::connect_realtime(key).await?;
    notice(client,json!({"type":"ready","sample_rate":24000,"channels":1,"format":"pcm_s16le","generated_by_ai":true,"max_turns":3})).await?;
    let mut budget=SessionBudget::default();
    for _ in 0..MAX_EVENTS{
        tokio::select!{
            event=client.next()=>{
                match event{
                    Some(Ok(ClientMessage::Binary(audio)))=>{
                        budget.append(audio.len())?;
                        ws.send(Message::Text(json!({"type":"input_audio_buffer.append","audio":STANDARD.encode(audio)}).to_string().into()))
                            .await.map_err(|_|"upstream_unavailable")?;
                    }
                    Some(Ok(ClientMessage::Text(s))) if s.as_str()=="commit"=>{
                        budget.commit()?;
                        ws.send(Message::Text(json!({"type":"input_audio_buffer.commit"}).to_string().into()))
                            .await.map_err(|_|"upstream_unavailable")?;
                        ws.send(Message::Text(json!({"type":"response.create","response":{
                            "output_modalities":["audio"],"max_output_tokens":1024,"tool_choice":"none"
                        }}).to_string().into())).await.map_err(|_|"upstream_unavailable")?;
                    }
                    Some(Ok(ClientMessage::Ping(_)|ClientMessage::Pong(_)))=>(),
                    Some(Ok(ClientMessage::Close(_)))|None=>return Ok(()),
                    _=>return Err("invalid_client_frame"),
                }
            }
            event=ws.next()=>{
                match event{
                    Some(Ok(Message::Text(s)))=>{
                        let value:Value=serde_json::from_str(&s).map_err(|_|"upstream_protocol")?;
                        match value["type"].as_str(){
                            Some("error")=>return Err("upstream_rejected"),
                            Some("response.output_audio.delta")=>{
                                let delta=value["delta"].as_str().ok_or("upstream_protocol")?;
                                let audio=STANDARD.decode(delta).map_err(|_|"upstream_protocol")?;
                                budget.output(audio.len())?;
                                client.send(ClientMessage::Binary(audio.into())).await.map_err(|_|"client_disconnected")?;
                            }
                            Some("response.done")=>{
                                if value["response"]["status"]!="completed"{return Err("upstream_incomplete");}
                                budget.done()?;
                                notice(client,json!({"type":"done","turn":budget.turns})).await?;
                                if budget.turns==3{return Ok(());}
                            }
                            Some(_)=>(),
                            None=>return Err("upstream_protocol"),
                        }
                    }
                    Some(Ok(Message::Ping(_)|Message::Pong(_)))=>(),
                    _=>return Err("upstream_incomplete"),
                }
            }
        }
    }
    Err("session_event_limit")
}
