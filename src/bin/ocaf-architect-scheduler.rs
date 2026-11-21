//! OCAF Meta Agent Architect scheduler tick service (AB-OM-10 partial).
//!
//! The actual Tokio process periodically persists UTC due periods into SQLite.
//! It does not start agents, perform external research, or admit evolutions.
//! The calling service manager MUST protect the environment and database path.

use ocaf_runtime::{
    architect_research_schedule::{ArchitectResearchScheduler, ResearchError},
    architect_workflow_bridge::register_due,
    om_engine::OmEngine,
};
use std::{
    env,
    error::Error,
    io,
    path::PathBuf,
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::MissedTickBehavior;

struct OmConfig {
    db_path: PathBuf,
    pinned_base_commit_sha: String,
    max_cost_minor: u64,
}

struct Config {
    db_path: PathBuf,
    om: Option<OmConfig>,
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

fn optional(name: &'static str) -> Result<Option<String>, io::Error> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(invalid("invalid OCAF_ARCHITECT_* environment setting")),
    }
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
    let om_db = optional("OCAF_ARCHITECT_OM_DB")?;
    let base_sha = optional("OCAF_ARCHITECT_BASE_COMMIT_SHA")?;
    let budget = optional("OCAF_ARCHITECT_MAX_COST_MINOR")?;
    let om = match (om_db, base_sha, budget) {
        (None, None, None) => None,
        (Some(db), Some(sha), Some(budget)) => {
            let db_path = PathBuf::from(db);
            let max_cost_minor: u64 = budget
                .parse()
                .map_err(|_| invalid("invalid OM Engine research cost limit"))?;
            if !db_path.is_absolute()
                || db_path.file_name().is_none()
                || !valid_sha(&sha)
                || !(3..=1_000_000).contains(&max_cost_minor)
            {
                return Err(invalid("invalid OM Engine research policy"));
            }
            Some(OmConfig {
                db_path,
                pinned_base_commit_sha: sha,
                max_cost_minor,
            })
        }
        _ => {
            return Err(invalid(
                "OM Engine research configuration must be all-or-nothing",
            ))
        }
    };
    Ok(Config {
        db_path: path,
        om,
        tenant,
        fleet,
        interval_seconds,
        once,
    })
}

fn tick(
    scheduler: &ArchitectResearchScheduler,
    engine: Option<&OmEngine>,
    cfg: &Config,
) -> Result<(), Box<dyn Error>> {
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let now_unix = i64::try_from(epoch)?;
    let report = scheduler.tick(&cfg.tenant, &cfg.fleet, now_unix)?;
    let due = scheduler.list_due(&cfg.tenant, &cfg.fleet, now_unix, 64)?;
    // Contracts are registered in OM Engine before anyone claims a worker
    // lease. Registration is idempotent for a pinned policy and source SHA.
    // No models, research or external effects are executed here.
    let om_workflows = match (engine, cfg.om.as_ref()) {
        (Some(engine), Some(om)) => register_due(
            scheduler,
            engine,
            &cfg.tenant,
            &cfg.fleet,
            &om.pinned_base_commit_sha,
            om.max_cost_minor,
            now_unix,
            64,
        )?
        .len(),
        (None, None) => 0,
        _ => return Err(invalid("inconsistent OM Engine policy").into()),
    };
    // due_sample is a bounded sample, not a count of the whole backlog.
    println!(
        "ocaf_architect_scheduler from_unix={} through_unix={} new_runs={} due_sample={} om_workflows={}",
        report.from_unix, report.through_unix, report.new_runs, due.len(), om_workflows
    );
    Ok(())
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cfg = config()?; // Strict validation before opening or mutating SQLite.
    let scheduler = ArchitectResearchScheduler::open(&cfg.db_path)?;
    let engine = cfg
        .om
        .as_ref()
        .map(|om| OmEngine::open(&om.db_path))
        .transpose()?;
    if cfg.once {
        return tick(&scheduler, engine.as_ref(), &cfg);
    }
    let mut poll = tokio::time::interval(Duration::from_secs(cfg.interval_seconds));
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        #[cfg(unix)]
        tokio::select! {
            _ = poll.tick() => tick(&scheduler,engine.as_ref(),&cfg)?,
            _ = tokio::signal::ctrl_c() => break,
            _ = sigterm.recv() => break,
        }
        #[cfg(not(unix))]
        tokio::select! {
            _ = poll.tick() => tick(&scheduler,engine.as_ref(),&cfg)?,
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
