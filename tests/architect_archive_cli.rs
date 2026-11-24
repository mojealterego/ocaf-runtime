use std::{path::Path, process::Command};
use tempfile::tempdir;

fn base_fetch_command(output: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-fetch"));
    cmd.arg("--archive-only")
        .env("OCAF_SOURCE_TENANT", "tenant-a")
        .env("OCAF_SOURCE_HOSTNAME", "example.com")
        .env("OCAF_SOURCE_PINNED_IPV4", "104.20.23.154")
        .env("OCAF_SOURCE_PATH", "/")
        .env("OCAF_SOURCE_MAX_BYTES", "8192")
        .env("OCAF_SOURCE_TIMEOUT_SECONDS", "8")
        .env("OCAF_SOURCE_OUTPUT", output);
    cmd
}

#[test]
fn archive_only_requires_explicit_operator_archive_before_network() {
    let dir = tempdir().unwrap();
    let raw = dir.path().join("output.html");
    let archive = dir.path().join("source.sqlite");
    let result = base_fetch_command(&raw)
        .env_remove("OCAF_SOURCE_ARCHIVE_DB")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .to_lowercase()
            .contains("archive"),
        "should reject missing archive explicitly: {:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!raw.exists());
    assert!(!archive.exists());
}

#[test]
fn archive_only_invalid_archive_path_and_ssrf_are_rejected() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.html");
    let result = base_fetch_command(&output)
        .env("OCAF_SOURCE_ARCHIVE_DB", "relative/not-safe.db")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!output.exists());
    let result = base_fetch_command(&output)
        .env("OCAF_SOURCE_ARCHIVE_DB", dir.path().join("sources.db"))
        .env("OCAF_SOURCE_PINNED_IPV4", "127.0.0.1")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!dir.path().join("sources.db").exists());
}

#[test]
fn strict_evidence_mode_rejects_missing_archive_configuration() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("evidence.sqlite");
    let engine = dir.path().join("engine.sqlite");
    let result = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-evidence"))
        .arg("--submit-archived")
        .env("OCAF_EVIDENCE_DB", &db)
        .env("OCAF_EVIDENCE_OM_DB", &engine)
        .env("OCAF_EVIDENCE_TENANT", "tenant-a")
        .env_remove("OCAF_EVIDENCE_ARCHIVE_DB")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .to_lowercase()
        .contains("archive"));
    assert!(!db.exists());
    assert!(!engine.exists());
}

#[test]
fn evidence_cli_rejects_archive_hardlink_of_om_journal_before_processing() {
    let dir = tempdir().unwrap();
    let om = dir.path().join("om.db");
    let archive_alias = dir.path().join("source_alias.db");
    let proof = dir.path().join("evidence.db");
    ocaf_runtime::om_engine::OmEngine::open(&om).unwrap();
    std::fs::hard_link(&om, &archive_alias).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_ocaf-architect-evidence"))
        .arg("--submit-archived")
        .env("OCAF_EVIDENCE_DB", &proof)
        .env("OCAF_EVIDENCE_OM_DB", &om)
        .env("OCAF_EVIDENCE_ARCHIVE_DB", &archive_alias)
        .env("OCAF_EVIDENCE_TENANT", "tenant-a")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("distinct"),
        "invalid trust path should be rejected before signer keys: {:?}",
        String::from_utf8_lossy(&result.stderr)
    );
}
