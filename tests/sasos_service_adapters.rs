use ocaf_runtime::sasos_service_adapters::{
    SasosAdapterBindings, SasosAdapterError, SasosCatalog, SasosServiceBinding,
};
use serde_json::json;

fn catalog() -> SasosCatalog {
    SasosCatalog::from_json(include_str!("../omat/catalog/sasos_tools_001_555.json")).unwrap()
}
fn bindings() -> SasosAdapterBindings {
    SasosAdapterBindings::new(
        &catalog(),
        vec![SasosServiceBinding {
            sasos_tool_id: "SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR".into(),
            route_id: "before-sunset".into(),
            broker_tool_id: "calendar-read".into(),
            max_body_bytes: 4096,
        }],
    ).unwrap()
}
#[test]
fn all_555_catalog_entries_are_distinct_and_unactivated() {
    let cat = catalog();
    assert_eq!(cat.len(), 555);
    assert!(cat.contains("SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR"));
    assert!(!cat.contains("SASOS_TOOL_556_UNKNOWN"));
    assert!(cat.verified_active_count() == 0);
}
#[test]
fn adapter_requires_exact_operator_binding_and_declared_fields() {
    let b = bindings();
    assert_eq!(b.len(), 1);
    let ok = b.prepare_call(
        "SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR",
        &json!({"tasks_pointer":"opaque-task-id","time_horizon_hours":24}),
    ).unwrap();
    assert_eq!(ok.route_id, "before-sunset");
    assert_eq!(ok.broker_tool_id, "calendar-read");
    assert!(serde_json::from_slice::<serde_json::Value>(&ok.body).is_ok());
    assert!(matches!(
        b.prepare_call("SASOS_TOOL_005_ELEVENLABS_MCP_COORDINATOR", &json!({})),
        Err(SasosAdapterError::Unbound)
    ));
    for bad in [
        json!({"tasks_pointer":"id"}),
        json!({"tasks_pointer":"id","time_horizon_hours":24,"override":"admin"}),
        json!({"tasks_pointer":null,"time_horizon_hours":24}),
        json!(["id",24]),
    ] {
        assert!(matches!(
            b.prepare_call("SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR", &bad),
            Err(SasosAdapterError::InvalidArguments)
        ));
    }
}
#[test]
fn no_ring0_kernel_backend_without_ostc() {
    let r = SasosAdapterBindings::new(
        &catalog(),
        vec![SasosServiceBinding {
            sasos_tool_id: "SASOS_TOOL_002_ANTHROPIC_CLAUDE_OPUS_4_6".into(),
            route_id: "provider".into(),
            broker_tool_id: "model".into(),
            max_body_bytes: 4096,
        }],
    );
    assert!(matches!(r, Err(SasosAdapterError::PrivilegedTool)));
    let r = SasosAdapterBindings::new(
        &catalog(),
        vec![SasosServiceBinding {
            sasos_tool_id: "SASOS_TOOL_003_DEEPSEEK_CODER_V2_MOE".into(),
            route_id: "provider".into(),
            broker_tool_id: "model".into(),
            max_body_bytes: 4096,
        }],
    );
    assert!(matches!(r, Err(SasosAdapterError::PrivilegedTool)));
}
#[test]
fn reject_duplicate_binding_invalid_identifiers_and_invalid_catalog() {
    let binding = SasosServiceBinding {
        sasos_tool_id: "SASOS_TOOL_004_BEFORESUNSET_MCP_COORDINATOR".into(),
        route_id: "before-sunset".into(),
        broker_tool_id: "calendar-read".into(),
        max_body_bytes: 4096,
    };
    assert!(matches!(
        SasosAdapterBindings::new(&catalog(), vec![binding.clone(), binding.clone()]),
        Err(SasosAdapterError::InvalidConfiguration)
    ));
    let mut bad = binding.clone();
    bad.route_id = "http://127.0.0.1/".into();
    assert!(matches!(
        SasosAdapterBindings::new(&catalog(), vec![bad]),
        Err(SasosAdapterError::InvalidConfiguration)
    ));
    let mut bad = binding;
    bad.max_body_bytes = 0;
    assert!(matches!(
        SasosAdapterBindings::new(&catalog(), vec![bad]),
        Err(SasosAdapterError::InvalidConfiguration)
    ));
    assert!(SasosCatalog::from_json("[]").is_err());
}
