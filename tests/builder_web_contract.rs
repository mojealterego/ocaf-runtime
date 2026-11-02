//! Verify that the shared browser Code/NoCode fixture is accepted by the Rust compiler.
use ocaf_runtime::builder_core::{compile, from_dsl, to_dsl};

#[test]
fn browser_project_fixture_compiles_and_roundtrips_in_rust() {
    let source = include_str!("../apps/agents-builder-web/fixtures/hello.json");
    let project = from_dsl(source, "local").expect("browser fixture must match Rust schema");
    let plan = compile(&project, "local").expect("fixture compiles without effects");
    assert_eq!(plan.ordered_nodes, vec!["input", "output"]);
    assert!(plan.requested_tools.is_empty());
    assert_eq!(
        from_dsl(&to_dsl(&project, "local").unwrap(), "local").unwrap(),
        project
    );
}
