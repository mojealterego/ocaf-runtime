#[test]
fn local_g01_never_opens_an_unverified_zenoh_transport() {
    assert!(ocaf_g01_swarm::zenoh::transport_status().is_err());
}
