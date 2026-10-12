use ocaf_runtime::transport_policy::{validate_zenoh_config, TransportPolicyError};

fn loopback_config(protocol: &str, listen: &str) -> String {
    format!(r#"{{
        mode: "peer",
        listen: {{ endpoints: ["{protocol}/{listen}:7447"] }},
        scouting: {{ multicast: {{enabled:false}}, gossip: {{enabled:false}} }},
        transport: {{link:{{protocols:["{protocol}"]}}}}
    }}"#)
}

#[test]
fn plaintext_is_denied_by_default_and_cannot_bind_public_interfaces() {
    let local = loopback_config("tcp", "127.0.0.1");
    assert!(matches!(
        validate_zenoh_config(&local, false), Err(TransportPolicyError::Unsafe)
    ));
    assert!(validate_zenoh_config(&local, true).is_ok());
    let public = loopback_config("tcp", "0.0.0.0");
    assert!(matches!(
        validate_zenoh_config(&public, true), Err(TransportPolicyError::Unsafe)
    ));
    let external = loopback_config("tcp", "example.com");
    assert!(validate_zenoh_config(&external, true).is_err());
}

#[test]
fn implicit_scouting_and_unrestricted_protocols_are_rejected() {
    let implicit = r#"{mode:"client",connect:{endpoints:["tcp/127.0.0.1:7447"]}}"#;
    assert!(validate_zenoh_config(implicit, true).is_err());
    let mixed = r#"{
       mode:"client",connect:{endpoints:["tcp/127.0.0.1:7447"]},
       scouting:{multicast:{enabled:false},gossip:{enabled:false}},
       transport:{link:{protocols:["tcp","tls"]}}
    }"#;
    assert!(validate_zenoh_config(mixed, true).is_err());
    let external = loopback_config("tcp", "127.evil.org");
    assert!(validate_zenoh_config(&external, true).is_err());
}

#[test]
fn forged_tls_endpoint_without_mutual_auth_is_rejected() {
    let tls = loopback_config("tls", "127.0.0.1");
    assert!(validate_zenoh_config(&tls, false).is_err());
}

#[test]
fn secure_mtls_configuration_requires_complete_certificates_and_strict_validation() {
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let paths = ["ca.pem", "server-key.pem", "server-cert.pem"];
    for p in paths {
        fs::write(dir.path().join(p), "test-placeholder").unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.path().join("server-key.pem"),fs::Permissions::from_mode(0o600)).unwrap();
    }
    let ca=dir.path().join("ca.pem");
    let key=dir.path().join("server-key.pem");
    let cert=dir.path().join("server-cert.pem");
    let good = format!(r#"{{
        mode:"peer",
        listen:{{endpoints:["tls/127.0.0.1:7447"]}},
        scouting:{{multicast:{{enabled:false}},gossip:{{enabled:false}}}},
        transport:{{link:{{
            protocols:["tls"],
            tls:{{
                root_ca_certificate:"{}",
                listen_private_key:"{}",
                listen_certificate:"{}",
                enable_mtls:true,
                verify_name_on_connect:true,
                close_link_on_expiration:true
            }}
        }}}}
    }}"#,ca.display(),key.display(),cert.display());
    assert!(validate_zenoh_config(&good, false).is_ok());
    assert!(validate_zenoh_config(&good.replace("enable_mtls:true", "enable_mtls:false"), false).is_err());
    assert!(validate_zenoh_config(&good.replace("verify_name_on_connect:true", "verify_name_on_connect:false"), false).is_err());
    assert!(validate_zenoh_config(&good.replace("close_link_on_expiration:true", "close_link_on_expiration:false"), false).is_err());
    assert!(validate_zenoh_config(&good.replace("protocols:[\"tls\"]", "protocols:[\"tcp\"]"), false).is_err());
}

#[test]
fn malformed_json_and_excessive_config_size_are_rejected() {
    assert!(validate_zenoh_config("{missing", false).is_err());
    let big = "a".repeat(70_000);
    assert!(matches!(validate_zenoh_config(&big, false), Err(TransportPolicyError::Malformed)));
}
