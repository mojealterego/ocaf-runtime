#[cfg(feature = "zenoh")]
use std::time::Instant;

#[cfg(feature = "zenoh")]
use ocaf_runtime::runtime::{OcafRuntime, ToolCallRequest};
#[cfg(feature = "zenoh")]
use ocaf_runtime::transport::{pricing_tool_manifest, EchoTool, ZenohTransport};

#[cfg(feature = "zenoh")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let transport = ZenohTransport::connect().await?;
    tracing::info!("Zenoh session initialized: {:?}", transport.session().zid());

    let runtime = OcafRuntime::new(1_000);
    runtime.register(pricing_tool_manifest(), EchoTool).await?;

    let request = ToolCallRequest {
        call_id: "call-001".to_string(),
        tool: "demo_echo".to_string(),
        args: serde_json::json!({"message": "hello from OCAF"}),
        auth: None,
    };

    let start = Instant::now();
    let result = runtime.execute(request).await?;
    tracing::info!("Local execution took {} us", start.elapsed().as_micros());
    println!("{}", serde_json::to_string_pretty(&result)?);

    let metrics = runtime.metrics.snapshot().await;
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

#[cfg(not(feature = "zenoh"))]
fn main() {
    eprintln!("OCAF core compiled successfully; enable --features zenoh for the example.");
}
