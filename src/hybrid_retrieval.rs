//! SOURCE-1.65: bounded tenant-scoped lexical/vector fusion with revision fencing.
//!
//! This is a real in-memory Rust index using the existing BM25 implementation.
//! It accepts *operator-admitted* embeddings and verifies byte-to-vector binding
//! through a SHA-256 of the source text. A matching digest does NOT authenticate
//! the model or provider, and this module does not perform external ingestion,
//! signing, vector generation, durable persistence or network access.
use crate::retrieval_quality::{LexicalIndex, SourceDocument};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

const MAX_DOCUMENTS: usize = 256;
const MAX_DIMENSION: usize = 256;
const MAX_BODY_BYTES: usize = 65_536;
const MAX_TOTAL_BYTES: usize = 2 * 1024 * 1024;
const MAX_RESULTS: usize = 100;
const MIN_NORM_SQUARED: f64 = 1e-24;
const RRF_K: f64 = 60.0;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HybridError {
    #[error("invalid, unauthenticated or mismatched tenant / embedding identity")]
    Invalid,
    #[error("revision mismatch or missing record")]
    Conflict,
    #[error("source and embedding digest mismatch")]
    SourceDigestMismatch,
    #[error("index or query resource bound exceeded")]
    Limit,
    #[error("lexical index refused input")]
    Lexical,
}

