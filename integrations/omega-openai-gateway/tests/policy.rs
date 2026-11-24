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
