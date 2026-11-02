use ocaf_runtime::builder_core::{
    compile, from_dsl, to_dsl, BuilderEdge, BuilderError, BuilderNode, BuilderProject, NodeKind,
    Port, PortType,
};

fn port(name: &str, kind: PortType) -> Port {
    Port {
        name: name.into(),
        kind,
    }
}

fn project() -> BuilderProject {
    BuilderProject {
        schema_version: 1,
        tenant_id: "tenant_a".into(),
        project_id: "project_01".into(),
        revision: 1,
        max_cost_minor: 25,
        grants: vec!["tool_search".into()],
        nodes: vec![
            BuilderNode {
                id: "input".into(),
                kind: NodeKind::Input,
                inputs: vec![],
                outputs: vec![port("text", PortType::Text)],
                tool_name: None,
                max_cost_minor: 0,
            },
            BuilderNode {
                id: "search".into(),
                kind: NodeKind::Tool,
                inputs: vec![port("query", PortType::Text)],
                outputs: vec![port("result", PortType::Json)],
                tool_name: Some("tool_search".into()),
                max_cost_minor: 10,
            },
            BuilderNode {
                id: "output".into(),
                kind: NodeKind::Output,
                inputs: vec![port("data", PortType::Json)],
                outputs: vec![],
                tool_name: None,
                max_cost_minor: 0,
            },
        ],
        edges: vec![
            BuilderEdge {
                from_node: "input".into(),
                from_port: "text".into(),
                to_node: "search".into(),
                to_port: "query".into(),
            },
            BuilderEdge {
                from_node: "search".into(),
                from_port: "result".into(),
                to_node: "output".into(),
                to_port: "data".into(),
            },
        ],
    }
}

#[test]
fn valid_typed_graph_compiles_without_executing_effects() {
    let plan = compile(&project(), "tenant_a").unwrap();
    assert_eq!(plan.ordered_nodes, vec!["input", "search", "output"]);
    assert_eq!(plan.requested_tools, vec!["tool_search"]);
    assert_eq!(plan.max_cost_minor, 25);
    assert_eq!(plan.project_sha256.len(), 64);
}

#[test]
fn code_json_dsl_roundtrip_is_canonical_and_deterministic() {
    let original = project();
    let dsl = to_dsl(&original, "tenant_a").unwrap();
    assert_eq!(from_dsl(&dsl, "tenant_a").unwrap(), original);
    let mut reordered = original.clone();
    reordered.nodes.reverse();
    reordered.edges.reverse();
    assert_eq!(to_dsl(&reordered, "tenant_a").unwrap(), dsl);
    assert_eq!(
        compile(&reordered, "tenant_a").unwrap().project_sha256,
        compile(&original, "tenant_a").unwrap().project_sha256
    );
}

#[test]
fn trusted_tenant_is_not_derived_from_untrusted_payload() {
    assert_eq!(compile(&project(), "tenant_b"), Err(BuilderError::Denied));
    let dsl = to_dsl(&project(), "tenant_a").unwrap();
    assert_eq!(from_dsl(&dsl, "tenant_b"), Err(BuilderError::Denied));
}

#[test]
fn unsupported_schema_and_unknown_json_fields_are_rejected() {
    let mut p = project();
    p.schema_version = 2;
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Invalid));
    let dsl = to_dsl(&project(), "tenant_a").unwrap();
    let changed = dsl.replacen("\"revision\": 1", "\"revision\": 1, \"unsafe\": true", 1);
    assert_eq!(
        from_dsl(&changed, "tenant_a"),
        Err(BuilderError::Serialization)
    );
}

#[test]
fn missing_or_unused_tool_permission_is_denied() {
    let mut p = project();
    p.grants.clear();
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Permission));
    p.grants = vec!["tool_search".into(), "tool_extra".into()];
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Permission));
}

#[test]
fn edge_type_mismatch_and_missing_ports_are_rejected() {
    let mut p = project();
    p.edges[0].to_port = "result".into();
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Graph));
    p = project();
    p.nodes[1].inputs[0].kind = PortType::Json;
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::TypeMismatch));
}

#[test]
fn cyclic_graph_and_duplicate_edges_are_rejected() {
    let mut p = project();
    p.nodes[0].kind = NodeKind::Transform;
    p.nodes[0].inputs = vec![port("feedback", PortType::Json)];
    p.edges.push(BuilderEdge {
        from_node: "search".into(),
        from_port: "result".into(),
        to_node: "input".into(),
        to_port: "feedback".into(),
    });
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Graph));
    p = project();
    p.edges.push(p.edges[0].clone());
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Graph));
}

#[test]
fn duplicate_ids_and_disconnected_nodes_are_rejected() {
    let mut p = project();
    p.nodes.push(p.nodes[0].clone());
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Invalid));
    p = project();
    p.nodes.push(BuilderNode {
        id: "orphan".into(),
        kind: NodeKind::Transform,
        inputs: vec![port("in", PortType::Text)],
        outputs: vec![port("out", PortType::Text)],
        tool_name: None,
        max_cost_minor: 0,
    });
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Graph));
}

#[test]
fn budget_overrun_and_overflow_fail_closed() {
    let mut p = project();
    p.max_cost_minor = 9;
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Budget));
    p.max_cost_minor = u64::MAX;
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Limit));
}

#[test]
fn fan_in_to_same_typed_port_is_rejected() {
    let mut p = project();
    p.edges.push(BuilderEdge {
        from_node: "input".into(),
        from_port: "text".into(),
        to_node: "search".into(),
        to_port: "query".into(),
    });
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Graph));
}

#[test]
fn malformed_and_oversized_dsl_are_rejected_before_execution() {
    assert_eq!(from_dsl("{", "tenant_a"), Err(BuilderError::Serialization));
    assert_eq!(
        from_dsl(&"x".repeat(131_073), "tenant_a"),
        Err(BuilderError::Limit)
    );
}

#[test]
fn project_revision_and_identifiers_are_validated() {
    let mut p = project();
    p.revision = 0;
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Invalid));
    p = project();
    p.project_id = "../escape".into();
    assert_eq!(compile(&p, "tenant_a"), Err(BuilderError::Invalid));
}
