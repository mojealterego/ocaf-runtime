use ocaf_ext_family_care_agent::{profile, zenoh::probe};
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn correlated_native_zenoh_probe() {
    let mut config = ::zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config.insert_json5("scouting/multicast/enabled", "false").unwrap();
    config.insert_json5("scouting/gossip/enabled", "false").unwrap();
    config.insert_json5("listen/endpoints", "[\"tcp/127.0.0.1:0\"]").unwrap();
    let session = ::zenoh::open(config).await.unwrap();
    let key = ocaf_import_core::transport::probe_key(&profile(), "nonce_1").unwrap();
    let queryable = session.declare_queryable(key.as_str()).await.unwrap();
    let reply = async {
        let query = tokio::time::timeout(std::time::Duration::from_secs(6), queryable.recv_async()).await.unwrap().unwrap();
        let bytes = query.payload().unwrap().to_bytes();
        query.reply(query.key_expr().as_str(), bytes.as_ref()).await.unwrap();
    };
    let (_, result) = tokio::join!(reply, probe(&session, "nonce_1"));
    assert!(result.is_ok(), "{result:?}");
    assert!(probe(&session, "*").await.is_err());
}
