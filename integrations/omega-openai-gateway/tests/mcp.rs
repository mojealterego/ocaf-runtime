use omega_openai_gateway::mcp::{Profiles,StartRequest,ApprovalStore,ApprovalDecision};
use serde_json::json;
use std::time::{Instant,Duration};

fn profiles()->Profiles{
    Profiles::parse(r#"[{"id":"research","url":"https://mcp.example.com/mcp","tools":["search"]}]"#).unwrap()
}
fn start()->StartRequest{
    serde_json::from_value(json!({"profile":"research","model":"astra","input":"research","allow_cloud":true})).unwrap()
}
fn pending()->serde_json::Value{
    json!({"status":"completed","output":[{"type":"mcp_approval_request","id":"mcpr_1",
        "server_label":"research","name":"search","arguments":"{\"q\":\"rust\"}"}]})
}
#[test]
fn operator_profile_and_explicit_approval_are_mandatory(){
    let p=profiles();
    let req=start();
    assert!(req.payload(&p,false).is_err());
    let payload=req.payload(&p,true).unwrap();
    assert_eq!(payload["tools"][0]["require_approval"],"always");
    assert_eq!(payload["store"],false);
    assert!(Profiles::parse(r#"[{"id":"x","url":"http://127.0.0.1:80","tools":["x"]}]"#).is_err());
    assert!(Profiles::parse(r#"[{"id":"x","url":"https://user@host.example.com/mcp","tools":["x"]}]"#).is_err());
    assert!(serde_json::from_value::<StartRequest>(json!({"profile":"research","model":"astra","input":"x","url":"https://evil.example"})).is_err());
}
#[test]
fn approval_is_one_time_and_bound_to_server_output(){
    let p=profiles();let request=start().payload(&p,true).unwrap();
    let mut s=ApprovalStore::default();
    let now=Instant::now();
    let ticket=s.issue(&p,&request,&pending(),now).unwrap().unwrap();
    assert_eq!(ticket["name"],"search");
    let id=ticket["ticket"].as_str().unwrap();
    let next=s.consume(id,true,now).unwrap();
    let ApprovalDecision::Approved(next)=next else{panic!("expected approval")};
    assert_eq!(next["input"][2]["approval_request_id"],"mcpr_1");
    assert_eq!(next["input"][2]["approve"],true);
    assert!(s.consume(id,true,now).is_err());
}
#[test]
fn expired_denied_or_unlisted_call_cannot_execute(){
    let p=profiles();let request=start().payload(&p,true).unwrap();
    let mut s=ApprovalStore::default();let now=Instant::now();
    let ticket=s.issue(&p,&request,&pending(),now).unwrap().unwrap();
    assert!(s.consume(ticket["ticket"].as_str().unwrap(),true,now+Duration::from_secs(121)).is_err());
    let t=s.issue(&p,&request,&pending(),now).unwrap().unwrap();
    assert!(matches!(s.consume(t["ticket"].as_str().unwrap(),false,now).unwrap(),ApprovalDecision::Denied));
    let mut bad=pending();bad["output"][0]["name"]=json!("write");
    assert!(s.issue(&p,&request,&bad,now).is_err());
    let mut bad=pending();bad["output"].as_array_mut().unwrap().push(pending()["output"][0].clone());
    assert!(s.issue(&p,&request,&bad,now).is_err());
}
