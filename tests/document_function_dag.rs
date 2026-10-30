use ocaf_runtime::document_function_dag::{
    AttributeKey, Driver, FunctionError, FunctionGraph, FunctionSpec,
};
use ocaf_runtime::document_state::{DocumentState, LabelPath, Mutation, StateError};
use ocaf_runtime::document_store::{
    Change, DocumentDriver, DocumentKey, Event, Head, SqliteDocumentDriver,
};
use std::collections::BTreeSet;
use uuid::Uuid;

fn key(id: u128) -> AttributeKey {
    AttributeKey {
        label: LabelPath::parse("0:1").unwrap(),
        guid: Uuid::from_u128(id),
    }
}
fn function(name: &str, inputs: &[u128], output: u128, driver: Driver) -> FunctionSpec {
    FunctionSpec {
        id: name.into(),
        inputs: inputs.iter().map(|id| key(*id)).collect(),
        output: key(output),
        driver,
    }
}
fn state(a: i64, b: i64) -> DocumentState {
    let mut state = DocumentState::default();
    state
        .commit(
            "inputs",
            &[
                Mutation::CreateLabel(key(1).label),
                Mutation::Attach {
                    label: key(1).label,
                    guid: key(1).guid,
                    payload: a.to_le_bytes().to_vec(),
                },
                Mutation::Attach {
                    label: key(2).label,
                    guid: key(2).guid,
                    payload: b.to_le_bytes().to_vec(),
                },
            ],
        )
        .unwrap();
    state
}

#[test]
fn out_of_order_dependencies_recompute_only_dirty_closure() {
    let graph = FunctionGraph::new(vec![
        function("copy", &[3], 4, Driver::Copy),
        function("sum", &[1, 2], 3, Driver::CheckedAddI64),
    ])
    .unwrap();
    assert!(graph
        .plan(&state(20, 22), &BTreeSet::new())
        .unwrap()
        .is_empty());
    let changes = graph
        .plan(&state(20, 22), &BTreeSet::from([key(1)]))
        .unwrap();
    assert_eq!(changes.len(), 2);
    assert!(matches!(&changes[0], Change::Attach { payload, guid, .. }
        if payload.as_slice() == 42_i64.to_le_bytes() && guid == &key(3).guid.to_string()));
    assert!(matches!(&changes[1], Change::Attach { payload, guid, .. }
        if payload.as_slice() == 42_i64.to_le_bytes() && guid == &key(4).guid.to_string()));
}

#[test]
fn cycles_duplicates_overflow_and_missing_inputs_are_rejected() {
    assert!(matches!(
        FunctionGraph::new(vec![
            function("a", &[2], 1, Driver::Copy),
            function("b", &[1], 2, Driver::Copy),
        ]),
        Err(FunctionError::Cycle)
    ));
    assert!(matches!(
        FunctionGraph::new(vec![
            function("one", &[1], 3, Driver::Copy),
            function("two", &[2], 3, Driver::Copy),
        ]),
        Err(FunctionError::Invalid)
    ));
    let sum = FunctionGraph::new(vec![function("sum", &[1, 2], 3, Driver::CheckedAddI64)]).unwrap();
    assert!(matches!(
        sum.plan(&state(i64::MAX, 1), &BTreeSet::from([key(1)])),
        Err(FunctionError::Value)
    ));
    let missing = FunctionGraph::new(vec![function("copy", &[99], 3, Driver::Copy)]).unwrap();
    assert!(matches!(
        missing.plan(&state(1, 2), &BTreeSet::from([key(99)])),
        Err(FunctionError::Missing)
    ));
}

