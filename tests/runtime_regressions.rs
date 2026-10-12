use ocaf_runtime::auth::{AuthContext, AuthError, AuthVerifier};
use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::{RegistryError, ToolManifest};
use ocaf_runtime::runtime::{OcafRuntime, RuntimeError, ToolCallRequest};
use serde_json::{json, Value};

struct Echo;
#[async_trait::async_trait]
impl ToolExecutor for Echo {
    async fn execute(&self, input: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        Ok(json!({"answer": input["message"]}))
    }
}

struct Fails;
#[async_trait::async_trait]
impl ToolExecutor for Fails {
    async fn execute(&self, _: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        Err(ToolError::Execution("expected test failure".into()))
    }
}

struct Hangs;
#[async_trait::async_trait]
impl ToolExecutor for Hangs {
    async fn execute(&self, _: Value, _: &ExecutionContext) -> Result<Value, ToolError> {
        std::future::pending::<()>().await;
        unreachable!()
    }
}

struct TestOnlyVerifier;
#[async_trait::async_trait]
impl AuthVerifier for TestOnlyVerifier {
    async fn verify(&self, _: &str, _: &AuthContext, _: u64, _: &[String]) -> Result<(), AuthError> {
        Ok(())
    }
}

fn manifest() -> ToolManifest {
    ToolManifest {
        name: "echo".into(),
        description: "test echo".into(),
        input_schema: json!({
            "type": "object",
            "properties": {"message": {"type": "string"}},
            "required": ["message"],
            "additionalProperties": false
        }),
        output_schema: json!({
            "type": "object",
            "properties": {"answer": {"type": "string"}},
            "required": ["answer"],
            "additionalProperties": false
        }),
        cost_sats: 0,
        permissions: vec![],
        timeout_ms: 100,
        enabled: true,
    }
}

fn request(auth: Option<AuthContext>, args: Value) -> ToolCallRequest {
    ToolCallRequest {
        call_id: "call-a".into(),
        tool: "echo".into(),
        args,
        auth,
    }
}

fn fake_auth() -> AuthContext {
    AuthContext {
        macaroon: "forged-but-nonempty".into(),
        preimage: "forged".into(),
        invoice: "fake-invoice".into(),
        nonce: "fake-nonce".into(),
    }
}

#[tokio::test]
async fn free_tool_accepts_missing_authentication() {
    let runtime = OcafRuntime::new(0);
    runtime.register(manifest(), Echo).await.unwrap();
    let output = runtime.execute(request(None, json!({"message":"ok"}))).await.unwrap();
    assert_eq!(output.output, json!({"answer":"ok"}));
    assert_eq!(runtime.metrics.snapshot().await.successful_calls, 1);
}

#[tokio::test]
async fn forged_nonempty_payment_fields_do_not_authorize_paid_tool() {
    let runtime = OcafRuntime::new(100);
    let mut paid = manifest();
    paid.cost_sats = 25;
    runtime.register(paid, Echo).await.unwrap();
    assert!(matches!(
        runtime.execute(request(Some(fake_auth()), json!({"message":"ok"}))).await,
        Err(RuntimeError::AuthRequired(_))
    ));
    assert_eq!(runtime.remaining_budget_sats(), 100);
    assert_eq!(runtime.metrics.snapshot().await.failed_calls, 1);
}

#[tokio::test]
async fn permissioned_tool_denied_without_authorizer_even_if_free() {
    let runtime = OcafRuntime::new(0);
    let mut restricted = manifest();
    restricted.permissions.push("file:write".into());
    runtime.register(restricted, Echo).await.unwrap();
    assert!(matches!(
        runtime.execute(request(Some(fake_auth()), json!({"message":"ok"}))).await,
        Err(RuntimeError::AuthRequired(_))
    ));
}

