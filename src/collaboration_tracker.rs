//! Tenant-scoped, bounded collaborative object provenance (Argon source 2.02).
//! This is a local journal. Durable storage, authenticated ingress and signed
//! actor identities must be provided by the authoritative runtime.
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CognitiveOperation {
    Propose,
    Modify,
    Validate,
    Challenge,
    Synthesize,
}

#[derive(Clone, Debug)]
pub struct TrustedActor {
    pub tenant: String,
    pub actor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollaborationEvent {
    pub revision: u64,
    pub actor: String,
    pub operation: CognitiveOperation,
    pub focus: String,
    pub artifact_sha256: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrackerError {
    #[error("invalid tenant, actor, object, focus or artifact hash")]
    Invalid,
    #[error("stale object revision or replayed event")]
    Conflict,
    #[error("bounded provenance limit exceeded")]
    Limit,
}

#[derive(Default)]
pub struct CollaborationTracker {
    objects: BTreeMap<(String, String), Vec<CollaborationEvent>>,
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b))
}

impl CollaborationTracker {
    pub fn append(
        &mut self,
        actor: &TrustedActor,
        object: &str,
        expected_revision: u64,
        operation: CognitiveOperation,
        focus: &str,
        artifact_sha256: &str,
    ) -> Result<u64, TrackerError> {
        if !valid_id(&actor.tenant)
            || !valid_id(&actor.actor)
            || !valid_id(object)
            || !valid_id(focus)
            || artifact_sha256.len() != 64
            || !artifact_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(TrackerError::Invalid);
        }
        let key = (actor.tenant.clone(), object.to_string());
        let next_revision = self
            .objects
            .get(&key)
            .map_or(0, |events| events.len() as u64);
        if next_revision != expected_revision {
            return Err(TrackerError::Conflict);
        }
        if !self.objects.contains_key(&key) && self.objects.len() >= 1024 || next_revision >= 256 {
            return Err(TrackerError::Limit);
        }
        let event = CollaborationEvent {
            revision: expected_revision + 1,
            actor: actor.actor.clone(),
            operation,
            focus: focus.to_string(),
            artifact_sha256: artifact_sha256.to_ascii_lowercase(),
        };
        self.objects.entry(key).or_default().push(event);
        Ok(expected_revision + 1)
    }

    /// The trusted caller supplies tenant identity, never a model output.
    pub fn history(
        &self,
        tenant: &str,
        object: &str,
    ) -> Result<Vec<CollaborationEvent>, TrackerError> {
        if !valid_id(tenant) || !valid_id(object) {
            return Err(TrackerError::Invalid);
        }
        Ok(self
            .objects
            .get(&(tenant.to_string(), object.to_string()))
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(tenant: &str) -> TrustedActor {
        TrustedActor {
            tenant: tenant.into(),
            actor: "agent-1".into(),
        }
    }

    #[test]
    fn immutable_actor_history_isolated_by_tenant_and_fenced() {
        let mut tracker = CollaborationTracker::default();
        let digest = "a".repeat(64);
        assert_eq!(
            tracker.append(
                &actor("tenant-a"),
                "object1",
                0,
                CognitiveOperation::Propose,
                "design",
                &digest
            ),
            Ok(1)
        );
        assert_eq!(
            tracker.append(
                &actor("tenant-a"),
                "object1",
                0,
                CognitiveOperation::Modify,
                "design",
                &digest
            ),
            Err(TrackerError::Conflict)
        );
        assert!(tracker.history("tenant-b", "object1").unwrap().is_empty());
        assert_eq!(
            tracker.append(
                &actor("tenant-b"),
                "object1",
                0,
                CognitiveOperation::Challenge,
                "design",
                &digest
            ),
            Ok(1)
        );
        assert_eq!(tracker.history("tenant-a", "object1").unwrap().len(), 1);
        assert_eq!(tracker.history("tenant-b", "object1").unwrap().len(), 1);
    }

    #[test]
    fn malformed_and_unbounded_events_are_denied() {
        let mut tracker = CollaborationTracker::default();
        assert_eq!(
            tracker.append(
                &actor("tenant-a"),
                "object1",
                0,
                CognitiveOperation::Validate,
                "focus",
                "fake"
            ),
            Err(TrackerError::Invalid)
        );
        let digest = "1".repeat(64);
        assert_eq!(
            tracker.append(
                &actor("tenant-a"),
                "../",
                0,
                CognitiveOperation::Synthesize,
                "focus",
                &digest
            ),
            Err(TrackerError::Invalid)
        );
        for revision in 0..256 {
            tracker
                .append(
                    &actor("tenant-a"),
                    "object1",
                    revision,
                    CognitiveOperation::Validate,
                    "focus",
                    &digest,
                )
                .unwrap();
        }
        assert_eq!(
            tracker.append(
                &actor("tenant-a"),
                "object1",
                256,
                CognitiveOperation::Validate,
                "focus",
                &digest
            ),
            Err(TrackerError::Limit)
        );
    }
}
