//! Read-only, JSON-only operational probe. The exit code is nonzero for an
//! unavailable leader or database, making it suitable for a systemd timer.
//! Do not expose this process as an unauthenticated public HTTP endpoint.
use std::env;

use ocaf_runtime::cluster::observability::{AlertPolicy, ClusterHealth, PgClusterMonitor};

fn mandatory(name: &str) -> anyhow::Result<String> {
    let value = env::var(name)
        .map_err(|_| anyhow::anyhow!("missing environment setting: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty environment setting: {name}");
    }
    Ok(value)
}

fn threshold(name: &str, default: u64) -> anyhow::Result<u64> {
    match env::var(name) {
        Ok(value) => {
            let number = value
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("{name} must be an unsigned number"))?;
            if !(1..=1_000_000).contains(&number) {
                anyhow::bail!("{name} must be in the range 1..=1000000");
            }
            Ok(number)
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(_) => anyhow::bail!("{name} is not valid UTF-8"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dsn = mandatory("OCAF_CLUSTER_PG_DSN")?;
    let cluster = mandatory("OCAF_CLUSTER_ID")?;
    let insecure = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");
    let policy = AlertPolicy::new(
        threshold("OCAF_ALERT_PENDING_THRESHOLD", 100)?,
        threshold("OCAF_ALERT_UNCERTAIN_THRESHOLD", 1)?,
        threshold("OCAF_ALERT_EXPIRED_AGENTS_THRESHOLD", 1)?,
    )?;
    let probe = PgClusterMonitor::connect(&dsn, &cluster, insecure).await?;
    let report = probe.report(policy).await?;
    println!("{}", serde_json::to_string(&report)?);
    let fail_warning = env::var("OCAF_HEALTH_FAIL_ON_WARNING").as_deref() == Ok("1");
    if report.health == ClusterHealth::Critical
        || (fail_warning && report.health == ClusterHealth::Warning)
    {
        anyhow::bail!("OCAF cluster operational probe returned an unhealthy status");
    }
    Ok(())
}
