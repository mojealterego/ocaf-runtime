//! Bounded tenant-isolated operation CRDT (causal multi-value register with
//! deterministic presentation). No network I/O, signature validation or durable
//! replay storage: authenticate actor/tenant before applying remote operations.
//!
//! Every unique dot (actor,counter) is immutable. We retain causal maxima rather
//! than prematurely discarding concurrent updates; this prevents order-dependent
//! resurrection and yields deterministic snapshots after replica reconciliation.
//! Tombstones are first-class operations. This is a bounded reference core, not
//! a replacement for authenticated Zenoh transport or persistent CRDT storage.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_OPERATIONS: usize = 2_048;
pub const MAX_ACTORS: usize = 64;
pub const MAX_VALUE_BYTES: usize = 4_096;
pub const MAX_HISTORY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VectorClock {
    pub entries: BTreeMap<String, u64>,
}

impl VectorClock {
    pub fn get(&self, actor: &str) -> u64 {
        self.entries.get(actor).copied().unwrap_or(0)
    }

    /// Inclusive dominance. Equality is dominance, not concurrency.
    pub fn dominates(&self, other: &Self) -> bool {
        other.entries.iter().all(|(actor, count)| self.get(actor) >= *count)
    }

    pub fn strictly_dominates(&self, other: &Self) -> bool {
        self.dominates(other)
            && self.entries.iter().any(|(actor, count)| *count > other.get(actor))
    }

    fn observe(&mut self, actor: &str, counter: u64) {
        let value = self.entries.entry(actor.to_owned()).or_insert(0);
        *value = (*value).max(counter);
    }

