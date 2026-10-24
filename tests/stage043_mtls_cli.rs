//! Stage 0.43 executable fail-closed mTLS bootstrap contract.
use std::process::Command;

#[test]
fn mtls_server_refuses_start_without_operator_configuration() {
    let binary = env!("CARGO_BIN_EXE_ocaf-omas-mtls-server");
    let output = Command::new(binary)
        .env_remove("OCAF_OMAS_MTLS_BIND")
        .output()
        .expect("start OMAs mTLS executable");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("OCAF_OMAS_MTLS_BIND"));
}
