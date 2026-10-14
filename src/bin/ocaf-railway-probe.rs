//! Opt-in, bounded Railway trial smoke check for the actual OCAF core.
//! No public listener, credentials, payment flows, Wasmtime claims or production readiness.
use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::runtime::{OcafRuntime, RuntimeError, ToolCallRequest};
use serde_json::{json, Value};

struct FixedEcho;

#[async_trait::async_trait]
impl ToolExecutor for FixedEcho {
    async fn execute(
        &self,
        args: Value,
        _context: &ExecutionContext,
    ) -> Result<Value, ToolError> {
        Ok(json!({"ok": true, "echo": args["message"]}))
    }
}

fn request(call_id: &str, tool: &str, args: Value) -> ToolCallRequest {
    ToolCallRequest {
        call_id: call_id.to_owned(),
        tool: tool.to_owned(),
        args,
        auth: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let runtime = OcafRuntime::new(0);
    runtime
        .register(
            ToolManifest {
                name: "ocaf_trial_echo".into(),
                description: "Local unprivileged validation fixture".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {"message": {"type": "string"}},
                    "required": ["message"],
                    "additionalProperties": false
                }),
                output_schema: json!({
                    "type": "object",
                    "properties": {"ok": {"const": true}, "echo": {"type": "string"}},
                    "required": ["ok", "echo"],
                    "additionalProperties": false
                }),
                cost_sats: 0,
                permissions: Vec::new(),
                timeout_ms: 1_000,
                enabled: true,
            },
            FixedEcho,
        )
        .await?;
    let ok = runtime
        .execute(request("railway-valid", "ocaf_trial_echo", json!({"message": "hello"})))
        .await?;
    anyhow::ensure!(ok.output == json!({"ok": true, "echo": "hello"}), "good output mismatch");
    let invalid = runtime
        .execute(request("railway-invalid", "ocaf_trial_echo", json!({"message": 5})))
        .await;
    anyhow::ensure!(matches!(invalid, Err(RuntimeError::InvalidInput(_))), "input must fail closed");
    let missing = runtime.execute(request("railway-missing", "unknown_tool", json!({}))).await;
    anyhow::ensure!(matches!(missing, Err(RuntimeError::ToolNotFound(_))), "unknown tool must fail");
    let tenant = OcafRuntime::new(0).with_tenant_namespace("trial-tenant")?;
    anyhow::ensure!(tenant.with_tenant_namespace("invalid:tenant").is_err(), "invalid namespace must fail");
    println!(
        "{}",
        json!({
            "status": "PASS",
            "scope": "railway-trial-local-ocaf-runtime-only",
            "checks": ["registered-tool-execution", "input-schema-rejection",
                       "unknown-tool-rejection", "invalid-tenant-namespace-rejection"],
            "stage_038_complete": false,
            "systemd_host_accepted": false,
            "mtls_multi_host_accepted": false,
            "production_approved": false
        })
    );
    Ok(())
}
