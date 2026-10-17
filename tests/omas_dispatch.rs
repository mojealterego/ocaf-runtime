use ocaf_runtime::executor::{ExecutionContext, ToolError, ToolExecutor};
use ocaf_runtime::omas_signed::SignedOperation;
use ocaf_runtime::registry::ToolManifest;
use ocaf_runtime::runtime::{OcafRuntime, ToolCallRequest};
use openssl::pkey::PKey;
use openssl::sign::Signer;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountExecutor(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl ToolExecutor for CountExecutor {
    async fn execute(&self, args: Value, _ctx: &ExecutionContext) -> Result<Value, ToolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(args)
    }
}

#[tokio::test]
async fn signed_dispatch_runs_once_and_rejects_replay_and_tampering() {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("nonce.sqlite");
    rusqlite::Connection::open(&database).unwrap().close().unwrap();

    let runtime = OcafRuntime::new(100)
        .with_tenant_namespace("tenant_a").unwrap()
        .with_omas_trusted_key(public);
    let counter = Arc::new(AtomicUsize::new(0));
    runtime.register(
        ToolManifest {
            name: "run".into(), description: "test".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            cost_sats: 0, permissions: vec![], timeout_ms: 1_000, enabled: true,
        },
        CountExecutor(counter.clone()),
    ).await.unwrap();
    let request = ToolCallRequest {
        call_id: "call_1".into(), tool: "run".into(),
        args: json!({"value": 42}), auth: None,
    };
    let mut signed = SignedOperation {
        tenant: "tenant_a".into(), namespace: "tenant_a/tools".into(),
        target: "tenant_a/tools/run".into(), opcode: 1, fuel: 100,
        issued_at: ocaf_runtime::omas_signed::unix_now().unwrap(),
        nonce: "nonce_for_dispatch_001".into(),
        payload: serde_json::to_vec(&request).unwrap(),
        signature: Vec::new(),
    };
    signed.signature = Signer::new_without_digest(&key).unwrap()
        .sign_oneshot_to_vec(&signed.canonical_bytes()).unwrap();

    let unconfigured = OcafRuntime::new(100)
        .with_tenant_namespace("tenant_a").unwrap();
    assert!(unconfigured.execute_signed_omas(&signed, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let wrong_key = PKey::generate_ed25519().unwrap();
    let wrong_public: [u8; 32] = wrong_key.raw_public_key().unwrap().try_into().unwrap();
    let untrusted = OcafRuntime::new(100)
        .with_tenant_namespace("tenant_a").unwrap()
        .with_omas_trusted_key(wrong_public);
    assert!(untrusted.execute_signed_omas(&signed, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    // An unauthorized key cannot consume the nonce, so the valid signer can still execute.
    let wrong_signer = PKey::generate_ed25519().unwrap();
    let mut forged = signed.clone();
    forged.signature = Signer::new_without_digest(&wrong_signer).unwrap()
        .sign_oneshot_to_vec(&forged.canonical_bytes()).unwrap();
    assert!(runtime.execute_signed_omas(&forged, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let mut altered = signed.clone();
    altered.opcode = 2;
    assert!(runtime.execute_signed_omas(&altered, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let mut wrong_tenant = signed.clone();
    wrong_tenant.tenant = "tenant_b".into();
    wrong_tenant.signature = Signer::new_without_digest(&key).unwrap()
        .sign_oneshot_to_vec(&wrong_tenant.canonical_bytes()).unwrap();
    assert!(runtime.execute_signed_omas(&wrong_tenant, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let mut broad_namespace = signed.clone();
    broad_namespace.namespace = "tenant_a".into();
    broad_namespace.signature = Signer::new_without_digest(&key).unwrap()
        .sign_oneshot_to_vec(&broad_namespace.canonical_bytes()).unwrap();
    assert!(runtime.execute_signed_omas(&broad_namespace, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let mut wrong_target = signed.clone();
    wrong_target.target = "tenant_a/tools/other".into();
    wrong_target.signature = Signer::new_without_digest(&key).unwrap()
        .sign_oneshot_to_vec(&wrong_target.canonical_bytes()).unwrap();
    assert!(runtime.execute_signed_omas(&wrong_target, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    let result = runtime.execute_signed_omas(&signed, &database, request.clone()).await.unwrap();
    assert_eq!(result.output, json!({"value":42}));
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    assert!(runtime.execute_signed_omas(&signed, &database, request.clone()).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    let changed_request = ToolCallRequest { call_id: "call_2".into(), ..request };
    assert!(runtime.execute_signed_omas(&signed, &database, changed_request).await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
