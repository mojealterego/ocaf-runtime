//! Read-only operator diagnostics. No unprotected HTTP listener is opened.
//! Database credentials and transport TLS policy protect access to metrics.
use std::env;

use ocaf_runtime::cluster::observability::{
    AlertPolicy, ClusterHealth, ClusterSnapshot, PgClusterMonitor,
};

fn required(name: &str) -> anyhow::Result<String> {
    let value =
        env::var(name).map_err(|_| anyhow::anyhow!("missing required operator setting: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty operator setting: {name}");
    }
    Ok(value)
}

fn threshold(name: &str, default: u64) -> anyhow::Result<u64> {
    match env::var(name) {
        Ok(raw) => raw
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("{name} must be a positive integer")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(_) => anyhow::bail!("invalid value for {name}"),
    }
}

/// Prometheus/OpenMetrics-style numeric samples, printed to stdout for
/// a protected collector/textfile-exporter. No dynamic labels/identities.
fn prometheus(snapshot: &ClusterSnapshot, health: ClusterHealth) -> String {
    let measurements = [
        ("leader_sessions", snapshot.leader_sessions),
        ("agents_live", snapshot.agents_live),
        ("agents_expired", snapshot.agents_expired),
        ("jobs_pending", snapshot.jobs_pending),
        ("jobs_running", snapshot.jobs_running),
        ("jobs_succeeded", snapshot.jobs_succeeded),
        ("jobs_failed", snapshot.jobs_failed),
        ("jobs_uncertain", snapshot.jobs_uncertain),
        ("jobs_running_expired", snapshot.jobs_running_expired),
    ];
    let mut output = String::new();
    for (name, count) in measurements {
        output.push_str(&format!(
            "# TYPE ocaf_cluster_{name} gauge\nocaf_cluster_{name} {count}\n"
        ));
    }
    let health_level = match health {
        ClusterHealth::Healthy => 0,
        ClusterHealth::Warning => 1,
        ClusterHealth::Critical => 2,
    };
    output.push_str(&format!(
        "# TYPE ocaf_cluster_health_level gauge\nocaf_cluster_health_level {health_level}\n"
    ));
    output
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut format = "json";
    let mut no_fail = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" => format = "json",
            "--prometheus" => format = "prometheus",
            "--no-fail" => no_fail = true,
            _ => anyhow::bail!("unknown option; use --json, --prometheus or --no-fail"),
        }
    }

    let dsn = required("OCAF_CLUSTER_PG_DSN")?;
    let cluster = required("OCAF_CLUSTER_ID")?;
    let allow_loopback = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");
    let policy = AlertPolicy::new(
        threshold("OCAF_ALERT_PENDING_THRESHOLD", 100)?,
        threshold("OCAF_ALERT_UNCERTAIN_THRESHOLD", 1)?,
        threshold("OCAF_ALERT_EXPIRED_AGENTS_THRESHOLD", 1)?,
    )?;
    let monitor = PgClusterMonitor::connect(&dsn, &cluster, allow_loopback).await?;
    let report = policy.evaluate(monitor.snapshot().await?);

    match format {
        "prometheus" => print!("{}", prometheus(&report.snapshot, report.health)),
        _ => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    // Hooks for cron, systemd and external alert/metrics pipelines:
    // 0 healthy, 1 warning, 2 critical; --no-fail prints without alert exit.
    if !no_fail {
        let code = match report.health {
            ClusterHealth::Healthy => 0,
            ClusterHealth::Warning => 1,
            ClusterHealth::Critical => 2,
        };
        if code != 0 {
            std::process::exit(code);
        }
    }
    Ok(())
}
