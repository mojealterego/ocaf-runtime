use std::{path::Path, process::Command};
use tempfile::tempdir;

fn command(path: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-fetch"));
    cmd.arg("--once")
        .env("OCAF_SOURCE_TENANT", "tenant-a")
        .env("OCAF_SOURCE_HOSTNAME", "example.com")
        .env("OCAF_SOURCE_PINNED_IPV4", "127.0.0.1")
        .env("OCAF_SOURCE_PATH", "/sample")
        .env("OCAF_SOURCE_MAX_BYTES", "4096")
        .env("OCAF_SOURCE_TIMEOUT_SECONDS", "5")
        .env("OCAF_SOURCE_OUTPUT", path);
    cmd
}

#[test]
fn unsafe_loopback_route_is_rejected_before_network_or_file_mutation() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("snapshot.txt");
    let result = command(&output).output().unwrap();
    assert!(!result.status.success());
    assert!(!output.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unsafe or invalid source route"));
}

#[test]
fn newline_in_host_path_and_invalid_limits_are_rejected_early() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("snapshot.txt");
    for (field, value) in [
        ("OCAF_SOURCE_HOSTNAME", "example.com\r\nHost:internal"),
        ("OCAF_SOURCE_PATH", "/safe\r\nAuthorization: secret"),
        ("OCAF_SOURCE_MAX_BYTES", "1000000000"),
        ("OCAF_SOURCE_TIMEOUT_SECONDS", "0"),
    ] {
        let res = command(&output)
            .env("OCAF_SOURCE_PINNED_IPV4", "104.20.23.154")
            .env(field, value)
            .output()
            .unwrap();
        assert!(!res.status.success(), "{field}");
        assert!(!output.exists());
    }
}
