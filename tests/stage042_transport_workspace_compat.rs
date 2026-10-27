//! The public root path must still enforce transport policy after extraction.
use ocaf_runtime::transport_policy::{validate_zenoh_config, TransportPolicyError};

#[test]
fn original_transport_path_still_rejects_plaintext_remote_endpoint() {
    let config = r#"{
      mode: "client",
      scouting: { multicast: { enabled: false }, gossip: { enabled: false } },
      transport: { link: { protocols: ["tcp"] } },
      connect: { endpoints: ["tcp/203.0.113.2:7447"] }
    }"#;
    assert_eq!(
        validate_zenoh_config(config, true),
        Err(TransportPolicyError::Unsafe)
    );
}

#[test]
fn extracted_crate_and_root_api_have_the_same_error_type() {
    let source = "not-json";
    let through_root = ocaf_runtime::transport_policy::validate_zenoh_config(source, false);
    let through_crate = ocaf_transport_crdt::validate_zenoh_config(source, false);
    assert_eq!(through_root, through_crate);
}