#[derive(Clone, Debug)]
struct HybridDocument {
    body: String,
    vector: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HybridHit {
    pub source_id: String,
    pub reciprocal_rank_score: f64,
    pub lexical_rank: Option<usize>,
    pub semantic_rank: usize,
}

/// No interior mutability: share with readers through a caller-managed snapshot,
/// mutex, or another authenticated runtime layer. Revision checks reject stale
/// writes; they do not provide distributed consensus across independent stores.
pub struct HybridIndex {
    tenant: String,
    model_id: String,
    dimension: usize,
    revision: u64,
    documents: BTreeMap<String, HybridDocument>,
    lexical: LexicalIndex,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-.:".contains(&byte))
}

fn vector_norm_squared(vector: &[f32], dimension: usize) -> Result<f64, HybridError> {
    if vector.len() != dimension || vector.iter().any(|x| !x.is_finite()) {
        return Err(HybridError::Invalid);
    }
    let norm: f64 = vector.iter().map(|v| f64::from(*v).powi(2)).sum();
    if !norm.is_finite() || norm <= MIN_NORM_SQUARED {
        return Err(HybridError::Invalid);
    }
    Ok(norm)
}

impl HybridIndex {
    pub fn new(
        authenticated_tenant: &str,
        model_id: &str,
        dimension: usize,
    ) -> Result<Self, HybridError> {
        if !valid_id(authenticated_tenant)
            || !valid_id(model_id)
            || !(1..=MAX_DIMENSION).contains(&dimension)
        {
            return Err(HybridError::Invalid);
        }
        Ok(Self {
            tenant: authenticated_tenant.to_owned(),
            model_id: model_id.to_owned(),
            dimension,
            revision: 0,
            documents: BTreeMap::new(),
            lexical: LexicalIndex::build(Vec::new()).map_err(|_| HybridError::Lexical)?,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn authorize_tenant(&self, authenticated_tenant: &str) -> Result<(), HybridError> {
        if !valid_id(authenticated_tenant) || authenticated_tenant != self.tenant {
            return Err(HybridError::Invalid);
        }
        Ok(())
    }

    /// Atomically replace text, associated embedding and lexical postings.
    /// The digest checks consistency with the caller's source bytes only; it
    /// cannot certify that a model/provider genuinely created the vector.
    // Retain the established public contract; changing argument order/types
    // would be a breaking API change for repository consumers.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert(
        &mut self,
        authenticated_tenant: &str,
        source_id: &str,
        body: String,
        embedding: Vec<f32>,
        embedding_model: &str,
        source_sha256: [u8; 32],
        expected_revision: u64,
    ) -> Result<u64, HybridError> {
        self.authorize_tenant(authenticated_tenant)?;
        if !valid_id(source_id) || embedding_model != self.model_id {
            return Err(HybridError::Invalid);
        }
        if expected_revision != self.revision {
            return Err(HybridError::Conflict);
        }
        if body.len() > MAX_BODY_BYTES {
            return Err(HybridError::Limit);
        }
        vector_norm_squared(&embedding, self.dimension)?;
        let observed: [u8; 32] = Sha256::digest(body.as_bytes()).into();
        if observed != source_sha256 {
            return Err(HybridError::SourceDigestMismatch);
        }
        let mut candidate = self.documents.clone();
        candidate.insert(
            source_id.to_owned(),
            HybridDocument {
                body,
                vector: embedding,
            },
        );
        self.publish(candidate)
    }

    /// Delete both lexical and vector entries as one snapshot transition.
    pub fn remove(
        &mut self,
        authenticated_tenant: &str,
        source_id: &str,
        expected_revision: u64,
    ) -> Result<u64, HybridError> {
        self.authorize_tenant(authenticated_tenant)?;
        if !valid_id(source_id) {
            return Err(HybridError::Invalid);
        }
        if expected_revision != self.revision || !self.documents.contains_key(source_id) {
            return Err(HybridError::Conflict);
        }
        let mut candidate = self.documents.clone();
        candidate.remove(source_id);
        self.publish(candidate)
    }

    fn publish(&mut self, candidate: BTreeMap<String, HybridDocument>) -> Result<u64, HybridError> {
        if candidate.len() > MAX_DOCUMENTS {
            return Err(HybridError::Limit);
        }
        let mut total_bytes = 0usize;
        for record in candidate.values() {
            total_bytes = total_bytes
                .checked_add(record.body.len())
                .ok_or(HybridError::Limit)?;
            if total_bytes > MAX_TOTAL_BYTES {
                return Err(HybridError::Limit);
            }
        }
        let next_revision = self.revision.checked_add(1).ok_or(HybridError::Limit)?;
        // No external callbacks between construction and publishing. If BM25
        // rejects an input, the previous state and revision remain unchanged.
        let lexical = LexicalIndex::build(
            candidate
                .iter()
                .map(|(id, item)| SourceDocument {
                    tenant: self.tenant.clone(),
                    source_id: id.clone(),
                    body: item.body.clone(),
                })
                .collect(),
        )
        .map_err(|_| HybridError::Lexical)?;
        self.lexical = lexical;
        self.documents = candidate;
        self.revision = next_revision;
        Ok(next_revision)
    }

    /// RRF(BM25 rank, cosine rank), with k=60; stable lexical tie-break by ID.
    /// The embedding_model binds the query to the index's model and dimension.
    pub fn search(
        &self,
        authenticated_tenant: &str,
        query: &str,
        query_embedding: &[f32],
        embedding_model: &str,
        limit: usize,
    ) -> Result<Vec<HybridHit>, HybridError> {
        self.authorize_tenant(authenticated_tenant)?;
        if embedding_model != self.model_id {
            return Err(HybridError::Invalid);
        }
        if limit == 0 || limit > MAX_RESULTS {
            return Err(HybridError::Limit);
        }
        let query_norm = vector_norm_squared(query_embedding, self.dimension)?.sqrt();
        let lexical = self
            .lexical
            .search(&self.tenant, query, MAX_RESULTS)
            .map_err(|_| HybridError::Invalid)?;
        if self.documents.is_empty() {
            return Ok(Vec::new());
        }
        let lexical_ranks: BTreeMap<String, usize> = lexical
            .iter()
            .enumerate()
            .map(|(idx, hit)| (hit.source_id.clone(), idx + 1))
            .collect();
        let mut semantic: Vec<(String, f64)> = self
            .documents
            .iter()
            .map(|(source_id, record)| {
                let norm_squared = vector_norm_squared(&record.vector, self.dimension)?;
                let dot: f64 = query_embedding
                    .iter()
                    .zip(&record.vector)
                    .map(|(left, right)| f64::from(*left) * f64::from(*right))
                    .sum();
                let cosine = dot / (query_norm * norm_squared.sqrt());
                if !cosine.is_finite() {
                    return Err(HybridError::Invalid);
                }
                Ok((source_id.clone(), cosine))
            })
            .collect::<Result<_, _>>()?;
        semantic.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut hits: Vec<HybridHit> = semantic
            .into_iter()
            .enumerate()
            .map(|(index, (source_id, _))| {
                let semantic_rank = index + 1;
                let lexical_rank = lexical_ranks.get(&source_id).copied();
                let lexical_score = lexical_rank
                    .map(|rank| 1.0 / (RRF_K + rank as f64))
                    .unwrap_or(0.0);
                HybridHit {
                    source_id,
                    reciprocal_rank_score: lexical_score + 1.0 / (RRF_K + semantic_rank as f64),
                    lexical_rank,
                    semantic_rank,
                }
            })
            .collect();
        hits.sort_by(|a, b| {
            b.reciprocal_rank_score
                .total_cmp(&a.reciprocal_rank_score)
                .then_with(|| a.source_id.cmp(&b.source_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_index() -> HybridIndex {
        HybridIndex::new("tenant_a", "emb_v1", 2).unwrap()
    }

    fn put(
        index: &mut HybridIndex,
        id: &str,
        body: &str,
        vector: &[f32],
    ) -> Result<u64, HybridError> {
        let revision = index.revision();
        index.upsert(
            "tenant_a",
            id,
            body.to_owned(),
            vector.to_vec(),
            "emb_v1",
            Sha256::digest(body.as_bytes()).into(),
            revision,
        )
    }

    #[test]
    fn semantic_retrieval_without_lexical_match_is_deterministic() {
        let mut index = new_index();
        assert_eq!(put(&mut index, "a", "forest", &[1.0, 0.0]), Ok(1));
        assert_eq!(put(&mut index, "b", "desert", &[0.0, 1.0]), Ok(2));
        let hits = index
            .search("tenant_a", "unmatched", &[1.0, 0.0], "emb_v1", 2)
            .unwrap();
        assert_eq!(
            hits.iter()
                .map(|h| h.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(hits[0].lexical_rank, None);
        assert_eq!(hits[0].semantic_rank, 1);
    }

    #[test]
    fn reciprocal_rank_fusion_merges_bm25_and_cosine() {
        let mut index = new_index();
        put(&mut index, "vector", "alpha", &[1.0, 0.0]).unwrap();
        put(&mut index, "lexical", "needle needle needle", &[0.0, 1.0]).unwrap();
        let hits = index
            .search("tenant_a", "needle", &[1.0, 0.0], "emb_v1", 2)
            .unwrap();
        assert_eq!(hits[0].source_id, "lexical");
        assert_eq!(hits[0].lexical_rank, Some(1));
        assert_eq!(hits[0].semantic_rank, 2);
        assert_eq!(hits[1].source_id, "vector");
        assert_eq!(hits[1].lexical_rank, None);
    }

    #[test]
    fn overwritten_embeddings_and_stale_tokens_are_replaced_together() {
        let mut index = new_index();
        put(&mut index, "doc", "stale secret", &[0.0, 1.0]).unwrap();
        put(&mut index, "doc", "fresh", &[1.0, 0.0]).unwrap();
        let hits = index
            .search("tenant_a", "stale", &[1.0, 0.0], "emb_v1", 1)
            .unwrap();
        assert_eq!(hits[0].lexical_rank, None);
        assert_eq!(hits[0].semantic_rank, 1);
        assert_eq!(index.revision(), 2);
    }

    #[test]
    fn tenant_and_embedding_provenance_gate_are_fail_closed() {
        let mut index = new_index();
        assert_eq!(
            index.upsert(
                "tenant_b",
                "id",
                "s".into(),
                vec![1.0, 0.0],
                "emb_v1",
                Sha256::digest(b"s").into(),
                0
            ),
            Err(HybridError::Invalid)
        );
        assert_eq!(
            index.search("tenant_b", "s", &[1.0, 0.0], "emb_v1", 1),
            Err(HybridError::Invalid)
        );
        assert_eq!(
            index.upsert(
                "tenant_a",
                "id",
                "s".into(),
                vec![1.0, 0.0],
                "emb_v1",
                [0; 32],
                0
            ),
            Err(HybridError::SourceDigestMismatch)
        );
        assert_eq!(
            index.upsert(
                "tenant_a",
                "id",
                "s".into(),
                vec![1.0, 0.0],
                "wrong",
                Sha256::digest(b"s").into(),
                0
            ),
            Err(HybridError::Invalid)
        );
        assert_eq!(index.revision(), 0);
    }

    #[test]
    fn stale_revision_and_invalid_vectors_never_publish() {
        let mut index = new_index();
        put(&mut index, "id", "before", &[1.0, 0.0]).unwrap();
        assert_eq!(
            index.upsert(
                "tenant_a",
                "id",
                "after".into(),
                vec![1.0, 0.0],
                "emb_v1",
                Sha256::digest(b"after").into(),
                0
            ),
            Err(HybridError::Conflict)
        );
        for invalid in [
            vec![0.0, 0.0],
            vec![f32::NAN, 1.0],
            vec![1.0],
            vec![f32::INFINITY, 0.0],
        ] {
            assert_eq!(
                put(&mut index, "id", "after", &invalid),
                Err(HybridError::Invalid)
            );
        }
        let hit = index
            .search("tenant_a", "before", &[1.0, 0.0], "emb_v1", 1)
            .unwrap();
        assert_eq!(hit[0].lexical_rank, Some(1));
        assert_eq!(index.revision(), 1);
    }

    #[test]
    fn delete_erases_both_indices_and_stale_replay_is_refused() {
        let mut index = new_index();
        put(&mut index, "id", "secret", &[1.0, 0.0]).unwrap();
        assert_eq!(index.remove("tenant_a", "id", 1), Ok(2));
        assert!(index
            .search("tenant_a", "secret", &[1.0, 0.0], "emb_v1", 1)
            .unwrap()
            .is_empty());
        assert_eq!(
            index.remove("tenant_a", "id", 2),
            Err(HybridError::Conflict)
        );
        assert_eq!(
            index.upsert(
                "tenant_a",
                "id",
                "replay".into(),
                vec![1.0, 0.0],
                "emb_v1",
                Sha256::digest(b"replay").into(),
                1
            ),
            Err(HybridError::Conflict)
        );
    }

    #[test]
    fn tie_breaks_and_resource_bounds_are_stable() {
        let mut index = new_index();
        put(&mut index, "zeta", "same", &[1.0, 0.0]).unwrap();
        put(&mut index, "alpha", "same", &[1.0, 0.0]).unwrap();
        let hits = index
            .search("tenant_a", "same", &[1.0, 0.0], "emb_v1", 2)
            .unwrap();
        assert_eq!(hits[0].source_id, "alpha");
        assert_eq!(hits[1].source_id, "zeta");
        assert_eq!(
            index.search("tenant_a", "same", &[1.0, 0.0], "emb_v1", 101),
            Err(HybridError::Limit)
        );
        assert_eq!(
            put(
                &mut index,
                "oversized",
                &"x".repeat(MAX_BODY_BYTES + 1),
                &[1.0, 0.0]
            ),
            Err(HybridError::Limit)
        );
        assert_eq!(index.revision(), 2);
        assert!(HybridIndex::new("tenant_a", "m", 0).is_err());
        assert!(HybridIndex::new("tenant_a", "m", 257).is_err());
    }
}
