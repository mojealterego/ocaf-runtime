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


/// A chronological focus change within one authenticated tenant's object.
/// This never establishes human/agent intention or ownership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusTransition {
    pub revision: u64,
    pub actor: String,
    pub from_focus: String,
    pub to_focus: String,
}

/// Deterministic advisory analytics. Scores are weighted contribution shares,
/// NOT legal ownership percentages or evidence of human intent.
/// Diversity is normalized Gini-Simpson scaled to 0..=10_000 basis points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectAnalytics {
    pub event_count: usize,
    pub contribution_basis_points: BTreeMap<String, u16>,
    pub diversity_basis_points: u16,
    pub focus_transitions: Vec<FocusTransition>,
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
    /// Compute bounded, per-object, read-only collaboration signals.
    /// The trusted caller must supply tenant identity from authenticated ingress.
    /// Values are reproducible on the same recorded event sequence; they cannot
    /// authorize state changes, certify evidence, or identify legal owners.
    pub fn analyze_object(
        &self,
        tenant: &str,
        object: &str,
    ) -> Result<ObjectAnalytics, TrackerError> {
        let events = self.history(tenant, object)?;
        if events.is_empty() {
            return Ok(ObjectAnalytics {
                event_count: 0,
                contribution_basis_points: BTreeMap::new(),
                diversity_basis_points: 0,
                focus_transitions: Vec::new(),
            });
        }

        let mut actor_weights: BTreeMap<String, u64> = BTreeMap::new();
        for event in &events {
            let points = match event.operation {
                CognitiveOperation::Propose => 15,
                CognitiveOperation::Modify => 10,
                CognitiveOperation::Validate => 8,
                CognitiveOperation::Challenge => 9,
                CognitiveOperation::Synthesize => 14,
            };
            *actor_weights.entry(event.actor.clone()).or_default() += points;
        }
        let total: u64 = actor_weights.values().sum();
        let mut ranked: Vec<(String, u16, u64)> = actor_weights
            .iter()
            .map(|(actor, weight)| {
                let numerator = weight * 10_000;
                (actor.clone(), (numerator / total) as u16, numerator % total)
            })
            .collect();
        let allocated: u64 = ranked.iter().map(|(_, points, _)| u64::from(*points)).sum();
        let remaining = (10_000 - allocated) as usize;
        // Largest-remainder apportionment yields exactly 10,000 basis points.
        // Ties break lexically on actor IDs, independent of event arrival order.
        ranked.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        for (_, points, _) in ranked.iter_mut().take(remaining) {
            *points += 1;
        }
        let contribution_basis_points = ranked
            .into_iter()
            .map(|(actor, points, _)| (actor, points))
            .collect();

        let actors = actor_weights.len() as u128;
        let total_squared = u128::from(total) * u128::from(total);
        let squared_sum: u128 = actor_weights
            .values()
            .map(|w| u128::from(*w) * u128::from(*w))
            .sum();
        let diversity_basis_points = if actors > 1 {
            let numerator = (total_squared - squared_sum) * actors * 10_000;
            let denominator = total_squared * (actors - 1);
            ((numerator + denominator / 2) / denominator).min(10_000) as u16
        } else {
            0
        };
        let focus_transitions = events
            .windows(2)
            .filter(|pair| pair[0].focus != pair[1].focus)
            .map(|pair| FocusTransition {
                revision: pair[1].revision,
                actor: pair[1].actor.clone(),
                from_focus: pair[0].focus.clone(),
                to_focus: pair[1].focus.clone(),
            })
            .collect();
        Ok(ObjectAnalytics {
            event_count: events.len(),
            contribution_basis_points,
            diversity_basis_points,
            focus_transitions,
        })
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

    #[test]
    fn focus_and_weighted_contribution_are_tenant_isolated() {
        let mut tracker = CollaborationTracker::default();
        let hash = "a".repeat(64);
        let human = TrustedActor {
            tenant: "tenant1".into(),
            actor: "human".into(),
        };
        let ai = TrustedActor {
            tenant: "tenant1".into(),
            actor: "agent".into(),
        };
        tracker.append(&human, "shared", 0, CognitiveOperation::Propose, "design", &hash).unwrap();
        tracker.append(&ai, "shared", 1, CognitiveOperation::Validate, "security", &hash).unwrap();
        tracker.append(&ai, "shared", 2, CognitiveOperation::Modify, "security", &hash).unwrap();

        let analysis = tracker.analyze_object("tenant1", "shared").unwrap();
        assert_eq!(analysis.event_count, 3);
        assert_eq!(
            analysis.contribution_basis_points.values().map(|x| u64::from(*x)).sum::<u64>(),
            10_000
        );
        // human=15, agent=18. Weighted scores are heuristics, not ownership.
        assert_eq!(analysis.contribution_basis_points["human"], 4545);
        assert_eq!(analysis.contribution_basis_points["agent"], 5455);
        assert!(analysis.diversity_basis_points > 0);
        assert!(analysis.diversity_basis_points <= 10_000);
        assert_eq!(analysis.focus_transitions.len(), 1);
        assert_eq!(analysis.focus_transitions[0].revision, 2);
        assert_eq!(analysis.focus_transitions[0].from_focus, "design");
        assert_eq!(analysis.focus_transitions[0].to_focus, "security");
        assert_eq!(analysis.focus_transitions[0].actor, "agent");

        assert_eq!(tracker.analyze_object("tenant2", "shared").unwrap().event_count, 0);
    }

    #[test]
    fn empty_and_single_actor_analytics_never_claim_collaboration() {
        let mut tracker = CollaborationTracker::default();
        let empty = tracker.analyze_object("tenant1", "object1").unwrap();
        assert_eq!(empty.event_count, 0);
        assert_eq!(empty.diversity_basis_points, 0);
        assert!(empty.contribution_basis_points.is_empty());
        let hash = "b".repeat(64);
        tracker.append(&actor("tenant1"), "object1", 0, CognitiveOperation::Propose, "same", &hash).unwrap();
        tracker.append(&actor("tenant1"), "object1", 1, CognitiveOperation::Modify, "same", &hash).unwrap();
        let a = tracker.analyze_object("tenant1", "object1").unwrap();
        assert_eq!(a.contribution_basis_points["agent-1"], 10_000);
        assert_eq!(a.diversity_basis_points, 0);
        assert!(a.focus_transitions.is_empty());
    }

    #[test]
    fn equal_contributions_have_exact_basis_point_total_and_stable_ties() {
        let mut tracker = CollaborationTracker::default();
        let hash = "c".repeat(64);
        for (revision, id) in ["bravo", "alpha", "charlie"].iter().enumerate() {
            tracker.append(
                &TrustedActor { tenant: "tenant1".into(), actor: (*id).into() },
                "shared", revision as u64, CognitiveOperation::Propose, "shared", &hash
            ).unwrap();
        }
        let one = tracker.analyze_object("tenant1", "shared").unwrap();
        let two = tracker.analyze_object("tenant1", "shared").unwrap();
        assert_eq!(one, two);
        assert_eq!(one.contribution_basis_points["alpha"], 3334);
        assert_eq!(one.contribution_basis_points["bravo"], 3333);
        assert_eq!(one.contribution_basis_points["charlie"], 3333);
        assert_eq!(one.diversity_basis_points, 10_000);
    }

    #[test]
    fn analytics_rejects_invalid_context_without_disclosing_objects() {
        let tracker = CollaborationTracker::default();
        assert_eq!(tracker.analyze_object("../tenant", "shared"), Err(TrackerError::Invalid));
        assert_eq!(tracker.analyze_object("tenant1", "../shared"), Err(TrackerError::Invalid));
    }

}
