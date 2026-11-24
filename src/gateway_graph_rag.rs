//! Local GraphRAG retrieval over the existing tenant-bound hybrid index.
//! Edges and embeddings are operator-admitted data, not extracted facts.
//! A snapshot is replaced atomically. Retrieved text is untrusted evidence,
//! not an instruction or authorization to execute tools or export data.
use crate::hybrid_retrieval::HybridIndex;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    #[error("tenant denied")]
    Denied,
    #[error("stale revision")]
    Conflict,
    #[error("invalid graph or embedding")]
    Invalid,
    #[error("resource limit")]
    Limit,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphSource {
    pub id: String,
    pub body: String,
    pub embedding: Vec<f32>,
    pub sha256: [u8; 32],
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GraphEvidence {
    pub source_id: String,
    pub body: String,
    pub sha256: [u8; 32],
    pub path: Vec<String>,
    pub revision: u64,
}
pub struct GraphRag {
    tenant: String,
    model: String,
    dim: usize,
    revision: u64,
    index: HybridIndex,
    sources: BTreeMap<String, GraphSource>,
    adjacency: BTreeMap<String, BTreeSet<String>>,
}
impl GraphRag {
    pub fn new(tenant: &str, model: &str, dim: usize) -> Result<Self, GraphError> {
        Ok(Self {
            tenant: tenant.into(), model: model.into(), dim, revision: 0,
            index: HybridIndex::new(tenant, model, dim).map_err(|_| GraphError::Invalid)?,
            sources: BTreeMap::new(), adjacency: BTreeMap::new(),
        })
    }
    pub fn revision(&self) -> u64 { self.revision }

    pub fn replace(&mut self, tenant: &str, expected: u64, sources: Vec<GraphSource>,
                   edges: Vec<GraphEdge>) -> Result<u64, GraphError> {
        if tenant != self.tenant { return Err(GraphError::Denied); }
        if expected != self.revision { return Err(GraphError::Conflict); }
        if sources.len() > 256 || edges.len() > 2048 { return Err(GraphError::Limit); }
        let revision = self.revision.checked_add(1).ok_or(GraphError::Limit)?;
        let mut index = HybridIndex::new(tenant, &self.model, self.dim).map_err(|_| GraphError::Invalid)?;
        let mut records = BTreeMap::new();
        for item in sources {
            if records.contains_key(&item.id) || item.body.trim().is_empty() {
                return Err(GraphError::Invalid);
            }
            let current = index.revision();
            index.upsert(tenant, &item.id, item.body.clone(), item.embedding.clone(),
                &self.model, item.sha256, current).map_err(|_| GraphError::Invalid)?;
            records.insert(item.id.clone(), item);
        }
        let mut adjacency: BTreeMap<String,BTreeSet<String>> = BTreeMap::new();
        for edge in edges {
            if edge.from == edge.to || !records.contains_key(&edge.from) || !records.contains_key(&edge.to)
                || !adjacency.entry(edge.from).or_default().insert(edge.to) {
                return Err(GraphError::Invalid);
            }
        }
        self.index = index;
        self.sources = records;
        self.adjacency = adjacency;
        self.revision = revision;
        Ok(revision)
    }

    /// Seed with hybrid RRF search and expand at most two directed hops.
    /// Stable order: seed ranking followed by BFS and lexicographic edge IDs.
    /// Byte budget covers source bodies, not JSON encoding or model tokens.
    pub fn retrieve(&self, tenant: &str, query: &str, vector: &[f32],
                    seeds: usize, limit: usize, body_bytes: usize) -> Result<Vec<GraphEvidence>, GraphError> {
        if tenant != self.tenant { return Err(GraphError::Denied); }
        if seeds == 0 || seeds > limit || limit > 8 || body_bytes == 0 || body_bytes > 16384 {
            return Err(GraphError::Limit);
        }
        let roots = self.index.search(tenant, query, vector, &self.model, seeds).map_err(|_| GraphError::Invalid)?;
        let mut queue = VecDeque::new();
        let mut visited = BTreeSet::new();
        for root in roots {
            if visited.insert(root.source_id.clone()) { queue.push_back(vec![root.source_id]); }
        }
        let mut result = Vec::new();
        let mut remaining = body_bytes;
        while let Some(path) = queue.pop_front() {
            let id = path.last().ok_or(GraphError::Invalid)?;
            let record = self.sources.get(id).ok_or(GraphError::Invalid)?;
            if record.body.len() <= remaining {
                remaining -= record.body.len();
                result.push(GraphEvidence { source_id: id.clone(), body: record.body.clone(),
                    sha256: record.sha256, path: path.clone(), revision: self.revision });
                if result.len() == limit { break; }
            }
            if path.len() < 3 {
                if let Some(neighbors) = self.adjacency.get(id) {
                    for neighbor in neighbors {
                        if visited.insert(neighbor.clone()) {
                            let mut next = path.clone();
                            next.push(neighbor.clone());
                            queue.push_back(next);
                        }
                    }
                }
            }
        }
        Ok(result)
    }
}
