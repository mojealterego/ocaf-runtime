#![cfg(feature = "zenoh")]

// Regression for the reviewed Zenoh v1.10.1 JSON5-only migration.
// TLS and mTLS handshakes are tested by zenoh_mtls_e2e.
#[test]
fn zenoh_yaml_configuration_is_explicitly_disabled() {
    let dir = tempfile::tempdir().unwrap();
    for extension in ["yaml", "yml"] {
        let path = dir.path().join(format!("zenoh.{extension}"));
        std::fs::write(&path, "mode: peer\n").unwrap();
        assert!(zenoh::config::Config::from_file(&path).is_err());
    }
}

#[test]
fn zenoh_json5_configuration_stays_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zenoh.json5");
    std::fs::write(&path, r#"{ mode: "peer" }"#).unwrap();
    assert!(zenoh::config::Config::from_file(&path).is_ok());
}