#[tokio::test]
async fn disabled_tool_must_not_execute() {
    let runtime = OcafRuntime::new(0);
    let mut disabled = manifest();
    disabled.enabled = false;
    runtime.register(disabled, Echo).await.unwrap();
    assert!(matches!(
        runtime.execute(request(None, json!({"message":"ok"}))).await,
        Err(RuntimeError::ToolDisabled(_))
    ));
}

#[tokio::test]
async fn input_schema_rejects_wrong_type() {
    let runtime = OcafRuntime::new(0);
    runtime.register(manifest(), Echo).await.unwrap();
    assert!(matches!(
        runtime.execute(request(None, json!({"message": 3}))).await,
        Err(RuntimeError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn output_schema_rejects_nonconforming_result() {
    let runtime = OcafRuntime::new(0);
    let mut invalid_output = manifest();
    invalid_output.output_schema = json!({
        "type":"object","required":["missing"],"properties":{"missing":{"type":"integer"}}
    });
    runtime.register(invalid_output, Echo).await.unwrap();
    assert!(matches!(
        runtime.execute(request(None, json!({"message":"ok"}))).await,
        Err(RuntimeError::InvalidOutput(_))
    ));
}

#[tokio::test]
async fn executor_failure_updates_metrics() {
    let runtime = OcafRuntime::new(0);
    runtime.register(manifest(), Fails).await.unwrap();
    assert!(matches!(
        runtime.execute(request(None, json!({"message":"ok"}))).await,
        Err(RuntimeError::ToolExecution(_))
    ));
    let m = runtime.metrics.snapshot().await;
    assert_eq!((m.total_calls,m.failed_calls,m.successful_calls),(1,1,0));
}

#[tokio::test]
async fn budget_is_refunded_on_execution_error() {
    let runtime = OcafRuntime::new(25).with_auth_verifier(TestOnlyVerifier);
    let mut paid = manifest();
    paid.cost_sats = 25;
    runtime.register(paid, Fails).await.unwrap();
    assert!(matches!(
        runtime.execute(request(Some(fake_auth()), json!({"message":"ok"}))).await,
        Err(RuntimeError::ToolExecution(_))
    ));
    assert_eq!(runtime.remaining_budget_sats(), 25);
}

#[tokio::test]
async fn timed_out_tool_refunds_budget_and_records_failure() {
    let runtime = OcafRuntime::new(25).with_auth_verifier(TestOnlyVerifier);
    let mut paid = manifest();
    paid.cost_sats = 25;
    paid.timeout_ms = 10;
    runtime.register(paid, Hangs).await.unwrap();
    assert!(matches!(
        runtime.execute(request(Some(fake_auth()), json!({"message":"ok"}))).await,
        Err(RuntimeError::Timeout)
    ));
    assert_eq!(runtime.remaining_budget_sats(), 25);
    assert_eq!(runtime.metrics.snapshot().await.failed_calls, 1);
}

#[tokio::test]
async fn malformed_manifest_is_not_registered() {
    let runtime = OcafRuntime::new(0);
    let mut invalid = manifest();
    invalid.input_schema = json!({"type": "not-a-json-schema-type"});
    assert!(matches!(
        runtime.register(invalid, Echo).await,
        Err(RegistryError::InvalidSchema(_))
    ));
    assert!(runtime.registry.list().await.is_empty());
}

#[tokio::test]
async fn zero_timeout_manifest_is_rejected() {
    let runtime = OcafRuntime::new(0);
    let mut invalid = manifest();
    invalid.timeout_ms = 0;
    assert!(matches!(
        runtime.register(invalid, Echo).await,
        Err(RegistryError::InvalidManifest(_))
    ));
}

#[tokio::test]
async fn missing_tool_is_counted_as_failure() {
    let runtime = OcafRuntime::new(0);
    assert!(matches!(
        runtime.execute(request(None, json!({"message":"ok"}))).await,
        Err(RuntimeError::ToolNotFound(_))
    ));
    assert_eq!(runtime.metrics.snapshot().await.failed_calls, 1);
}
