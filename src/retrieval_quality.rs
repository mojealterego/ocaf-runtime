//! Local, tenant-scoped retrieval and ground-truth quality metrics.
//! Source PDF-1.47 / PDF-1.52 (not canonical Apex 1.47 / 1.52).
//! No external "Moss" adapter, network transport, or performance claim.
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_DOCUMENTS: usize = 4096;
const MAX_BODY_BYTES: usize = 65536;
const MAX_BODY_TOKENS: usize = 10000;
const MAX_QUERY_BYTES: usize = 2048;
const MAX_QUERY_TOKENS: usize = 256;
const MAX_RESULTS: usize = 100;
const MAX_EVALUATION_QUERIES: usize = 4096;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RetrievalError {
    #[error("invalid query, tenant, source identity, or relevance annotation")]
    Invalid,
    #[error("duplicate document or query identity")]
    Duplicate,
    #[error("retrieval resource bound exceeded")]
    Limit,
    #[error("stale mutable retrieval revision")]
    Conflict,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}

fn tokenize(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect()
}

pub struct SourceDocument {
    pub tenant: String,
    pub source_id: String,
    pub body: String,
}

struct IndexedDocument {
    source_id: String,
    len: usize,
}

struct PendingDocument {
    doc: IndexedDocument,
    term_counts: BTreeMap<String, usize>,
}

struct TenantIndex {
    documents: Vec<IndexedDocument>,
    // Each term resolves only matching documents; never scan cross-tenant text.
    postings: BTreeMap<String, Vec<(usize, usize)>>,
    average_length: f64,
}

pub struct LexicalIndex {
    tenants: BTreeMap<String, TenantIndex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub source_id: String,
    pub bm25_score: f64,
}

impl LexicalIndex {
    /// Index only operator-admitted, tenant-scoped documents. This is an in-memory
    /// baseline and MUST NOT be presented as a full semantic vector search engine.
    pub fn build(documents: Vec<SourceDocument>) -> Result<Self, RetrievalError> {
        if documents.len() > MAX_DOCUMENTS {
            return Err(RetrievalError::Limit);
        }
        let mut groups: BTreeMap<String, Vec<PendingDocument>> = BTreeMap::new();
        let mut unique = BTreeSet::new();
        for doc in documents {
            if !valid_id(&doc.tenant) || !valid_id(&doc.source_id) {
                return Err(RetrievalError::Invalid);
            }
            if doc.body.len() > MAX_BODY_BYTES {
                return Err(RetrievalError::Limit);
            }
            if !unique.insert((doc.tenant.clone(), doc.source_id.clone())) {
                return Err(RetrievalError::Duplicate);
            }
            let tokens = tokenize(&doc.body);
            if tokens.len() > MAX_BODY_TOKENS {
                return Err(RetrievalError::Limit);
            }
            let mut frequencies = BTreeMap::new();
            for word in &tokens {
                *frequencies.entry(word.clone()).or_insert(0) += 1;
            }
            groups.entry(doc.tenant).or_default().push(PendingDocument {
                doc: IndexedDocument {
                    source_id: doc.source_id,
                    len: tokens.len(),
                },
                term_counts: frequencies,
            });
        }
        let mut tenants = BTreeMap::new();
        for (tenant, entries) in groups {
            let mut postings: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
            let total: usize = entries.iter().map(|entry| entry.doc.len).sum();
            let average_length = total as f64 / entries.len() as f64;
            let mut documents = Vec::with_capacity(entries.len());
            for (index, entry) in entries.into_iter().enumerate() {
                for (term, count) in entry.term_counts {
                    postings.entry(term).or_default().push((index, count));
                }
                documents.push(entry.doc);
            }
            tenants.insert(
                tenant,
                TenantIndex {
                    documents,
                    postings,
                    average_length,
                },
            );
        }
        Ok(Self { tenants })
    }

