//! Opt-in, non-production PostgreSQL leadership probe for physically separated hosts.
//! Does not execute remote tools, kill processes, or modify OCAF jobs/effects.
use std::env;
use std::io::{self, Write};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::ops::secrets::load_env_secret;
use serde_json::json;

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Hold,
    Attempt,
}

#[derive(Debug, PartialEq, Eq)]
enum Expectation {
    Acquired,
    Denied,
}

#[derive(Debug)]
struct Args {
    mode: Mode,
    hold_ms: Option<u64>,
    expect: Option<Expectation>,
    min_epoch: Option<i64>,
}

impl Args {
    fn parse(raw: &[String]) -> Result<Self> {
        let mut mode = None;
        let mut hold_ms = None;
        let mut expect = None;
        let mut min_epoch = None;
        let mut cursor = 0;
        while cursor < raw.len() {
            let flag = raw[cursor].as_str();
            let value = raw
                .get(cursor + 1)
                .context("each argument requires a value")?;
            match flag {
                "--mode" if mode.is_none() => {
                    mode = Some(match value.as_str() {
                        "hold" => Mode::Hold,
                        "attempt" => Mode::Attempt,
                        _ => bail!("--mode must be hold or attempt"),
                    });
                }
                "--hold-ms" if hold_ms.is_none() => {
                    let parsed = value.parse::<u64>().context("invalid --hold-ms")?;
                    if !(500..=30_000).contains(&parsed) {
                        bail!("--hold-ms must be 500..30000");
                    }
                    hold_ms = Some(parsed);
                }
                "--expect" if expect.is_none() => {
                    expect = Some(match value.as_str() {
                        "acquired" => Expectation::Acquired,
                        "denied" => Expectation::Denied,
                        _ => bail!("--expect must be acquired or denied"),
                    });
                }
                "--min-epoch" if min_epoch.is_none() => {
                    let parsed = value.parse::<i64>().context("invalid --min-epoch")?;
                    if parsed <= 0 {
                        bail!("--min-epoch must be positive");
                    }
                    min_epoch = Some(parsed);
                }
                _ => bail!("unknown or repeated probe option"),
            }
            cursor += 2;
        }
        let mode = mode.context("--mode is required")?;
        match mode {
            Mode::Hold if hold_ms.is_none() || expect.is_some() || min_epoch.is_some() => {
                bail!("hold requires --hold-ms and does not accept --expect or --min-epoch");
            }
            Mode::Attempt if hold_ms.is_some() || expect.is_none() => {
                bail!("attempt requires --expect and cannot use --hold-ms");
            }
            _ => {}
        }
        if min_epoch.is_some() && expect != Some(Expectation::Acquired) {
            bail!("--min-epoch requires --expect acquired");
        }
        Ok(Self {
            mode,
            hold_ms,
            expect,
            min_epoch,
        })
    }
}

fn test_cluster_id() -> Result<String> {
    let id =
        env::var("OCAF_HA_PROBE_CLUSTER_ID").context("OCAF_HA_PROBE_CLUSTER_ID is required")?;
    let Some(random_id) = id.strip_prefix("ocaf33_") else {
        bail!("only dedicated ocaf33_ test namespaces are permitted");
    };
    if random_id.len() != 32 || !random_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("the probe cluster must end in a 32-character hexadecimal test identifier");
    }
    Ok(id)
}

fn emit(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    io::stdout().flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse(&env::args().skip(1).collect::<Vec<_>>())?;
    if env::var("OCAF_HA_PROBE_ACK").as_deref() != Ok("isolated-test-only") {
        bail!("operator must explicitly authorize isolated test-only probes");
    }
    let cluster = test_cluster_id()?;
    // Never print the DSN or credential material. Use the same private-file
    // secret handling and PostgreSQL TLS policy as the production runtime.
    let dsn = load_env_secret("OCAF_CLUSTER_PG_DSN", 4096)?;
    let allow_loopback = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");
    let leader = PgLeader::connect(&dsn, &cluster, allow_loopback).await?;

    let acquired = leader.try_acquire().await?;
    match args.mode {
        Mode::Attempt => {
            let epoch = if acquired {
                let epoch = leader.fencing_epoch().await?;
                leader.release().await?;
                Some(epoch)
            } else {
                None
            };
            let outcome = if acquired { "acquired" } else { "denied" };
            let valid_expectation = matches!(
                (args.expect, acquired),
                (Some(Expectation::Acquired), true) | (Some(Expectation::Denied), false)
            );
            if !valid_expectation
                || args
                    .min_epoch
                    .is_some_and(|required| epoch < Some(required))
            {
                bail!("unexpected leadership result or non-monotonic fencing epoch");
            }
            emit(&json!({
                "schema_version": 1,
                "cluster_id": cluster,
                "outcome": outcome,
                "epoch": epoch,
                "verification": "postgres_advisory_lock",
            }))?;
        }
        Mode::Hold => {
            if !acquired {
                bail!("probe hold could not acquire leadership");
            }
            let epoch = leader.fencing_epoch().await?;
            emit(&json!({
                "schema_version": 1,
                "cluster_id": cluster,
                "outcome": "acquired",
                "epoch": epoch,
                "verification": "postgres_advisory_lock",
            }))?;
            let duration = Duration::from_millis(args.hold_ms.expect("parsed hold duration"));
            let mut elapsed = Duration::ZERO;
            while elapsed < duration {
                let interval = (duration - elapsed).min(Duration::from_millis(250));
                tokio::time::sleep(interval).await;
                leader.verify().await?;
                elapsed += interval;
            }
            leader.release().await?;
            emit(&json!({
                "schema_version": 1,
                "cluster_id": cluster,
                "outcome": "released",
                "epoch": epoch,
            }))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args> {
        Args::parse(&args.iter().map(|text| (*text).into()).collect::<Vec<_>>())
    }

    #[test]
    fn refuses_unbounded_and_invalid_holds() {
        assert!(parse(&["--mode", "hold", "--hold-ms", "0"]).is_err());
        assert!(parse(&["--mode", "hold", "--hold-ms", "30001"]).is_err());
        assert!(parse(&["--mode", "hold", "--hold-ms", "500"]).is_ok());
        assert!(parse(&["--mode", "hold", "--hold-ms", "500", "--expect", "acquired"]).is_err());
    }

    #[test]
    fn fail_closed_for_ambiguous_or_bypassed_expectations() {
        assert!(parse(&["--mode", "attempt"]).is_err());
        assert!(parse(&[
            "--mode",
            "attempt",
            "--expect",
            "denied",
            "--min-epoch",
            "10"
        ])
        .is_err());
        assert!(parse(&[
            "--mode",
            "attempt",
            "--expect",
            "acquired",
            "--min-epoch",
            "1"
        ])
        .is_ok());
        assert!(
            parse(&["--mode", "attempt", "--expect", "acquired", "--expect", "denied"]).is_err()
        );
        assert!(parse(&[
            "--mode",
            "attempt",
            "--expect",
            "acquired",
            "--unknown",
            "1"
        ])
        .is_err());
    }
}
