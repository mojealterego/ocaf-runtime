use ocaf_runtime::gateway_graph_rag::{GraphRag, GraphSource, GraphEdge, GraphError};
use sha2::{Digest, Sha256};

fn source(id: &str, body: &str, vector: [f32; 2]) -> GraphSource {
    GraphSource { id: id.into(), body: body.into(), embedding: vector.to_vec(),
        sha256: Sha256::digest(body.as_bytes()).into() }
}
fn edge(a: &str, b: &str) -> GraphEdge { GraphEdge { from: a.into(), to: b.into() } }

#[test]
fn linked_source_has_path_and_digest() {
    let mut g = GraphRag::new("a","emb",2).unwrap();
    g.replace("a",0,vec![source("a","needle",[1.,0.]),source("b","related",[0.,1.])],
        vec![edge("a","b")]).unwrap();
    let result = g.retrieve("a","needle",&[1.,0.],1,2,100).unwrap();
    assert_eq!(result.len(),2);
    assert_eq!(result[0].source_id,"a");
    assert_eq!(result[1].path,vec!["a".to_owned(),"b".to_owned()]);
    assert_eq!(result[1].sha256, <[u8;32]>::from(Sha256::digest(b"related")));
}
#[test]
fn snapshot_change_is_atomic_and_tenant_bound() {
    let mut g = GraphRag::new("a","emb",2).unwrap();
    g.replace("a",0,vec![source("a","old",[1.,0.])],vec![]).unwrap();
    assert_eq!(g.replace("b",1,vec![],vec![]),Err(GraphError::Denied));
    assert_eq!(g.replace("a",0,vec![],vec![]),Err(GraphError::Conflict));
    assert!(g.replace("a",1,vec![source("a","new",[1.,0.])],vec![edge("a","missing")]).is_err());
    assert_eq!(g.revision(),1);
    assert_eq!(g.retrieve("a","old",&[1.,0.],1,1,100).unwrap()[0].body,"old");
    assert_eq!(g.retrieve("b","old",&[1.,0.],1,1,100),Err(GraphError::Denied));
}
#[test]
fn cycles_duplicates_and_byte_limits_are_bounded() {
    let mut g = GraphRag::new("a","emb",2).unwrap();
    let docs = vec![source("a","needle",[1.,0.]),source("b","other",[0.,1.])];
    assert!(g.replace("a",0,docs.clone(),vec![edge("a","b"),edge("a","b")]).is_err());
    g.replace("a",0,docs,vec![edge("a","b"),edge("b","a")]).unwrap();
    let hits = g.retrieve("a","needle",&[1.,0.],1,8,6).unwrap();
    assert_eq!(hits.len(),1);
    assert!(g.retrieve("a","needle",&[1.,0.],1,9,100).is_err());
}
#[test]
fn bad_digest_and_duplicate_sources_never_publish() {
    let mut g = GraphRag::new("a","emb",2).unwrap();
    let mut bad = source("a","body",[1.,0.]);
    bad.sha256 = [0;32];
    assert!(g.replace("a",0,vec![bad],vec![]).is_err());
    let a=source("a","body",[1.,0.]);
    assert!(g.replace("a",0,vec![a.clone(),a],vec![]).is_err());
    assert_eq!(g.revision(),0);
}
