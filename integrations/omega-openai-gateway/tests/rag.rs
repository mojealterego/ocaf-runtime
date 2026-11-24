use omega_openai_gateway::rag::{RagStore, Snapshot, Query, RagError};
use omega_openai_gateway::gateway_graph_rag::{GraphSource, GraphEdge};
use sha2::{Digest,Sha256};

fn doc(id: &str, body: &str, embedding: Vec<f32>) -> GraphSource {
    GraphSource { id:id.into(),body:body.into(),embedding,
        sha256:Sha256::digest(body.as_bytes()).into() }
}
fn fixture(revision:u64) -> Snapshot {
    Snapshot { expected_revision:revision, sources:vec![doc("a","needle",vec![1.,0.]),doc("b","related",vec![0.,1.])],
        edges:vec![GraphEdge{from:"a".into(),to:"b".into()}] }
}
fn query() -> Query {
    Query { query:"needle".into(), embedding:vec![1.,0.],seeds:1,limit:2,body_bytes:100 }
}
#[test]
fn durable_snapshot_and_evidence() {
    let d=tempfile::tempdir().unwrap();
    let path=d.path().join("rag.sqlite");
    let store=RagStore::open(&path,"emb",2).unwrap();
    assert_eq!(store.replace(fixture(0)).unwrap(),1);
    drop(store);
    let store=RagStore::open(&path,"emb",2).unwrap();
    let result=store.retrieve(query()).unwrap();
    assert_eq!(result.revision,1);
    assert_eq!(result.evidence[1].path,vec!["a".to_owned(),"b".to_owned()]);
    assert_eq!(result.evidence[1].body,"related");
    assert_eq!(result.evidence[1].revision,1);
}
#[test]
fn stale_or_invalid_replacement_preserves_previous_snapshot() {
    let d=tempfile::tempdir().unwrap();
    let store=RagStore::open(&d.path().join("rag.sqlite"),"emb",2).unwrap();
    store.replace(fixture(0)).unwrap();
    assert!(matches!(store.replace(fixture(0)),Err(RagError::Conflict)));
    let mut bad=fixture(1);
    bad.sources[0].sha256=[0;32];
    assert!(store.replace(bad).is_err());
    assert_eq!(store.retrieve(query()).unwrap().revision,1);
}
#[test]
fn embedding_profile_cannot_change_silently() {
    let d=tempfile::tempdir().unwrap();
    let path=d.path().join("rag.sqlite");
    RagStore::open(&path,"emb",2).unwrap();
    assert!(RagStore::open(&path,"other",2).is_err());
    assert!(RagStore::open(&path,"emb",3).is_err());
}
#[test]
fn concurrent_compare_and_swap_has_single_winner() {
    let d=tempfile::tempdir().unwrap();
    let store=RagStore::open(&d.path().join("rag.sqlite"),"emb",2).unwrap();
    let handles:Vec<_>=(0..8).map(|_|{
        let store=store.clone();
        std::thread::spawn(move||store.replace(fixture(0)).is_ok())
    }).collect();
    assert_eq!(handles.into_iter().map(|h|h.join().unwrap()).filter(|x|*x).count(),1);
    assert_eq!(store.retrieve(query()).unwrap().revision,1);
}