#[test]
fn atomic_update_undo_redo_and_invalid_mutation() {
    let mut doc = state(3, 5);
    let graph =
        FunctionGraph::new(vec![function("sum", &[1, 2], 3, Driver::CheckedAddI64)]).unwrap();
    let prepared = graph.plan(&doc, &BTreeSet::from([key(1)])).unwrap();
    assert!(matches!(&prepared[0], Change::Attach { .. }));
    doc.commit(
        "computed",
        &[Mutation::Attach {
            label: key(3).label,
            guid: key(3).guid,
            payload: 8_i64.to_le_bytes().to_vec(),
        }],
    )
    .unwrap();
    let updated = graph.plan(&doc, &BTreeSet::from([key(2)])).unwrap();
    assert!(matches!(&updated[0], Change::Update { .. }));
    doc.commit(
        "new result",
        &[Mutation::Update {
            label: key(3).label,
            guid: key(3).guid,
            payload: 9_i64.to_le_bytes().to_vec(),
        }],
    )
    .unwrap();
    assert_eq!(
        doc.attribute(&key(3).label, key(3).guid),
        Some(9_i64.to_le_bytes().as_slice())
    );
    doc.undo().unwrap();
    assert_eq!(
        doc.attribute(&key(3).label, key(3).guid),
        Some(8_i64.to_le_bytes().as_slice())
    );
    doc.redo().unwrap();
    assert_eq!(
        doc.attribute(&key(3).label, key(3).guid),
        Some(9_i64.to_le_bytes().as_slice())
    );
    assert_eq!(
        doc.commit(
            "bad",
            &[Mutation::Update {
                label: key(3).label,
                guid: key(3).guid,
                payload: vec![0; 5000],
            }]
        ),
        Err(StateError::Limit)
    );
    assert_eq!(
        doc.attribute(&key(3).label, key(3).guid),
        Some(9_i64.to_le_bytes().as_slice())
    );
}

#[test]
fn durable_sqlite_replay_fences_stale_writers_and_isolates_tenants() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let store = SqliteDocumentDriver::open_existing(file.path()).unwrap();
    let tenant = DocumentKey::new("tenant-a", "dag").unwrap();
    let other = DocumentKey::new("tenant-b", "dag").unwrap();
    let initial = Event::Commit {
        name: "inputs".into(),
        changes: vec![
            Change::Create {
                label: "0:1".into(),
            },
            Change::Attach {
                label: "0:1".into(),
                guid: key(1).guid.to_string(),
                payload: 3_i64.to_le_bytes().to_vec(),
            },
            Change::Attach {
                label: "0:1".into(),
                guid: key(2).guid.to_string(),
                payload: 5_i64.to_le_bytes().to_vec(),
            },
        ],
    };
    let h1 = store.append(&tenant, &Head::default(), &initial).unwrap();
    let graph =
        FunctionGraph::new(vec![function("sum", &[1, 2], 3, Driver::CheckedAddI64)]).unwrap();
    let (state, _) = store.load(&tenant, Some(&h1)).unwrap();
    let changes = graph.plan(&state, &BTreeSet::from([key(1)])).unwrap();
    let h2 = store
        .append(
            &tenant,
            &h1,
            &Event::Commit {
                name: "compute".into(),
                changes,
            },
        )
        .unwrap();
    let reopened = SqliteDocumentDriver::open_existing(file.path()).unwrap();
    let (state, _) = reopened.load(&tenant, Some(&h2)).unwrap();
    assert_eq!(
        state.attribute(&key(3).label, key(3).guid),
        Some(8_i64.to_le_bytes().as_slice())
    );
    assert!(!reopened
        .load(&other, None)
        .unwrap()
        .0
        .contains_label(&key(1).label));
    let changes = graph.plan(&state, &BTreeSet::from([key(2)])).unwrap();
    assert!(matches!(changes[0], Change::Update { .. }));
    assert!(reopened
        .append(
            &tenant,
            &h1,
            &Event::Commit {
                name: "stale".into(),
                changes: changes.clone()
            }
        )
        .is_err());
    let h3 = reopened
        .append(
            &tenant,
            &h2,
            &Event::Commit {
                name: "update".into(),
                changes,
            },
        )
        .unwrap();
    assert_eq!(
        reopened
            .load(&tenant, Some(&h3))
            .unwrap()
            .0
            .attribute(&key(3).label, key(3).guid),
        Some(8_i64.to_le_bytes().as_slice())
    );
}
