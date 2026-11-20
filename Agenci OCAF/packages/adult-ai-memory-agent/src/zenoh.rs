//! Real Zenoh staging probe; does not grant model/tool execution.
pub async fn probe(session: &::zenoh::Session, nonce: &str) -> Result<(), String> {
    ocaf_import_core::transport::staging_probe(session, &crate::PROFILE, nonce).await
}