    fn merge(&mut self, other: &Self) {
        for (actor, counter) in &other.entries {
            self.observe(actor, *counter);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrdtOperation {
    pub tenant: String,
    pub actor: String,
    pub counter: u64,
    pub key: String,
    /// None is a tombstone; an empty Vec is a valid nondeleted value.
    pub value: Option<Vec<u8>>,
    /// Causal context observed BEFORE this operation's own dot.
    pub context: VectorClock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrdtError {
    InvalidIdentity,
    InvalidOperation,
    InvalidCausality,
    TenantMismatch,
    IdentityCollision,
    CapacityExceeded,
    CounterExhausted,
}

fn valid_id(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:-/".contains(&b))
}

impl CrdtOperation {
    fn validate(&self) -> Result<(), CrdtError> {
        if !valid_id(&self.tenant, 128)
            || !valid_id(&self.actor, 128)
            || !valid_id(&self.key, 256)
        {
            return Err(CrdtError::InvalidIdentity);
        }
        if self.counter == 0
            || self.value.as_ref().is_some_and(|v| v.len() > MAX_VALUE_BYTES)
            || self.context.entries.len() > MAX_ACTORS
            || self.context.entries.iter().any(|(actor, count)| {
                !valid_id(actor, 128) || *count == 0
            })
        {
            return Err(CrdtError::InvalidOperation);
        }
        if self.context.get(&self.actor).checked_add(1) != Some(self.counter) {
            return Err(CrdtError::InvalidCausality);
        }
        Ok(())
    }

    fn estimated_bytes(&self) -> Option<usize> {
        let mut size = self.tenant.len().checked_add(self.actor.len())?
            .checked_add(self.key.len())?
            .checked_add(self.value.as_ref().map_or(0, Vec::len))?
            .checked_add(32)?;
        for actor in self.context.entries.keys() {
            size = size.checked_add(actor.len())?.checked_add(16)?;
        }
        Some(size)
    }
}

#[derive(Clone, Debug)]
pub struct CrdtStateStore {
    tenant: String,
    operations: BTreeMap<(String, u64), CrdtOperation>,
    observed: VectorClock,
    history_bytes: usize,
}

impl CrdtStateStore {
    pub fn new(tenant: &str) -> Result<Self, CrdtError> {
        if !valid_id(tenant, 128) {
            return Err(CrdtError::InvalidIdentity);
        }
        Ok(Self {
            tenant: tenant.to_owned(),
            operations: BTreeMap::new(),
            observed: VectorClock::default(),
            history_bytes: 0,
        })
    }

    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    pub fn observed_clock(&self) -> &VectorClock {
        &self.observed
    }

    /// Idempotent for identical dots, fail-closed for conflicting reuse.
    /// Rejection never changes stored operations or the vector clock.
    pub fn apply_operation(&mut self, op: CrdtOperation) -> Result<ApplyOutcome, CrdtError> {
        op.validate()?;
        if op.tenant != self.tenant {
            return Err(CrdtError::TenantMismatch);
        }
        let identity = (op.actor.clone(), op.counter);
        if let Some(previous) = self.operations.get(&identity) {
            return if previous == &op {
                Ok(ApplyOutcome::Duplicate)
            } else {
                Err(CrdtError::IdentityCollision)
            };
        }
        if self.operations.len() >= MAX_OPERATIONS {
            return Err(CrdtError::CapacityExceeded);
        }
        let additional_actors = op.context.entries.keys()
            .chain(std::iter::once(&op.actor))
            .filter(|actor| !self.observed.entries.contains_key(*actor))
            .collect::<BTreeSet<_>>()
            .len();
        if self.observed.entries.len() + additional_actors > MAX_ACTORS {
            return Err(CrdtError::CapacityExceeded);
        }
        let size = op.estimated_bytes().ok_or(CrdtError::CapacityExceeded)?;
        let total = self.history_bytes.checked_add(size)
            .ok_or(CrdtError::CapacityExceeded)?;
        if total > MAX_HISTORY_BYTES {
            return Err(CrdtError::CapacityExceeded);
        }
        // If both dots claim to have observed each other, their causal graph
        // is inconsistent. This check is necessary but not a substitute for
        // cryptographic authentication of the causal context.
        if self.operations.values().any(|existing| {
            op.context.get(&existing.actor) >= existing.counter
                && existing.context.get(&op.actor) >= op.counter
        }) {
            return Err(CrdtError::InvalidCausality);
        }
        self.observed.merge(&op.context);
        self.observed.observe(&op.actor, op.counter);
        self.history_bytes = total;
        self.operations.insert(identity, op);
        Ok(ApplyOutcome::Applied)
    }

    /// Create a local dotted operation from the current observed clock.
    /// Caller must authenticate the actor and authorize the key independently.
    pub fn apply_local(
        &mut self,
        actor: &str,
        key: &str,
        value: Option<Vec<u8>>,
    ) -> Result<CrdtOperation, CrdtError> {
        if !valid_id(actor, 128) || !valid_id(key, 256) {
            return Err(CrdtError::InvalidIdentity);
        }
        let counter = self.observed.get(actor).checked_add(1)
            .ok_or(CrdtError::CounterExhausted)?;
        let op = CrdtOperation {
            tenant: self.tenant.clone(),
            actor: actor.to_owned(),
            counter,
            key: key.to_owned(),
            value,
            context: self.observed.clone(),
        };
        self.apply_operation(op.clone())?;
        Ok(op)
    }

    /// A causal successor dominates its predecessor regardless of delivery
    /// order. Among maximal concurrent dots, a stable (counter,actor) order
    /// selects a presentation value. The operation history is retained.
    fn winner(&self, key: &str) -> Option<&CrdtOperation> {
        let mut winner: Option<&CrdtOperation> = None;
        for candidate in self.operations.values().filter(|op| op.key == key) {
            let superseded = self.operations.values().any(|other| {
                other.key == key
                    && (other.actor != candidate.actor || other.counter != candidate.counter)
                    && other.context.get(&candidate.actor) >= candidate.counter
            });
            if superseded {
                continue;
            }
            if winner.is_none_or(|current| {
                (candidate.counter, candidate.actor.as_str())
                    > (current.counter, current.actor.as_str())
            }) {
                winner = Some(candidate);
            }
        }
        winner
    }

    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.winner(key).and_then(|op| op.value.as_deref())
    }

    pub fn is_tombstoned(&self, key: &str) -> bool {
        self.winner(key).is_some_and(|op| op.value.is_none())
    }

    /// Materialize all keys in one O(N²) pass, not one full causal scan per
    /// key (which would be O(N³) for a many-key history).
    pub fn visible_snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut winners: BTreeMap<&str, &CrdtOperation> = BTreeMap::new();
        for candidate in self.operations.values() {
            let superseded = self.operations.values().any(|other| {
                other.key == candidate.key
                    && (other.actor != candidate.actor || other.counter != candidate.counter)
                    && other.context.get(&candidate.actor) >= candidate.counter
            });
            if superseded {
                continue;
            }
            let replace = winners.get(candidate.key.as_str()).is_none_or(|current| {
                (candidate.counter, candidate.actor.as_str())
                    > (current.counter, current.actor.as_str())
            });
            if replace {
                winners.insert(candidate.key.as_str(), candidate);
            }
        }
        winners.into_iter().filter_map(|(key, op)| {
            op.value.as_ref().map(|value| (key.to_owned(), value.clone()))
        }).collect()
    }

    /// Atomic in-memory reconciliation: a collision or quota error cannot
    /// partially import another replica's history.
    pub fn merge_from(&mut self, other: &Self) -> Result<(), CrdtError> {
        if self.tenant != other.tenant {
            return Err(CrdtError::TenantMismatch);
        }
        let mut candidate = self.clone();
        for op in other.operations.values() {
            candidate.apply_operation(op.clone())?;
        }
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(actor: &str, counter: u64, key: &str, value: Option<&[u8]>,
        context: &[(&str, u64)]) -> CrdtOperation {
        CrdtOperation {
            tenant: "tenant-a".into(),
            actor: actor.into(),
            counter,
            key: key.into(),
            value: value.map(<[u8]>::to_vec),
            context: VectorClock {
                entries: context.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect(),
            },
        }
    }

    #[test]
    fn equal_clocks_dominate_but_are_not_strictly_ordered() {
        let mut a = VectorClock::default();
        a.entries.insert("a".into(), 1);
        assert!(a.dominates(&a));
        assert!(!a.strictly_dominates(&a));
        let mut b = a.clone();
        b.entries.insert("b".into(), 2);
        assert!(b.strictly_dominates(&a));
        assert!(!a.dominates(&b));
    }

    #[test]
    fn causal_tombstone_wins_even_if_delivered_before_original() {
        let a = operation("a", 1, "key", Some(b"value"), &[]);
        let b = operation("b", 1, "key", None, &[("a", 1)]);
        let mut left = CrdtStateStore::new("tenant-a").unwrap();
        let mut right = CrdtStateStore::new("tenant-a").unwrap();
        left.apply_operation(a.clone()).unwrap();
        left.apply_operation(b.clone()).unwrap();
        right.apply_operation(b).unwrap();
        right.apply_operation(a).unwrap();
        assert!(left.is_tombstoned("key"));
        assert!(right.is_tombstoned("key"));
        assert_eq!(left.visible_snapshot(), right.visible_snapshot());
    }

    #[test]
    fn concurrent_values_converge_independently_of_arrival_order() {
        let a = operation("a", 1, "k", Some(b"A"), &[]);
        let b = operation("b", 1, "k", Some(b"B"), &[]);
        let mut left = CrdtStateStore::new("tenant-a").unwrap();
        let mut right = CrdtStateStore::new("tenant-a").unwrap();
        left.apply_operation(a.clone()).unwrap();
        left.apply_operation(b.clone()).unwrap();
        right.apply_operation(b).unwrap();
        right.apply_operation(a).unwrap();
        assert_eq!(left.get("k"), Some(b"B".as_slice()));
        assert_eq!(left.visible_snapshot(), right.visible_snapshot());
    }

    #[test]
    fn causal_maxima_are_not_lost_to_unrelated_high_counter() {
        let old = operation("a", 10, "k", Some(b"old"), &[("a", 9)]);
        let newer = operation("b", 1, "k", Some(b"new"), &[("a", 10)]);
        let parallel = operation("c", 1, "k", Some(b"parallel"), &[]);
        let mut first = CrdtStateStore::new("tenant-a").unwrap();
        let mut second = CrdtStateStore::new("tenant-a").unwrap();
        for op in [old.clone(), newer.clone(), parallel.clone()] {
            first.apply_operation(op).unwrap();
        }
        for op in [parallel, newer, old] {
            second.apply_operation(op).unwrap();
        }
        assert_eq!(first.get("k"), Some(b"parallel".as_slice()));
        assert_eq!(first.visible_snapshot(), second.visible_snapshot());
    }

    #[test]
    fn duplicate_is_idempotent_and_dot_collision_fails_closed() {
        let mut store = CrdtStateStore::new("tenant-a").unwrap();
        let op = operation("a", 1, "k", Some(b"first"), &[]);
        assert_eq!(store.apply_operation(op.clone()), Ok(ApplyOutcome::Applied));
        assert_eq!(store.apply_operation(op.clone()), Ok(ApplyOutcome::Duplicate));
        let mut tampered = op;
        tampered.value = Some(b"second".to_vec());
        assert_eq!(store.apply_operation(tampered), Err(CrdtError::IdentityCollision));
        assert_eq!(store.operation_count(), 1);
        assert_eq!(store.get("k"), Some(b"first".as_slice()));
    }

    #[test]
    fn tenant_isolation_and_bad_context_rejected_without_mutation() {
        let mut store = CrdtStateStore::new("tenant-a").unwrap();
        let mut op = operation("a", 1, "k", Some(b"v"), &[]);
        op.tenant = "tenant-b".into();
        assert_eq!(store.apply_operation(op), Err(CrdtError::TenantMismatch));
        let bad = operation("a", 2, "k", Some(b"v"), &[]);
        assert_eq!(store.apply_operation(bad), Err(CrdtError::InvalidCausality));
        assert_eq!(store.operation_count(), 0);
    }

    #[test]
    fn three_replica_partition_heal_is_convergent() {
        let mut a = CrdtStateStore::new("tenant-a").unwrap();
        let mut b = CrdtStateStore::new("tenant-a").unwrap();
        let mut c = CrdtStateStore::new("tenant-a").unwrap();
        a.apply_local("actor-a", "doc", Some(b"A".to_vec())).unwrap();
        b.apply_local("actor-b", "doc", Some(b"B".to_vec())).unwrap();
        c.apply_local("actor-c", "other", Some(b"C".to_vec())).unwrap();
        a.merge_from(&b).unwrap();
        c.merge_from(&a).unwrap();
        b.merge_from(&c).unwrap();
        a.merge_from(&b).unwrap();
        assert_eq!(a.visible_snapshot(), b.visible_snapshot());
        assert_eq!(b.visible_snapshot(), c.visible_snapshot());
    }

    #[test]
    fn merge_collision_is_atomic() {
        let mut a = CrdtStateStore::new("tenant-a").unwrap();
        let mut b = CrdtStateStore::new("tenant-a").unwrap();
        a.apply_operation(operation("a", 1, "k", Some(b"A"), &[])).unwrap();
        b.apply_operation(operation("a", 1, "k", Some(b"X"), &[])).unwrap();
        b.apply_operation(operation("b", 1, "new", Some(b"Y"), &[])).unwrap();
        let before = a.visible_snapshot();
        assert_eq!(a.merge_from(&b), Err(CrdtError::IdentityCollision));
        assert_eq!(a.visible_snapshot(), before);
        assert_eq!(a.operation_count(), 1);
    }

    #[test]
    fn bounds_and_counter_exhaustion_are_fail_closed() {
        let mut store = CrdtStateStore::new("tenant-a").unwrap();
        assert_eq!(
            store.apply_local("a", "key", Some(vec![1; MAX_VALUE_BYTES + 1])),
            Err(CrdtError::InvalidOperation)
        );
        assert_eq!(store.operation_count(), 0);
        let extreme = operation("a", u64::MAX, "key", Some(b"x"), &[("a", u64::MAX - 1)]);
        store.apply_operation(extreme).unwrap();
        assert_eq!(store.apply_local("a", "key", None), Err(CrdtError::CounterExhausted));
    }

    #[test]
    fn explicit_zero_entry_is_not_strict_dominance() {
        let empty = VectorClock::default();
        let mut explicit_zero = VectorClock::default();
        explicit_zero.entries.insert("a".into(), 0);
        assert!(explicit_zero.dominates(&empty));
        assert!(!explicit_zero.strictly_dominates(&empty));
    }

    #[test]
    fn many_distinct_keys_materialize_without_repeated_full_scans() {
        let mut store = CrdtStateStore::new("tenant-a").unwrap();
        for i in 0..128 {
            let key = format!("key_{i}");
            store.apply_local("actor", &key, Some(vec![i as u8])).unwrap();
        }
        let snapshot = store.visible_snapshot();
        assert_eq!(snapshot.len(), 128);
        assert_eq!(snapshot.get("key_127"), Some(&vec![127]));
    }

    #[test]
    fn conflicting_causal_claims_are_rejected() {
        let mut store = CrdtStateStore::new("tenant-a").unwrap();
        let a = operation("a", 1, "k", Some(b"a"), &[("b", 1)]);
        let b = operation("b", 1, "k", Some(b"b"), &[("a", 1)]);
        store.apply_operation(a).unwrap();
        assert_eq!(store.apply_operation(b), Err(CrdtError::InvalidCausality));
        assert_eq!(store.operation_count(), 1);
    }
}
