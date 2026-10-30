//! Deterministic OCAF attribute function graph.
//! This module cannot authorize runtime effects.
use crate::document_state::LabelPath;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttributeKey {
    pub label: LabelPath,
    pub guid: Uuid,
}
