//! Deterministic document Function Mechanism (Argon source 1.57).
//! Produces bounded proposals; never authorizes external effects.
use crate::document_state::{DocumentState, LabelPath};
use crate::document_store::Change;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttributeKey {
    pub label: LabelPath,
    pub guid: Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Driver {
    Copy,
    CheckedAddI64,
}

#[derive(Clone, Debug)]
pub struct FunctionSpec {
    pub id: String,
    pub inputs: Vec<AttributeKey>,
    pub output: AttributeKey,
    pub driver: Driver,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FunctionError {
    #[error("invalid function definition")]
    Invalid,
    #[error("cycle in function dependency graph")]
    Cycle,
    #[error("missing input or output label")]
    Missing,
    #[error("invalid typed input or overflow")]
    Value,
    #[error("function graph resource limit")]
    Limit,
}

pub struct FunctionGraph {
    ordered: Vec<FunctionSpec>,
}

impl FunctionGraph {
    pub fn new(specs: Vec<FunctionSpec>) -> Result<Self, FunctionError> {
        if specs.is_empty() || specs.len() > 128 {
            return Err(FunctionError::Limit);
        }
        let mut ids = BTreeSet::new();
        let mut producers = BTreeMap::new();
        for (i, spec) in specs.iter().enumerate() {
            if spec.id.is_empty()
                || spec.id.len() > 128
                || !spec.id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || !ids.insert(spec.id.clone())
                || spec.output.guid.is_nil()
                || spec.inputs.is_empty()
                || spec.inputs.len() > 16
                || spec.inputs.iter().any(|x| x.guid.is_nil() || x == &spec.output)
                || spec.inputs.iter().collect::<BTreeSet<_>>().len() != spec.inputs.len()
                || (spec.driver == Driver::Copy && spec.inputs.len() != 1)
                || (spec.driver == Driver::CheckedAddI64 && spec.inputs.len() != 2)
                || producers.insert(spec.output.clone(), i).is_some()
            {
                return Err(FunctionError::Invalid);
            }
        }
        let mut indegree = vec![0usize; specs.len()];
        let mut children = vec![Vec::new(); specs.len()];
        for (consumer, spec) in specs.iter().enumerate() {
            for input in &spec.inputs {
                if let Some(&producer) = producers.get(input) {
                    children[producer].push(consumer);
                    indegree[consumer] += 1;
                }
            }
        }
        let mut ready = BTreeSet::new();
        for (index, &degree) in indegree.iter().enumerate() {
            if degree == 0 { ready.insert(index); }
        }
        let mut ordered = Vec::with_capacity(specs.len());
        while let Some(&index) = ready.first() {
            ready.remove(&index);
            ordered.push(specs[index].clone());
            for &child in &children[index] {
                indegree[child] -= 1;
                if indegree[child] == 0 { ready.insert(child); }
            }
        }
        if ordered.len() != specs.len() {
            return Err(FunctionError::Cycle);
        }
        Ok(Self { ordered })
    }

    /// Recompute dirty dependencies, returning one atomic commit's change list.
    /// Caller must authenticate tenant and append using a pinned document head.
    pub fn plan(
        &self,
        document: &DocumentState,
        dirty: &BTreeSet<AttributeKey>,
    ) -> Result<Vec<Change>, FunctionError> {
        if dirty.len() > 256 { return Err(FunctionError::Limit); }
        let mut changed = dirty.clone();
        let mut overlay: BTreeMap<AttributeKey, Vec<u8>> = BTreeMap::new();
        let mut changes = Vec::new();
        for spec in &self.ordered {
            if !changed.contains(&spec.output)
                && !spec.inputs.iter().any(|x| changed.contains(x))
            {
                continue;
            }
            if !document.contains_label(&spec.output.label) {
                return Err(FunctionError::Missing);
            }
            let mut values = Vec::new();
            for key in &spec.inputs {
                let bytes = overlay.get(key).map(Vec::as_slice)
                    .or_else(|| document.attribute(&key.label, key.guid))
                    .ok_or(FunctionError::Missing)?;
                values.push(bytes);
            }
            let result = match spec.driver {
                Driver::Copy => values[0].to_vec(),
                Driver::CheckedAddI64 => {
                    let a: [u8; 8] = values[0].try_into().map_err(|_| FunctionError::Value)?;
                    let b: [u8; 8] = values[1].try_into().map_err(|_| FunctionError::Value)?;
                    i64::from_le_bytes(a).checked_add(i64::from_le_bytes(b))
                        .ok_or(FunctionError::Value)?.to_le_bytes().to_vec()
                }
            };
            if result.len() > 4096 { return Err(FunctionError::Limit); }
            let label = spec.output.label.entry();
            let guid = spec.output.guid.to_string();
            let change = if document.attribute(&spec.output.label, spec.output.guid).is_some() {
                Change::Update { label, guid, payload: result.clone() }
            } else if document.attribute_exists(&spec.output.label, spec.output.guid) {
                return Err(FunctionError::Value);
            } else {
                Change::Attach { label, guid, payload: result.clone() }
            };
            overlay.insert(spec.output.clone(), result);
            changed.insert(spec.output.clone());
            changes.push(change);
        }
        Ok(changes)
    }
}
