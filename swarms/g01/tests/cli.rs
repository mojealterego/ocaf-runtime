use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ocaf-g01-swarm"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_runs_a_local_claim_operator_sign_and_artifact_handoff() {
    let dir = TempDir::new().unwrap();
    let path = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    let store = path("state");
    let public = path("operator.pub");
    let private = path("operator.key");
    let plan = path("plan.json");
    let claim = path("claim.json");
    let active = path("active.json");
    let artifact = path("artifact.bin");
    let payload_file = path("payload.json");
    let signed_file = path("signed.json");
    fs::write(
        &plan,
        r#"[{"stage_id":"900.1","dependencies":[],"status":"READY","claimed_by":null}]"#,
    )
    .unwrap();
    fs::write(&claim, r#"{"swarm_id":"OCAF-SWARM-01","declared_state":"IN_DEVELOPMENT","readiness":false,"ostc_enabled":false,"source_access":"disabled"}"#).unwrap();
    fs::write(&active, r#"{"swarm_id":"OCAF-SWARM-01","declared_state":"ACTIVE","readiness":true,"ostc_enabled":false,"source_access":"disabled"}"#).unwrap();
    fs::write(&artifact, b"actual compiled output").unwrap();

    assert!(invoke(&["keygen", &public, &private]).status.success());
    assert!(invoke(&["init", &store, &plan, &public]).status.success());
    let next = invoke(&["next", &store]);
    assert!(next.status.success());
    assert!(String::from_utf8(next.stdout).unwrap().contains("900.1"));
    assert!(!invoke(&["claim", &store, &active]).status.success());
    assert!(invoke(&["claim", &store, &claim]).status.success());
    let payload = invoke(&[
        "prepare",
        &store,
        &artifact,
        &"a".repeat(40),
        "cargo test",
        "PASS",
        "cli-receipt-1",
    ]);
    assert!(
        payload.status.success(),
        "{}",
        String::from_utf8_lossy(&payload.stderr)
    );
    fs::write(&payload_file, payload.stdout).unwrap();
    let signed = invoke(&["operator-sign", &store, &artifact, &payload_file, &private]);
    assert!(
        signed.status.success(),
        "{}",
        String::from_utf8_lossy(&signed.stderr)
    );
    fs::write(&signed_file, signed.stdout).unwrap();
    assert!(invoke(&["handoff", &store, &artifact, &signed_file])
        .status
        .success());
    let status = invoke(&["status", &store]);
    assert!(status.status.success());
    assert!(String::from_utf8(status.stdout)
        .unwrap()
        .contains("COMPLETE_VERIFIED"));
}
