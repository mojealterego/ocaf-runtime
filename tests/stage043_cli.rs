//! Stage 0.43 real process smoke/negative tests for the distinct OMAs and OMAt binaries.
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn omas_standalone_reports_staged_only_and_denies_active() {
    let health = Command::new(env!("CARGO_BIN_EXE_ocaf-omas-server"))
        .arg("--health")
        .output()
        .expect("run OMAs binary");
    assert!(health.status.success());
    let data: Value = serde_json::from_slice(&health.stdout).unwrap();
    assert_eq!(data["service"], "OMAs");
    assert_eq!(data["active_admission"], "DENY");
    assert_eq!(data["ostc_enabled"], false);
}

#[test]
fn omat_standalone_cannot_claim_executor_readiness() {
    let status = Command::new(env!("CARGO_BIN_EXE_ocaf-omat-server"))
        .output()
        .expect("run OMAt binary");
    assert!(status.status.success());
    let data: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(data["service"], "OMAt");
    assert_eq!(data["ready"], false);
    assert_eq!(data["executor"], "fail_closed");
    assert_eq!(data["source_access"], "disabled");
}

#[test]
fn omas_json_line_transport_denies_replay_and_spoofed_active() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ocaf-omas-server"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn OMAs");
    let staged = json!({
        "request_id":"cli-001",
        "agent_id":"OCAF-AGENT-001",
        "tenant":"tenant_a",
        "manifest_digest":format!("sha256:{}", "a".repeat(64)),
        "declared_state":"STAGED",
        "readiness":false,
        "ostc_enabled":false,
        "source_access":"disabled"
    });
    let mut active = staged.clone();
    active["request_id"] = json!("cli-active");
    active["declared_state"] = json!("ACTIVE");
    active["readiness"] = json!(true);
    let pipe = child.stdin.as_mut().expect("stdin available");
    writeln!(pipe, "{staged}").unwrap();
    writeln!(pipe, "{staged}").unwrap();
    writeln!(pipe, "{active}").unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["admitted"], true);
    assert_eq!(responses[0]["lifecycle"], "STAGED");
    assert_eq!(responses[1]["admitted"], false);
    assert_eq!(responses[1]["reason"], "replay_request_id");
    assert_eq!(responses[2]["admitted"], false);
    assert_eq!(responses[2]["reason"], "active_denied_ostc_disabled");
}
