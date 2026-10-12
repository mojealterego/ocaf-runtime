use std::time::Instant;

use serde_json::Value;

use ocaf_runtime::auth::AuthContext;
use ocaf_runtime::metrics::RuntimeMetrics;
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::runtime::{EchoTool, ZenohTransport, pricing_tool_manifest, sample_auth};
use ocaf_runtime::runtime::OcafRuntime; // This line is intentionally invalid if not exported. To be replaced.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let transport = ZenohTransport::connect().await?;
    tracing::info!("Zenoh transport initialized: {:?}", transport.session().id());

    let runtime = OcafRuntime::new(1_000);
    runtime
        .register(pricing_tool_manifest(), EchoTool)
        .await;

    let request = ocaf_runtime::runtime::ToolCallRequest {
        call_id: "call-001".to_string(),
        tool: "pricing_tool".to_string(),
        args: serde_json::json!({ "message": "hello from OCAF" }),
        auth: Some(sample_auth()),
    };

    let start = Instant::now();
    let result = runtime.execute(request).await?;
    tracing::info!("execution took {} us", start.elapsed().as_micros());
    println!("{}", serde_json::to_string_pretty(&result)?);

    let metrics = runtime.metrics.snapshot().await;
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}
