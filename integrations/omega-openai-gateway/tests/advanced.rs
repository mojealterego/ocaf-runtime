use omega_openai_gateway::advanced::{AdvancedRequest, ToolPolicy, reserve_budget, execute_function};
use serde_json::json;

fn request(v: serde_json::Value) -> AdvancedRequest {
    serde_json::from_value(v).unwrap()
}
#[test]
fn image_is_inline_and_requires_consent() {
    let mut r = request(json!({"model":"astra","input":"describe","image":"data:image/png;base64,iVBORw0KGgo="}));
    let policy = ToolPolicy::default();
    assert!(r.payload(true, &policy).is_err());
    r.allow_cloud = true;
    let p = r.payload(true, &policy).unwrap();
    assert_eq!(p["input"][0]["content"][1]["type"], "input_image");
    assert_eq!(p["store"], false);
    r.image = Some("https://example.com/image.png".into());
    assert!(r.payload(true, &policy).is_err());
}
#[test]
fn file_and_vector_store_are_operator_pinned() {
    let r = request(json!({"model":"sol","input":"summarize","allow_cloud":true,"file_id":"file-123","tool":"file_search"}));
    assert!(r.payload(true, &ToolPolicy::default()).is_err());
    let p = ToolPolicy { files: vec!["file-123".into()], vector_store: Some("vs_123".into()), ..Default::default() };
    let payload = r.payload(true, &p).unwrap();
    assert_eq!(payload["input"][0]["content"][1]["file_id"], "file-123");
    assert_eq!(payload["tools"][0]["vector_store_ids"][0], "vs_123");
    assert_eq!(payload["tools"][0]["max_num_results"], 5);
}
#[test]
fn hosted_python_and_function_require_operator_enablement() {
    for tool in ["code_interpreter","sha256"] {
        let r = request(json!({"model":"astra","input":"test","allow_cloud":true,"tool":tool}));
        assert!(r.payload(true, &ToolPolicy::default()).is_err());
        let p = ToolPolicy { code: true, function: true, ..Default::default() };
        let payload = r.payload(true, &p).unwrap();
        assert_eq!(payload["max_output_tokens"], 512);
        assert!(payload["tools"].is_array());
    }
}
#[test]
fn local_and_unknown_advanced_fields_are_denied() {
    let r = request(json!({"model":"local","input":"x","allow_cloud":true}));
    assert!(r.payload(true, &ToolPolicy::default()).is_err());
    assert!(serde_json::from_value::<AdvancedRequest>(json!({"model":"astra","input":"x","server_url":"http://localhost"})).is_err());
}
#[test]
fn function_execution_is_pure_and_allowlisted() {
    let p = ToolPolicy { function: true, ..Default::default() };
    let call = json!({"name":"sha256_text","arguments":"{\"text\":\"abc\"}"});
    assert_eq!(execute_function(&call, &p).unwrap()["sha256"], "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert!(execute_function(&call, &ToolPolicy::default()).is_err());
    assert!(execute_function(&json!({"name":"shell","arguments":"{}"}), &p).is_err());
    assert!(execute_function(&json!({"name":"sha256_text","arguments":"{\"text\":\"x\",\"path\":\"/etc/passwd\"}"}), &p).is_err());
}
#[test]
fn reservation_is_atomic_persistent_and_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quota.db");
    assert!(!reserve_budget(&db, 5, 0, 4).unwrap());
    assert!(reserve_budget(&db, 5, 10, 4).unwrap());
    assert!(reserve_budget(&db, 5, 10, 4).unwrap());
    assert!(!reserve_budget(&db, 5, 10, 4).unwrap());
    assert!(!reserve_budget(&db, 2, 100, 4).unwrap());
}
#[test]
fn legacy_unpriced_attempts_block_budget_migration_for_day() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quota.db");
    assert!(omega_openai_gateway::reserve(&db, 5).unwrap());
    assert!(!reserve_budget(&db, 5, 100, 4).unwrap());
}
#[test]
fn concurrent_budget_reservation_never_overshoots() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("quota.db");
    let workers: Vec<_> = (0..12).map(|_| {
        let path = db.clone();
        std::thread::spawn(move || reserve_budget(&path, 20, 10, 3).unwrap())
    }).collect();
    assert_eq!(workers.into_iter().map(|t| t.join().unwrap()).filter(|x| *x).count(), 3);
}

#[test]
fn stateless_function_continuation_binds_output_to_call() {
    use omega_openai_gateway::advanced::function_continuation;
    let p = ToolPolicy { function: true, ..Default::default() };
    let req = request(json!({"model":"astra","input":"hash abc","allow_cloud":true,"tool":"sha256"}));
    let payload = req.payload(true, &p).unwrap();
    let first = json!({"status":"completed","output":[
        {"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"opaque"},
        {"type":"function_call","name":"sha256_text","call_id":"call_1","arguments":"{\"text\":\"abc\"}"}
    ]});
    let next = function_continuation(&payload,&first,&p).unwrap().unwrap();
    assert_eq!(next["store"], false);
    assert_eq!(next["tool_choice"], "none");
    assert_eq!(next["input"][1]["type"], "reasoning");
    assert_eq!(next["input"][3]["call_id"], "call_1");
    assert!(next["input"][3]["output"].as_str().unwrap().contains("ba7816bf"));
}
#[test]
fn continuation_rejects_multiple_calls_and_unknown_tools() {
    use omega_openai_gateway::advanced::function_continuation;
    let p = ToolPolicy { function: true, ..Default::default() };
    let req = request(json!({"model":"astra","input":"hash","allow_cloud":true,"tool":"sha256"}));
    let payload = req.payload(true,&p).unwrap();
    let call = json!({"type":"function_call","name":"sha256_text","call_id":"call_1","arguments":"{\"text\":\"x\"}"});
    assert!(function_continuation(&payload,&json!({"status":"completed","output":[call.clone(),call]}),&p).is_err());
    assert!(function_continuation(&payload,&json!({"status":"completed","output":[{"type":"function_call","name":"shell","call_id":"call_2","arguments":"{}"}]}),&p).is_err());
    assert!(function_continuation(&payload,&json!({"status":"incomplete","output":[]}),&p).is_err());
}
#[test]
fn automatic_function_does_not_accept_streaming() {
    let p = ToolPolicy { function: true, ..Default::default() };
    let req = request(json!({"model":"astra","input":"hash","allow_cloud":true,"tool":"sha256","stream":true}));
    assert!(req.payload(true,&p).is_err());
}
