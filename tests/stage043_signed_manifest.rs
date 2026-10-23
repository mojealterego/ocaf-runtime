//! Stage 0.43 cryptographically bound tool manifest admission.
//! Ed25519 signatures are verified before registry mutation.
use ocaf_runtime::omat_server::{ExecutorKind, OmatRegistry, ToolManifest, ToolRisk};
use ocaf_runtime::stage043_signed_manifest::{
    register_signed_manifest, SignedManifestError, SignedToolManifest,
};
use openssl::pkey::PKey;
use openssl::sign::Signer;
use serde_json::json;
use std::collections::BTreeSet;

const NOW: i64 = 1_800_000_000;

fn manifest(tenant: &str) -> ToolManifest {
    ToolManifest {
        tenant_id: tenant.into(),
        tool_id: "echo".into(),
        version: "v1".into(),
        description: "no side effects".into(),
        input_schema: json!({"type":"object", "properties":{"msg":{"type":"string"}}}),
        output_schema: json!({"type":"object"}),
        capabilities: BTreeSet::from(["read".into()]),
        risk: ToolRisk::ReadOnly,
        executor: ExecutorKind::Wasi,
        timeout_ms: 250,
        max_output_bytes: 4096,
        enabled: true,
    }
}

fn signed(tenant: &str) -> (SignedToolManifest, [u8; 32]) {
    let key = PKey::generate_ed25519().unwrap();
    let public: [u8; 32] = key.raw_public_key().unwrap().try_into().unwrap();
    let mut envelope = SignedToolManifest {
        manifest: manifest(tenant),
        signer_id: "trusted-operator".into(),
        issued_at: NOW - 1,
        expires_at: NOW + 60,
        signature: Vec::new(),
    };
    envelope.signature = Signer::new_without_digest(&key)
        .unwrap()
        .sign_oneshot_to_vec(&envelope.canonical_bytes().unwrap())
        .unwrap();
    (envelope, public)
}

#[test]
fn accepts_only_authenticated_tenant_scoped_manifest() {
    let mut registry = OmatRegistry::default();
    let (a, a_key) = signed("tenant_a");
    let (b, b_key) = signed("tenant_b");
    assert!(register_signed_manifest(&mut registry, &a, "tenant_a", &a_key, NOW).is_ok());
    assert!(register_signed_manifest(&mut registry, &b, "tenant_b", &b_key, NOW).is_ok());
    assert!(registry.get_for("tenant_a", "echo").is_some());
    assert!(registry.get_for("tenant_b", "echo").is_some());
    assert_eq!(registry.len(), 2);
}

#[test]
fn modified_tool_policy_or_schema_invalidates_signature_without_mutation() {
    let (valid, public) = signed("tenant_a");
    let mut registry = OmatRegistry::default();
    let mut changed = valid.clone();
    changed.manifest.risk = ToolRisk::ExternalSideEffect;
    assert!(matches!(
        register_signed_manifest(&mut registry, &changed, "tenant_a", &public, NOW),
        Err(SignedManifestError::Signature)
    ));
    let mut changed = valid.clone();
    changed.manifest.input_schema = json!({"type":"string"});
    assert!(matches!(
        register_signed_manifest(&mut registry, &changed, "tenant_a", &public, NOW),
        Err(SignedManifestError::Signature)
    ));
    assert!(registry.is_empty());
}

#[test]
fn mismatched_tenant_or_untrusted_signer_key_is_rejected() {
    let (valid, public) = signed("tenant_a");
    let (_other, other_key) = signed("tenant_b");
    let mut registry = OmatRegistry::default();
    assert!(matches!(
        register_signed_manifest(&mut registry, &valid, "tenant_b", &public, NOW),
        Err(SignedManifestError::TenantMismatch)
    ));
    assert!(matches!(
        register_signed_manifest(&mut registry, &valid, "tenant_a", &other_key, NOW),
        Err(SignedManifestError::Signature)
    ));
    assert!(registry.is_empty());
}

#[test]
fn expired_or_unbounded_manifest_is_rejected() {
    let (valid, public) = signed("tenant_a");
    let mut registry = OmatRegistry::default();
    assert!(matches!(
        register_signed_manifest(&mut registry, &valid, "tenant_a", &public, NOW + 61),
        Err(SignedManifestError::TimeWindow)
    ));
    let mut unbounded = valid.clone();
    unbounded.expires_at = NOW + 301;
    assert!(matches!(
        register_signed_manifest(&mut registry, &unbounded, "tenant_a", &public, NOW),
        Err(SignedManifestError::TimeWindow)
    ));
    assert!(registry.is_empty());
}