    pub fn search(
        &self,
        tenant: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, RetrievalError> {
        if !valid_id(tenant) || query.is_empty() || query.len() > MAX_QUERY_BYTES {
            return Err(RetrievalError::Invalid);
        }
        if limit == 0 || limit > MAX_RESULTS {
            return Err(RetrievalError::Limit);
        }
        let tokens = tokenize(query);
        if tokens.is_empty() {
            return Err(RetrievalError::Invalid);
        }
        if tokens.len() > MAX_QUERY_TOKENS {
            return Err(RetrievalError::Limit);
        }
        let Some(index) = self.tenants.get(tenant) else {
            return Ok(vec![]);
        };
        let unique: BTreeSet<String> = tokens.into_iter().collect();
        let n = index.documents.len() as f64;
        let mut scores: BTreeMap<usize, f64> = BTreeMap::new();
        for term in &unique {
            let Some(postings) = index.postings.get(term) else {
                continue;
            };
            let df = postings.len() as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for &(doc_index, frequency) in postings {
                let doc = &index.documents[doc_index];
                let length_factor = if index.average_length == 0.0 {
                    1.0
                } else {
                    0.25 + 0.75 * (doc.len as f64 / index.average_length)
                };
                let tf = frequency as f64;
                *scores.entry(doc_index).or_insert(0.0) +=
                    idf * (tf * 2.2) / (tf + 1.2 * length_factor);
            }
        }
        let mut hits: Vec<SearchHit> = scores
            .into_iter()
            .filter(|(_, score)| *score > 0.0 && score.is_finite())
            .map(|(index_in_tenant, bm25_score)| SearchHit {
                source_id: index.documents[index_in_tenant].source_id.clone(),
                bm25_score,
            })
            .collect();
        hits.sort_by(|a, b| {
            b.bm25_score
                .total_cmp(&a.bm25_score)
                .then_with(|| a.source_id.cmp(&b.source_id))
        });
        hits.truncate(limit);
        Ok(hits)
    }
}

/// A bounded, copy-on-write mutable lexical index for one authenticated tenant.
///
/// Every update atomically replaces the full in-memory snapshot on success;
/// rejected updates leave both the revision and postings unchanged. This
/// prevents stale tokens after overwrite or deletion. No durable storage,
/// cross-process synchronization, vector embedding, or external provider is
/// implied. The tenant must be supplied by trusted ingress, never model text.
pub struct MutableLexicalIndex {
    tenant: String,
    revision: u64,
    bodies: BTreeMap<String, String>,
    snapshot: LexicalIndex,
}

const MAX_MUTABLE_DOCUMENTS: usize = 256;
const MAX_MUTABLE_TOTAL_BYTES: usize = 2 * 1024 * 1024;

impl MutableLexicalIndex {
    pub fn new(authenticated_tenant: &str) -> Result<Self, RetrievalError> {
        if !valid_id(authenticated_tenant) {
            return Err(RetrievalError::Invalid);
        }
        Ok(Self {
            tenant: authenticated_tenant.to_owned(),
            revision: 0,
            bodies: BTreeMap::new(),
            snapshot: LexicalIndex {
                tenants: BTreeMap::new(),
            },
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn ensure_tenant(&self, authenticated_tenant: &str) -> Result<(), RetrievalError> {
        if !valid_id(authenticated_tenant) || authenticated_tenant != self.tenant {
            return Err(RetrievalError::Invalid);
        }
        Ok(())
    }

    /// Search a stable snapshot. The caller must provide its authenticated
    /// tenant; this is an authorization boundary, not a trusted identity parser.
    pub fn search(
        &self,
        authenticated_tenant: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, RetrievalError> {
        self.ensure_tenant(authenticated_tenant)?;
        self.snapshot.search(&self.tenant, query, limit)
    }

    /// Insert or replace exactly one document at the expected index revision.
    /// The full snapshot is built before publication; no partial writes.
    pub fn upsert(
        &mut self,
        authenticated_tenant: &str,
        source_id: &str,
        body: &str,
        expected_revision: u64,
    ) -> Result<u64, RetrievalError> {
        self.ensure_tenant(authenticated_tenant)?;
        if !valid_id(source_id) {
            return Err(RetrievalError::Invalid);
        }
        if expected_revision != self.revision {
            return Err(RetrievalError::Conflict);
        }
        if body.len() > MAX_BODY_BYTES {
            return Err(RetrievalError::Limit);
        }
        let mut candidate = self.bodies.clone();
        candidate.insert(source_id.to_owned(), body.to_owned());
        self.publish(candidate)
    }

    /// Delete without leaving stale terms; a missing document is rejected.
    pub fn remove(
        &mut self,
        authenticated_tenant: &str,
        source_id: &str,
        expected_revision: u64,
    ) -> Result<u64, RetrievalError> {
        self.ensure_tenant(authenticated_tenant)?;
        if !valid_id(source_id) {
            return Err(RetrievalError::Invalid);
        }
        if expected_revision != self.revision {
            return Err(RetrievalError::Conflict);
        }
        if !self.bodies.contains_key(source_id) {
            return Err(RetrievalError::Invalid);
        }
        let mut candidate = self.bodies.clone();
        candidate.remove(source_id);
        self.publish(candidate)
    }

    fn publish(&mut self, candidate: BTreeMap<String, String>) -> Result<u64, RetrievalError> {
        if candidate.len() > MAX_MUTABLE_DOCUMENTS {
            return Err(RetrievalError::Limit);
        }
        let mut total_bytes = 0usize;
        for body in candidate.values() {
            total_bytes = total_bytes
                .checked_add(body.len())
                .ok_or(RetrievalError::Limit)?;
            if total_bytes > MAX_MUTABLE_TOTAL_BYTES {
                return Err(RetrievalError::Limit);
            }
        }
        let next_revision = self.revision.checked_add(1).ok_or(RetrievalError::Limit)?;
        let next = LexicalIndex::build(
            candidate
                .iter()
                .map(|(source_id, body)| SourceDocument {
                    tenant: self.tenant.clone(),
                    source_id: source_id.clone(),
                    body: body.clone(),
                })
                .collect(),
        )?;
        self.snapshot = next;
        self.bodies = candidate;
        self.revision = next_revision;
        Ok(next_revision)
    }
}

/// The caller owns the immutable ground-truth test corpus. Query IDs and
/// relevant document IDs are explicitly recorded, never inferred from results.
pub struct JudgedQuery {
    pub query_id: String,
    pub relevant_source_ids: Vec<String>,
    pub returned_source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityReport {
    pub evaluated_queries: usize,
    pub recall_at_k: f64,
    pub mean_reciprocal_rank: f64,
}

pub fn evaluate_quality(corpus: &[JudgedQuery], k: usize) -> Result<QualityReport, RetrievalError> {
    if corpus.is_empty() {
        return Err(RetrievalError::Invalid);
    }
    if corpus.len() > MAX_EVALUATION_QUERIES || k == 0 || k > MAX_RESULTS {
        return Err(RetrievalError::Limit);
    }
    let mut query_ids = BTreeSet::new();
    let mut recall = 0.0;
    let mut mrr = 0.0;
    for entry in corpus {
        if !valid_id(&entry.query_id) || !query_ids.insert(&entry.query_id) {
            return Err(RetrievalError::Duplicate);
        }
        if entry.relevant_source_ids.is_empty()
            || entry.relevant_source_ids.len() > MAX_DOCUMENTS
            || entry.returned_source_ids.len() > MAX_DOCUMENTS
        {
            return Err(RetrievalError::Invalid);
        }
        let relevant: BTreeSet<&str> = entry
            .relevant_source_ids
            .iter()
            .map(String::as_str)
            .collect();
        let returned: BTreeSet<&str> = entry
            .returned_source_ids
            .iter()
            .map(String::as_str)
            .collect();
        if relevant.len() != entry.relevant_source_ids.len()
            || returned.len() != entry.returned_source_ids.len()
            || !relevant.iter().all(|id| valid_id(id))
            || !returned.iter().all(|id| valid_id(id))
        {
            return Err(RetrievalError::Invalid);
        }
        let top: Vec<&str> = entry
            .returned_source_ids
            .iter()
            .take(k)
            .map(String::as_str)
            .collect();
        let found = top.iter().filter(|id| relevant.contains(*id)).count();
        recall += found as f64 / relevant.len() as f64;
        if let Some(pos) = top.iter().position(|id| relevant.contains(id)) {
            mrr += 1.0 / (pos + 1) as f64;
        }
    }
    Ok(QualityReport {
        evaluated_queries: corpus.len(),
        recall_at_k: recall / corpus.len() as f64,
        mean_reciprocal_rank: mrr / corpus.len() as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> LexicalIndex {
        LexicalIndex::build(vec![
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "invoice_1".into(),
                body: "invoice overdue finance finance".into(),
            },
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "finance_2".into(),
                body: "finance overview".into(),
            },
            SourceDocument {
                tenant: "tenant_b".into(),
                source_id: "private_3".into(),
                body: "invoice overdue finance private".into(),
            },
        ])
        .unwrap()
    }

    #[test]
    fn lexical_bm25_results_are_deterministic_and_tenant_isolated() {
        let index = fixture();
        let a = index.search("tenant_a", "invoice overdue", 10).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].source_id, "invoice_1");
        assert_eq!(a, index.search("tenant_a", "invoice overdue", 10).unwrap());
        assert_eq!(index.search("tenant_x", "invoice", 10).unwrap().len(), 0);
        let b = index.search("tenant_b", "private", 10).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].source_id, "private_3");
        assert!(!a.iter().any(|hit| hit.source_id == "private_3"));
    }

    #[test]
    fn polish_casefold_and_duplicate_query_terms_do_not_inflate_score() {
        let index = LexicalIndex::build(vec![SourceDocument {
            tenant: "a".into(),
            source_id: "s".into(),
            body: "ŻÓŁĆ matematyka".into(),
        }])
        .unwrap();
        assert_eq!(
            index.search("a", "żółć", 1).unwrap(),
            index.search("a", "ŻÓŁĆ ŻÓŁĆ", 1).unwrap()
        );
    }

    #[test]
    fn postings_are_independent_of_document_insertion_order() {
        let docs = vec![
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "aaa".into(),
                body: "alpha beta".into(),
            },
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "bbb".into(),
                body: "alpha beta".into(),
            },
        ];
        let first = LexicalIndex::build(docs)
            .unwrap()
            .search("tenant_a", "alpha", 10)
            .unwrap();
        let docs = vec![
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "bbb".into(),
                body: "alpha beta".into(),
            },
            SourceDocument {
                tenant: "tenant_a".into(),
                source_id: "aaa".into(),
                body: "alpha beta".into(),
            },
        ];
        let second = LexicalIndex::build(docs)
            .unwrap()
            .search("tenant_a", "alpha", 10)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].source_id, "aaa");
    }

    #[test]
    fn duplicate_sources_and_bounded_input_are_rejected() {
        let duplicate = vec![
            SourceDocument {
                tenant: "a".into(),
                source_id: "id".into(),
                body: "a".into(),
            },
            SourceDocument {
                tenant: "a".into(),
                source_id: "id".into(),
                body: "b".into(),
            },
        ];
        assert!(matches!(
            LexicalIndex::build(duplicate),
            Err(RetrievalError::Duplicate)
        ));
        assert!(matches!(
            fixture().search("tenant_a", "!!!", 10),
            Err(RetrievalError::Invalid)
        ));
        assert!(matches!(
            fixture().search("tenant_a", "query", 0),
            Err(RetrievalError::Limit)
        ));
        assert!(matches!(
            LexicalIndex::build(vec![SourceDocument {
                tenant: "a".into(),
                source_id: "b".into(),
                body: "x".repeat(MAX_BODY_BYTES + 1)
            }]),
            Err(RetrievalError::Limit)
        ));
    }

    #[test]
    fn recall_at_k_and_mrr_use_ground_truth_not_self_asserted_success() {
        let corpus = [
            JudgedQuery {
                query_id: "q1".into(),
                relevant_source_ids: vec!["s1".into(), "s2".into()],
                returned_source_ids: vec!["noise".into(), "s2".into(), "s1".into()],
            },
            JudgedQuery {
                query_id: "q2".into(),
                relevant_source_ids: vec!["s3".into()],
                returned_source_ids: vec!["unrelated".into()],
            },
        ];
        let score = evaluate_quality(&corpus, 2).unwrap();
        assert_eq!(score.evaluated_queries, 2);
        assert!((score.recall_at_k - 0.25).abs() < 1e-12);
        assert!((score.mean_reciprocal_rank - 0.25).abs() < 1e-12);
    }

    #[test]
    fn quality_evaluation_rejects_duplicate_ground_truth_and_results() {
        let duplicate = [JudgedQuery {
            query_id: "same".into(),
            relevant_source_ids: vec!["s".into()],
            returned_source_ids: vec!["s".into(), "s".into()],
        }];
        assert!(matches!(
            evaluate_quality(&duplicate, 1),
            Err(RetrievalError::Invalid)
        ));
        assert!(matches!(
            evaluate_quality(&[], 1),
            Err(RetrievalError::Invalid)
        ));
    }

    #[test]
    fn mutable_index_overwrite_removes_stale_postings_atomically() {
        let mut index = MutableLexicalIndex::new("tenant_a").unwrap();
        assert_eq!(index.upsert("tenant_a", "doc", "stale secret", 0), Ok(1));
        assert_eq!(index.search("tenant_a", "stale", 10).unwrap().len(), 1);
        assert_eq!(index.upsert("tenant_a", "doc", "fresh verified", 1), Ok(2));
        assert!(index.search("tenant_a", "stale", 10).unwrap().is_empty());
        assert_eq!(
            index.search("tenant_a", "fresh", 10).unwrap()[0].source_id,
            "doc"
        );
        assert_eq!(index.revision(), 2);
    }

    #[test]
    fn mutable_index_denies_cross_tenant_read_and_write() {
        let mut index = MutableLexicalIndex::new("tenant_a").unwrap();
        assert_eq!(
            index.upsert("tenant_b", "secret", "hidden", 0),
            Err(RetrievalError::Invalid)
        );
        assert_eq!(
            index.search("tenant_b", "hidden", 1),
            Err(RetrievalError::Invalid)
        );
        assert_eq!(index.revision(), 0);
        assert!(index.search("tenant_a", "hidden", 1).unwrap().is_empty());
    }

    #[test]
    fn mutable_index_rejected_write_preserves_revision_and_snapshot() {
        let mut index = MutableLexicalIndex::new("tenant_a").unwrap();
        assert_eq!(index.upsert("tenant_a", "doc", "unchanged", 0), Ok(1));
        assert_eq!(
            index.upsert("tenant_a", "doc", "stale", 0),
            Err(RetrievalError::Conflict)
        );
        assert_eq!(
            index.upsert("tenant_a", "doc", &"x".repeat(MAX_BODY_BYTES + 1), 1),
            Err(RetrievalError::Limit)
        );
        assert_eq!(index.revision(), 1);
        assert_eq!(
            index.search("tenant_a", "unchanged", 1).unwrap()[0].source_id,
            "doc"
        );
        assert!(index.search("tenant_a", "stale", 1).unwrap().is_empty());
    }

    #[test]
    fn mutable_index_remove_fences_replay_and_forgets_all_terms() {
        let mut index = MutableLexicalIndex::new("tenant_a").unwrap();
        assert_eq!(index.upsert("tenant_a", "doc", "secret", 0), Ok(1));
        assert_eq!(index.remove("tenant_a", "doc", 1), Ok(2));
        assert!(index.search("tenant_a", "secret", 1).unwrap().is_empty());
        assert_eq!(
            index.remove("tenant_a", "doc", 2),
            Err(RetrievalError::Invalid)
        );
        assert_eq!(
            index.upsert("tenant_a", "doc", "replayed", 1),
            Err(RetrievalError::Conflict)
        );
        assert_eq!(index.revision(), 2);
    }

    #[test]
    fn mutable_index_resource_limit_is_transactional() {
        let mut index = MutableLexicalIndex::new("tenant_a").unwrap();
        for id in 0..MAX_MUTABLE_DOCUMENTS {
            assert_eq!(
                index.upsert("tenant_a", &format!("doc_{id}"), "bounded", id as u64),
                Ok((id + 1) as u64)
            );
        }
        assert_eq!(
            index.upsert(
                "tenant_a",
                "one_too_many",
                "bounded",
                MAX_MUTABLE_DOCUMENTS as u64
            ),
            Err(RetrievalError::Limit)
        );
        assert_eq!(index.revision(), MAX_MUTABLE_DOCUMENTS as u64);
        assert_eq!(index.search("tenant_a", "bounded", 1).unwrap().len(), 1);
    }
}
