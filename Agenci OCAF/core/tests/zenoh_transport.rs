use ocaf_import_core::{AgentProfile, transport::*};

const PROFILE: AgentProfile = AgentProfile { id: "OCAF-AGENT-EXT-probe",
    source_id: "probe", name: "Probe", specialization: "Transport",
    source_ref: "test", instructions: "Probe only" };

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_zenoh_reply_and_wrong_payload() {
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config.insert_json5("scouting/multicast/enabled", "false").unwrap();
    config.insert_json5("scouting/gossip/enabled", "false").unwrap();
    config.insert_json5("listen/endpoints", "[\"tcp/127.0.0.1:0\"]").unwrap();
    let session = zenoh::open(config).await.unwrap();
    for (nonce, corrupt) in [("nonce_1", false), ("nonce_2", true)] {
        let key = probe_key(&PROFILE, nonce).unwrap();
        let queryable = session.declare_queryable(key.as_str()).await.unwrap();
        let reply = async {
            let query = queryable.recv_async().await.unwrap();
            let bytes = query.payload().unwrap().to_bytes();
            let payload = if corrupt { b"wrong".as_slice() } else { bytes.as_ref() };
            query.reply(query.key_expr().as_str(), payload).await.unwrap();
        };
        let (_, status) = tokio::join!(reply, staging_probe(&session, &PROFILE, nonce));
        assert_eq!(status.is_err(), corrupt);
    }
    assert!(probe_key(&PROFILE, "*").is_err());
}
