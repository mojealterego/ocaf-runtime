
use omega_openai_gateway::LocalChatRequest;
use serde_json::json;

#[test]
fn compatible_chat_preserves_operator_model_and_message_content() {
    let request: LocalChatRequest = serde_json::from_value(json!({
        "model":"omega-local",
        "messages":[{"role":"system","content":"Odpowiadaj po polsku."},
                    {"role":"user","content":"  Pierwsze pytanie.\n"},
                    {"role":"assistant","content":"Odpowiedź."},
                    {"role":"user","content":"Dalszy ciąg."}],
        "n":1,"temperature":0.2,"max_tokens":128
    })).unwrap();
    let payload = request.payload("configured-gguf").unwrap();
    assert_eq!(payload["model"], "configured-gguf");
    assert_eq!(payload["messages"][1]["content"], "  Pierwsze pytanie.\n");
    assert_eq!(payload["max_tokens"], 128);
    assert_eq!(payload["temperature"], 0.2);
}

#[test]
fn compatibility_cannot_select_cloud_or_inject_tools() {
    let req: LocalChatRequest = serde_json::from_value(json!({
        "model":"gpt-6.2-astra","messages":[{"role":"user","content":"x"}]
    })).unwrap();
    assert!(req.payload("configured-gguf").is_err());
    assert!(serde_json::from_value::<LocalChatRequest>(json!({
        "model":"omega-local","messages":[{"role":"user","content":"x"}],
        "tools":[{"type":"mcp","server_url":"http://localhost"}]
    })).is_err());
}

#[test]
fn compatible_chat_enforces_budget_and_turn_order() {
    for payload in [
        json!({"model":"omega-local","messages":[{"role":"user","content":"x"}],"n":2}),
        json!({"model":"omega-local","messages":[{"role":"user","content":"x"}],"max_tokens":4097}),
        json!({"model":"omega-local","messages":[{"role":"user","content":"x"}],"max_tokens":128,"max_completion_tokens":128}),
        json!({"model":"omega-local","messages":[{"role":"assistant","content":"x"}]}),
        json!({"model":"omega-local","messages":[{"role":"user","content":"x"},{"role":"system","content":"y"}]}),
        json!({"model":"omega-local","messages":[{"role":"user","content":"x".repeat(16385)}]}),
        json!({"model":"omega-local","messages":[{"role":"user","content":"x"}],"temperature":2.1})
    ] {
        let req: LocalChatRequest = serde_json::from_value(payload).unwrap();
        assert!(req.payload("configured-gguf").is_err());
    }
}
