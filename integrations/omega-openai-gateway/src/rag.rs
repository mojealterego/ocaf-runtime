//! Durable, single-operator local GraphRAG snapshot. No remote I/O.
use crate::gateway_graph_rag::{GraphRag, GraphSource, GraphEdge, GraphEvidence};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{path::{Path,PathBuf},time::Duration};

const TENANT: &str = "local-operator";
const MAX_SNAPSHOT: usize = 3 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum RagError {
    #[error("invalid graph snapshot or query")]
    Invalid,
    #[error("stale revision")]
    Conflict,
    #[error("storage unavailable")]
    Storage,
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub expected_revision: u64,
    pub sources: Vec<GraphSource>,
    pub edges: Vec<GraphEdge>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub query: String,
    pub embedding: Vec<f32>,
    pub seeds: usize,
    pub limit: usize,
    pub body_bytes: usize,
}
#[derive(Serialize)]
pub struct Evidence {
    pub revision: u64,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Clone)]
pub struct RagStore { path: PathBuf, model: String, dim: usize }
impl RagStore {
    pub fn open(path: &Path, model:&str, dim:usize) -> Result<Self,RagError> {
        if !path.is_absolute() { return Err(RagError::Invalid); }
        GraphRag::new(TENANT,model,dim).map_err(|_|RagError::Invalid)?;
        let store=Self{path:path.to_owned(),model:model.into(),dim};
        let conn=store.connection()?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS omega_rag_snapshot (
            id INTEGER PRIMARY KEY CHECK(id=1), model TEXT NOT NULL,
            dim INTEGER NOT NULL, revision INTEGER NOT NULL CHECK(revision>=0),
            payload BLOB NOT NULL CHECK(length(payload)<=3145728)
        )").map_err(|_|RagError::Storage)?;
        let empty=serde_json::to_vec(&Snapshot{expected_revision:0,sources:vec![],edges:vec![]})
            .map_err(|_|RagError::Invalid)?;
        conn.execute("INSERT OR IGNORE INTO omega_rag_snapshot(id,model,dim,revision,payload) VALUES(1,?1,?2,0,?3)",
            params![store.model,dim as i64,empty]).map_err(|_|RagError::Storage)?;
        store.read(&conn)?;
        Ok(store)
    }
    fn connection(&self) -> Result<Connection,RagError> {
        let conn=Connection::open(&self.path).map_err(|_|RagError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(|_|RagError::Storage)?;
        Ok(conn)
    }
    fn read(&self, conn:&Connection) -> Result<(u64,Snapshot),RagError> {
        let (model,dim,revision,len):(String,i64,i64,i64)=conn.query_row(
            "SELECT model,dim,revision,length(payload) FROM omega_rag_snapshot WHERE id=1",[],
            |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|RagError::Storage)?;
        if model!=self.model || dim!=self.dim as i64 || revision<0 || len<0 || len as usize>MAX_SNAPSHOT {
            return Err(RagError::Invalid);
        }
        // The caller opens a read or write transaction before this method
        // when concurrent replacement is possible.
        let payload:Vec<u8>=conn.query_row("SELECT payload FROM omega_rag_snapshot WHERE id=1",[],|r|r.get(0))
            .map_err(|_|RagError::Storage)?;
        if payload.len()!=len as usize {return Err(RagError::Storage);}
        let snapshot:Snapshot=serde_json::from_slice(&payload).map_err(|_|RagError::Invalid)?;
        Ok((revision as u64,snapshot))
    }
    fn graph(&self,snapshot:Snapshot)->Result<GraphRag,RagError>{
        let mut graph=GraphRag::new(TENANT,&self.model,self.dim).map_err(|_|RagError::Invalid)?;
        graph.replace(TENANT,0,snapshot.sources,snapshot.edges).map_err(|_|RagError::Invalid)?;
        Ok(graph)
    }
    pub fn replace(&self, snapshot:Snapshot)->Result<u64,RagError>{
        if snapshot.sources.len()>256 || snapshot.edges.len()>2048 {return Err(RagError::Invalid);}
        let expected=snapshot.expected_revision;
        let next=expected.checked_add(1).filter(|v|*v<=i64::MAX as u64).ok_or(RagError::Invalid)?;
        let data=serde_json::to_vec(&snapshot).map_err(|_|RagError::Invalid)?;
        if data.len()>MAX_SNAPSHOT {return Err(RagError::Invalid);}
        self.graph(snapshot)?;
        let mut conn=self.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|_|RagError::Storage)?;
        let (current,_)=self.read(&tx)?;
        if current!=expected {return Err(RagError::Conflict);}
        let count=tx.execute("UPDATE omega_rag_snapshot SET revision=?1,payload=?2 WHERE id=1 AND revision=?3",
            params![next as i64,data,expected as i64]).map_err(|_|RagError::Storage)?;
        if count!=1 {return Err(RagError::Conflict);}
        tx.commit().map_err(|_|RagError::Storage)?;
        Ok(next)
    }
    pub fn retrieve(&self,query:Query)->Result<Evidence,RagError>{
        let mut conn=self.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Deferred).map_err(|_|RagError::Storage)?;
        let (revision,snapshot)=self.read(&tx)?;
        tx.commit().map_err(|_|RagError::Storage)?;
        let graph=self.graph(snapshot)?;
        let mut evidence=graph.retrieve(TENANT,&query.query,&query.embedding,query.seeds,query.limit,query.body_bytes)
            .map_err(|_|RagError::Invalid)?;
        for item in &mut evidence {item.revision=revision;}
        Ok(Evidence{revision,evidence})
    }
}
