use std::fs;
use std::process::Command;

use ocaf_agent_001_asterion::agent::{claim_next, parse_ledger, plan, snapshot, AgentError};
use ocaf_agent_001_asterion::security::AgentConfig;
use ocaf_agent_001_asterion::{StageStatus, GROUP_ID};
use serde_json::Value;

fn config() -> AgentConfig {
    AgentConfig::parse(include_str!("../config/agent.toml")).unwrap()
}

fn stages() -> &'static [u8] {
    br#"[
      {"stage_id":"10","dependencies":[],"status":"READY"},
      {"stage_id":"2","dependencies":[],"status":"READY"},
      {"stage_id":"3","dependencies":["2"],"status":"READY"},
      {"stage_id":"4","dependencies":[],"status":"BLOCKED"}
    ]"#
}

#[test]
fn local_plan_uses_numeric_order_dependencies_and_step_bound() {
    let mut config = config();
    config.max_steps = 1;
    let ledger = parse_ledger(&config, stages()).unwrap();
    let one = plan(&config, &ledger).unwrap();
    assert_eq!(one.candidates.len(), 1);
    assert_eq!(one.candidates[0].stage_id, "2");
    assert_eq!(one.ready_waiting_for_dependencies, 1);
    assert!(!one.exhausted);
    assert_eq!(ledger.get("2").unwrap().status, StageStatus::Ready);

    config.max_steps = 2;
    let two = plan(&config, &ledger).unwrap();
    assert_eq!(
        two.candidates
            .iter()
            .map(|c| c.stage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["2", "10"]
    );
    assert!(two.exhausted);
    assert_eq!(ledger.get("10").unwrap().status, StageStatus::Ready);
}

#[test]
fn claim_requires_approval_and_cannot_impersonate_other_group() {
    let config = config();
    let mut ledger = parse_ledger(&config, stages()).unwrap();
    assert!(matches!(
        claim_next(&config, &mut ledger, GROUP_ID, false),
        Err(AgentError::ApprovalRequired)
    ));
    assert!(matches!(
        claim_next(&config, &mut ledger, "G02", true),
        Err(AgentError::WrongGroup)
    ));
    assert_eq!(ledger.get("2").unwrap().status, StageStatus::Ready);
    assert_eq!(
        claim_next(&config, &mut ledger, GROUP_ID, true).unwrap(),
        "2"
    );
    assert_eq!(
        ledger.get("2").unwrap().claimed_by.as_deref(),
        Some(GROUP_ID)
    );
    assert_eq!(snapshot(&ledger).len(), 4);
}

#[test]
fn untrusted_ledger_is_validated_and_bounded() {
    let mut config = config();
    config.max_input_bytes = 8;
    assert!(matches!(
        parse_ledger(&config, stages()),
        Err(AgentError::InputTooLarge)
    ));
    config.max_input_bytes = 1_048_576;
    let invalid = br#"[{"stage_id":"1","dependencies":["9"],"status":"READY"}]"#;
    assert!(matches!(
        parse_ledger(&config, invalid),
        Err(AgentError::Ledger(_))
    ));
}

#[test]
fn cli_plan_and_claim_emit_json_without_overwriting_input() {
    let config_path = concat!(env!("CARGO_MANIFEST_DIR"), "/config/agent.toml");
    let path = std::env::temp_dir().join(format!(
        "ocaf-asterion-stages-{}-{}.json",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    fs::write(&path, stages()).unwrap();
    let binary = env!("CARGO_BIN_EXE_ocaf-agent-001");
    let plan_output = Command::new(binary)
        .args(["plan", config_path, path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(plan_output.status.success(), "{:?}", plan_output);
    let plan_json: Value = serde_json::from_slice(&plan_output.stdout).unwrap();
    assert_eq!(plan_json["candidates"][0]["stage_id"], "2");
    let denied = Command::new(binary)
        .args(["claim", config_path, path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!denied.status.success());
    let claim_output = Command::new(binary)
        .args(["claim", config_path, path.to_str().unwrap(), "--approve"])
        .output()
        .unwrap();
    assert!(claim_output.status.success(), "{:?}", claim_output);
    let claim_json: Value = serde_json::from_slice(&claim_output.stdout).unwrap();
    assert_eq!(claim_json["claimed_stage"], "2");
    assert_eq!(claim_json["persisted"], false);
    assert_eq!(fs::read(&path).unwrap().as_slice(), stages());
    fs::remove_file(path).unwrap();
}
