use omega_openai_gateway::{admit, reserve, Model, Request, Tool};
use serde_json::json;

fn request() -> Request {
    serde_json::from_value(json!({"input":"hello"})).unwrap()
}

#[test]
fn defaults_to_local_without_tools() {
    let req = request();
    assert_eq!(req.model, Model::Local);
    assert_eq!(req.tool, Tool::None);
    assert!(admit(&req, false, false).is_ok());
}

#[test]
fn cloud_requires_request_consent_and_operator_enablement() {
    let mut req = request();
    req.model = Model::Astra;
    assert!(admit(&req, true, true).is_err());
    req.allow_cloud = true;
    assert!(admit(&req, false, true).is_err());
    assert!(admit(&req, true, false).is_ok());
}

#[test]
fn tools_require_separate_enablement_and_cloud() {
    let mut req = request();
    req.tool = Tool::Docs;
    assert!(admit(&req, true, true).is_err());
    req.model = Model::Sol;
    req.allow_cloud = true;
    assert!(admit(&req, true, false).is_err());
    assert!(admit(&req, true, true).is_ok());
}

#[test]
fn rejects_excess_and_unknown_fields() {
    let mut req = request();
    req.max_output_tokens = 4097;
    assert!(admit(&req, true, true).is_err());
    req.max_output_tokens = 64;
    req.input = " ".into();
    assert!(admit(&req, true, true).is_err());
    assert!(serde_json::from_value::<Request>(json!({"input":"x","server_url":"http://localhost"})).is_err());
}

#[test]
fn quota_survives_reopen_and_refuses_excess() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quota.sqlite");
    assert!(!reserve(&db, 0).unwrap());
    assert!(reserve(&db, 2).unwrap());
    assert!(reserve(&db, 2).unwrap());
    assert!(!reserve(&db, 2).unwrap());
    assert!(!reserve(&db, 1).unwrap());
}

#[test]
fn quota_concurrent_admission_is_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quota.sqlite");
    let threads: Vec<_> = (0..8).map(|_| {
        let db = db.clone();
        std::thread::spawn(move || reserve(&db, 3).unwrap())
    }).collect();
    let granted = threads.into_iter().map(|t| t.join().unwrap()).filter(|v| *v).count();
    assert_eq!(granted, 3);
}

#[test]
fn docs_tool_is_pinned_and_local_never_receives_cloud_fields() {
    let mut req = request();
    let local = req.payload("local-model");
    assert_eq!(local["model"], "local-model");
    assert!(local.get("tools").is_none());
    req.model = Model::Astra;
    req.tool = Tool::Docs;
    req.allow_cloud = true;
    let cloud = req.payload("ignored");
    assert_eq!(cloud["model"], "gpt-6.2-astra");
    assert_eq!(cloud["store"], false);
    assert_eq!(cloud["tools"][0]["server_url"], "https://developers.openai.com/mcp");
    assert_eq!(cloud["tools"][0]["allowed_tools"], json!(["search_openai_docs"]));
}

#[test]
fn multi_turn_preserves_original_text_and_roles() {
    let req: Request = serde_json::from_value(json!({
        "input":"  Dalszy ciąg — żółć.\n",
        "history":[{"role":"user","content":"Pierwsze pytanie."},
                   {"role":"assistant","content":"Odpowiedź.\n  "}]
    })).unwrap();
    assert!(admit(&req, false, false).is_ok());
    let local = req.payload("local");
    assert_eq!(local["messages"][1]["content"], "Odpowiedź.\n  ");
    assert_eq!(local["messages"][2]["content"], "  Dalszy ciąg — żółć.\n");
    assert_eq!(local["messages"][0]["role"], "user");
}

#[test]
fn cloud_history_requires_explicit_consent_and_preserves_roles() {
    let mut req: Request = serde_json::from_value(json!({
        "input":"next","model":"astra",
        "history":[{"role":"user","content":"first"},{"role":"assistant","content":"answer"}]
    })).unwrap();
    assert_eq!(admit(&req, true, false), Err("cloud_denied"));
    req.allow_cloud = true;
    assert!(admit(&req, true, false).is_ok());
    let cloud = req.payload("unused");
    assert_eq!(cloud["input"][1]["role"], "assistant");
    assert_eq!(cloud["input"][2]["content"], "next");
    assert_eq!(cloud["store"], false);
}

#[test]
fn history_rejects_invalid_roles_order_and_aggregate_size() {
    let invalid_role = json!({"input":"x","history":[{"role":"system","content":"x"}]});
    assert!(serde_json::from_value::<Request>(invalid_role).is_err());
    let wrong_order: Request = serde_json::from_value(json!({
        "input":"x","history":[{"role":"assistant","content":"x"},{"role":"user","content":"y"}]
    })).unwrap();
    assert_eq!(admit(&wrong_order, false, false), Err("invalid_input"));
    let oversized: Request = serde_json::from_value(json!({
        "input":"x","history":[{"role":"user","content":"a".repeat(8192)},
                              {"role":"assistant","content":"b".repeat(8192)}]
    })).unwrap();
    assert_eq!(admit(&oversized, false, false), Err("invalid_input"));
    let incomplete: Request = serde_json::from_value(json!({
        "input":"x","history":[{"role":"user","content":"y"}]
    })).unwrap();
    assert_eq!(admit(&incomplete, false, false), Err("invalid_input"));
}

#[test]
fn local_does_not_silently_switch_to_cloud() {
    let req: Request = serde_json::from_value(json!({"input":"tekst","allow_cloud":true})).unwrap();
    assert!(admit(&req, false, false).is_ok());
    assert_eq!(req.payload("configured-local")["model"], "configured-local");
    assert!(req.payload("configured-local").get("tools").is_none());
}
