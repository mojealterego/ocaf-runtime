use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::ExitCode;

use ocaf_agent_001_asterion::agent::{claim_next, parse_ledger, plan, snapshot};
use ocaf_agent_001_asterion::security::AgentConfig;
use serde_json::json;

fn read_bounded(path: &str, limit: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit as u64) + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("input exceeds configured byte limit".into());
    }
    Ok(bytes)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, config_path) = match args.as_slice() {
        [command, config_path] if command == "status" => (command.as_str(), config_path.as_str()),
        [command, config_path, _] if command == "plan" => {
            (command.as_str(), config_path.as_str())
        }
        [command, config_path, _, approval]
            if command == "claim" && approval == "--approve" =>
        {
            (command.as_str(), config_path.as_str())
        }
        _ => {
            return Err(
                "usage: ocaf-agent-001 status <agent.toml> | plan <agent.toml> <stages.json> | claim <agent.toml> <stages.json> --approve"
                    .into(),
            )
        }
    };
    let config_bytes = read_bounded(config_path, 65_536)?;
    let config = AgentConfig::parse(std::str::from_utf8(&config_bytes)?)?;
    if command == "status" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "agent_id": config.agent_id,
                "display_name": config.display_name,
                "lifecycle": config.lifecycle,
                "deployment_status": config.deployment_status,
                "model_backend": config.model_backend,
                "ostc_enabled": false,
                "source_access": "disabled",
                "active_admission": "DENY",
                "omas_endpoint": config.omas.endpoint,
                "omat_endpoint": config.omat.endpoint,
                "remote_transport": "disabled"
            }))?
        );
        return Ok(());
    }
    let stages_path = &args[2];
    let stages_bytes = read_bounded(stages_path, config.max_input_bytes)?;
    let mut ledger = parse_ledger(&config, &stages_bytes)?;
    if command == "plan" {
        println!(
            "{}",
            serde_json::to_string_pretty(&plan(&config, &ledger)?)?
        );
    } else {
        let claimed_stage = claim_next(&config, &mut ledger, "G01", true)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "claimed_stage": claimed_stage,
                "ledger": snapshot(&ledger),
                "persisted": false
            }))?
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Asterion denied: {error}");
            ExitCode::FAILURE
        }
    }
}
