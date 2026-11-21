//! OCAF Meta Agent Architect scheduler tick service (AB-OM-10 partial).
//!
//! The actual Tokio process periodically persists UTC due periods into SQLite.
//! It does not start agents, perform external research, or admit evolutions.
//! The calling service manager MUST protect the environment and database path.

use ocaf_runtime::architect_research_schedule::{ArchitectResearchScheduler, ResearchError};
use std::{
    env,
    error::Error,
    io,
    path::PathBuf,
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::MissedTickBehavior;

struct Config {
    db_path: PathBuf,
    tenant: String,
    fleet: String,
    interval_seconds: u64,
    once: bool,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn required(name: &'static str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| invalid("required OCAF_ARCHITECT_* environment setting missing"))
}

fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 96
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

fn config() -> Result<Config, io::Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    let once = match args.as_slice() {
        [] => false,
        [flag] if flag == "--once" => true,
        _ => return Err(invalid("unsupported scheduler arguments")),
    };
    let tenant = required("OCAF_ARCHITECT_TENANT")?;
    let fleet = required("OCAF_ARCHITECT_FLEET")?;
    if !valid_id(&tenant) || !valid_id(&fleet) {
        return Err(invalid("invalid tenant or fleet identity"));
    }
    let timezone = required("OCAF_ARCHITECT_TIMEZONE")?;
    if timezone != "UTC" {
        return Err(invalid(
            "only UTC is implemented; refusing implicit timezone conversion",
        ));
    }
    let path = PathBuf::from(required("OCAF_ARCHITECT_DB")?);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(invalid("database path must be an absolute path to a file"));
    }
    let interval_seconds: u64 = match env::var("OCAF_ARCHITECT_INTERVAL_SECONDS") {
        Ok(raw) => raw
            .parse()
            .map_err(|_| invalid("invalid scheduler interval"))?,
        Err(env::VarError::NotPresent) => 60,
        Err(_) => return Err(invalid("invalid scheduler interval")),
    };
    if !(10..=3600).contains(&interval_seconds) {
        return Err(invalid("scheduler interval must be 10..3600 seconds"));
    }
    Ok(Config {
        db_path: path,
        tenant,
        fleet,
        interval_seconds,
        once,
    })
}

fn tick(scheduler: &ArchitectResearchScheduler, cfg: &Config) -> Result<(), Box<dyn Error>> {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let now_unix = i64::try_from(epoch)?;
    let report = scheduler.tick(&cfg.tenant, &cfg.fleet, now_unix)?;
    let due = scheduler.list_due(&cfg.tenant, &cfg.fleet, now_unix, 64)?;
    // "due_sample" is deliberately bounded and is NOT a count of all work.
    println!(
        "ocaf_architect_scheduler from_unix={} through_unix={} new_runs={} due_sample={}",
        report.from_unix,
        report.through_unix,
        report.new_runs,
        due.len()
    );
    Ok(())
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cfg = config()?; // Strict validation before opening or mutating SQLite.
    let scheduler = ArchitectResearchScheduler::open(&cfg.db_path)?;
    if cfg.once {
        return tick(&scheduler, &cfg);
    }
    let mut poll = tokio::time::interval(Duration::from_secs(cfg.interval_seconds));
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        #[cfg(unix)]
        tokio::select! {
            _ = poll.tick() => tick(&scheduler,&cfg)?,
            _ = tokio::signal::ctrl_c() => break,
            _ = sigterm.recv() => break,
        }
        #[cfg(not(unix))]
        tokio::select! {
            _ = poll.tick() => tick(&scheduler,&cfg)?,
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        let category = if err.downcast_ref::<io::Error>().is_some()
            || err.downcast_ref::<ResearchError>().is_some()
        {
            "scheduler_error"
        } else {
            "runtime_error"
        };
        eprintln!("{category}: {err}");
        process::exit(1);
    }
}
