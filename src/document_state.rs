//! Source PDF 1.15–1.18: bounded document labels, GUID attributes and transactions.
//! This is local document state; it grants no execution authority and is not persistent.
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 1024;
const MAX_ATTRIBUTES: usize = 64;
const MAX_PAYLOAD: usize = 4096;
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_HISTORY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LabelPath(Vec<u32>);
impl LabelPath {
    pub fn parse(value: &str) -> Result<Self, StateError> {
        if value.len() > MAX_DEPTH * 11 {
            return Err(StateError::InvalidPath);
        }
        let parts: Vec<_> = value.split(':').collect();
        if parts.is_empty() || parts.len() > MAX_DEPTH || parts[0] != "0" {
            return Err(StateError::InvalidPath);
        }
        let mut tags = vec![0];
        for part in &parts[1..] {
            if part.is_empty() || part.starts_with('0') || !part.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(StateError::InvalidPath);
            }
            let tag = part.parse::<u32>().map_err(|_| StateError::InvalidPath)?;
            tags.push(tag);
        }
        Ok(Self(tags))
    }
    pub fn entry(&self) -> String {
        self.0
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(":")
    }
    fn parent(&self) -> Option<Self> {
        (self.0.len() > 1).then(|| Self(self.0[..self.0.len() - 1].to_vec()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Attribute {
    payload: Vec<u8>,
    forgotten: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    nodes: BTreeMap<LabelPath, BTreeMap<Uuid, Attribute>>,
}
#[derive(Debug, Clone)]
pub enum Mutation {
    CreateLabel(LabelPath),
    Attach {
        label: LabelPath,
        guid: Uuid,
        payload: Vec<u8>,
    },
    Forget {
        label: LabelPath,
        guid: Uuid,
    },
    Remember {
        label: LabelPath,
        guid: Uuid,
    },
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateError {
    #[error("invalid canonical label path")]
    InvalidPath,
    #[error("label or attribute already exists")]
    Duplicate,
    #[error("label, parent or attribute does not exist")]
    Missing,
    #[error("document or history limit exceeded")]
    Limit,
    #[error("invalid transaction name, GUID or state transition")]
    Invalid,
    #[error("no transaction available")]
    NoHistory,
}
#[derive(Debug, Clone)]
struct Transaction {
    name: String,
    before: Snapshot,
    after: Snapshot,
}

pub struct DocumentState {
    state: Snapshot,
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
}
impl Default for DocumentState {
    fn default() -> Self {
        Self {
            state: Snapshot {
                nodes: BTreeMap::from([(LabelPath(vec![0]), BTreeMap::new())]),
            },
            undo: vec![],
            redo: vec![],
        }
    }
}
impl DocumentState {
    /// All operations are applied to a private snapshot; any failure discards the entire batch.
    pub fn commit(&mut self, name: &str, changes: &[Mutation]) -> Result<(), StateError> {
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(char::is_control)
            || changes.is_empty()
        {
            return Err(StateError::Invalid);
        }
        if changes.len() > 1024 || self.undo.len() >= MAX_HISTORY {
            return Err(StateError::Limit);
        }
        let mut next = self.state.clone();
        for change in changes {
            next.apply(change)?;
        }
        let tx = Transaction {
            name: name.into(),
            before: self.state.clone(),
            after: next.clone(),
        };
        self.state = next;
        self.undo.push(tx);
        self.redo.clear();
        Ok(())
    }
    pub fn attribute(&self, label: &LabelPath, guid: Uuid) -> Option<&[u8]> {
        let attr = self.state.nodes.get(label)?.get(&guid)?;
        (!attr.forgotten).then_some(attr.payload.as_slice())
    }
    pub fn contains_label(&self, label: &LabelPath) -> bool {
        self.state.nodes.contains_key(label)
    }
    pub fn undo(&mut self) -> Result<String, StateError> {
        let tx = self.undo.pop().ok_or(StateError::NoHistory)?;
        self.state = tx.before.clone();
        let name = tx.name.clone();
        self.redo.push(tx);
        Ok(name)
    }
    pub fn redo(&mut self) -> Result<String, StateError> {
        let tx = self.redo.pop().ok_or(StateError::NoHistory)?;
        self.state = tx.after.clone();
        let name = tx.name.clone();
        self.undo.push(tx);
        Ok(name)
    }
    /// Explicit checkpoint of local undo history. Persistence is a separate contract.
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
impl Snapshot {
    fn apply(&mut self, change: &Mutation) -> Result<(), StateError> {
        match change {
            Mutation::CreateLabel(label) => {
                if self.nodes.contains_key(label) {
                    return Err(StateError::Duplicate);
                }
                let parent = label.parent().ok_or(StateError::InvalidPath)?;
                if !self.nodes.contains_key(&parent) {
                    return Err(StateError::Missing);
                }
                if self.nodes.len() >= MAX_NODES {
                    return Err(StateError::Limit);
                }
                self.nodes.insert(label.clone(), BTreeMap::new());
            }
            Mutation::Attach {
                label,
                guid,
                payload,
            } => {
                if guid.is_nil() {
                    return Err(StateError::Invalid);
                }
                if payload.len() > MAX_PAYLOAD {
                    return Err(StateError::Limit);
                }
                let total: usize = self
                    .nodes
                    .values()
                    .flat_map(|attrs| attrs.values())
                    .map(|a| a.payload.len())
                    .sum();
                if total + payload.len() > MAX_DOCUMENT_BYTES {
                    return Err(StateError::Limit);
                }
                let attrs = self.nodes.get_mut(label).ok_or(StateError::Missing)?;
                if attrs.contains_key(guid) {
                    return Err(StateError::Duplicate);
                }
                if attrs.len() >= MAX_ATTRIBUTES {
                    return Err(StateError::Limit);
                }
                attrs.insert(
                    *guid,
                    Attribute {
                        payload: payload.clone(),
                        forgotten: false,
                    },
                );
            }
            Mutation::Forget { label, guid } | Mutation::Remember { label, guid } => {
                let attr = self
                    .nodes
                    .get_mut(label)
                    .and_then(|attrs| attrs.get_mut(guid))
                    .ok_or(StateError::Missing)?;
                let forgotten = matches!(change, Mutation::Forget { .. });
                if attr.forgotten == forgotten {
                    return Err(StateError::Invalid);
                }
                attr.forgotten = forgotten;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_paths_and_no_aliases() {
        for bad in [
            "",
            "1",
            "00",
            "0:0",
            "0:01",
            "0:+1",
            "0:-1",
            "0:",
            "0:4294967296",
        ] {
            assert_eq!(LabelPath::parse(bad), Err(StateError::InvalidPath));
        }
        assert_eq!(LabelPath::parse("0:1:4:2").unwrap().entry(), "0:1:4:2");
    }
    #[test]
    fn transaction_rollback_lifecycle_and_branching() {
        let mut doc = DocumentState::default();
        let label = LabelPath::parse("0:1").unwrap();
        let guid = Uuid::from_u128(1);
        let attach = Mutation::Attach {
            label: label.clone(),
            guid,
            payload: b"value".to_vec(),
        };
        assert_eq!(
            doc.commit(
                "bad",
                &[
                    Mutation::CreateLabel(label.clone()),
                    attach.clone(),
                    attach.clone()
                ]
            ),
            Err(StateError::Duplicate)
        );
        assert!(!doc.contains_label(&label));
        doc.commit(
            "create",
            &[Mutation::CreateLabel(label.clone()), attach.clone()],
        )
        .unwrap();
        doc.commit(
            "forget",
            &[Mutation::Forget {
                label: label.clone(),
                guid,
            }],
        )
        .unwrap();
        assert_eq!(doc.attribute(&label, guid), None);
        assert_eq!(
            doc.commit("duplicate forgotten GUID", &[attach]),
            Err(StateError::Duplicate)
        );
        doc.undo().unwrap();
        assert_eq!(doc.attribute(&label, guid), Some(b"value".as_slice()));
        doc.redo().unwrap();
        assert_eq!(doc.attribute(&label, guid), None);
        doc.commit(
            "remember",
            &[Mutation::Remember {
                label: label.clone(),
                guid,
            }],
        )
        .unwrap();
        assert_eq!(doc.attribute(&label, guid), Some(b"value".as_slice()));
        doc.undo().unwrap();
        doc.commit(
            "branch",
            &[Mutation::CreateLabel(LabelPath::parse("0:2").unwrap())],
        )
        .unwrap();
        assert_eq!(doc.redo(), Err(StateError::NoHistory));
    }
    #[test]
    fn bounds_and_parent_existence() {
        let mut doc = DocumentState::default();
        assert_eq!(
            doc.commit(
                "orphan",
                &[Mutation::CreateLabel(LabelPath::parse("0:1:2").unwrap())]
            ),
            Err(StateError::Missing)
        );
        for i in 1..=MAX_HISTORY {
            doc.commit(
                "bounded",
                &[Mutation::CreateLabel(
                    LabelPath::parse(&format!("0:{i}")).unwrap(),
                )],
            )
            .unwrap();
        }
        assert_eq!(
            doc.commit(
                "full",
                &[Mutation::CreateLabel(LabelPath::parse("0:99").unwrap())]
            ),
            Err(StateError::Limit)
        );
        doc.clear_history();
        let root = LabelPath::parse("0").unwrap();
        assert_eq!(
            doc.commit(
                "oversized",
                &[Mutation::Attach {
                    label: root,
                    guid: Uuid::from_u128(1),
                    payload: vec![0; MAX_PAYLOAD + 1]
                }]
            ),
            Err(StateError::Limit)
        );
    }
}
