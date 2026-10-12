use std::time::Instant;

use ocaf_runtime::runtime::{OcafRuntime, ToolCallRequest};
use ocaf_runtime::transport::{EchoTool, ZenohTransport, pricing_tool_manifest, sample_auth};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let transport = ZenohTransport::connect().await?;
    tracing::info!("Zenoh transport initialized: {:?}", transport.session().id());

    let runtime = OcafRuntime::new(1_000);
    runtime.register(pricing_tool_manifest(), EchoTool).await;

    let request = ToolCallRequest {
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
