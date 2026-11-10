//! Optimistic concurrency task state machine with fail-closed fencing.
//! An in-memory aggregate is NOT a durable distributed CAS. Persist the
//! revision, state and receipt in one database transaction before publishing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,
    Queued,
    Leased,
    Running,
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAggregate {
    pub task_id: String,
    pub state: TaskState,
    pub revision: u64,
    pub fencing_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionReceipt {
    pub task_id: String,
    pub previous: TaskState,
    pub next: TaskState,
    pub previous_revision: u64,
    pub new_revision: u64,
    pub fencing_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasError {
    StaleRevision,
    StaleFencingEpoch,
    InvalidTransition,
    RevisionExhausted,
}

impl TaskAggregate {
    /// Apply one state transition with optimistic revision and fencing checks.
    /// Rejection must not mutate any aggregate field.
    pub fn transition_cas(
        &mut self,
        expected_revision: u64,
        authoritative_epoch: u64,
        next: TaskState,
    ) -> Result<TransitionReceipt, CasError> {
        if expected_revision != self.revision {
            return Err(CasError::StaleRevision);
        }
        if authoritative_epoch != self.fencing_epoch {
            return Err(CasError::StaleFencingEpoch);
        }
        if !valid_transition(self.state, next) {
            return Err(CasError::InvalidTransition);
        }
        let new_revision = self.revision.checked_add(1).ok_or(CasError::RevisionExhausted)?;
        let receipt = TransitionReceipt {
            task_id: self.task_id.clone(),
            previous: self.state,
            next,
            previous_revision: self.revision,
            new_revision,
            fencing_epoch: self.fencing_epoch,
        };
        self.state = next;
        self.revision = new_revision;
        Ok(receipt)
    }

    /// The authority layer must durably establish the new epoch first.
    /// This method only updates an in-memory replica after that decision.
    pub fn observe_fencing_epoch(&mut self, authoritative_epoch: u64) -> Result<(), CasError> {
        if authoritative_epoch <= self.fencing_epoch {
            return Err(CasError::StaleFencingEpoch);
        }
        self.fencing_epoch = authoritative_epoch;
        Ok(())
    }
}

fn valid_transition(from: TaskState, to: TaskState) -> bool {
    use TaskState::*;
    matches!(
        (from, to),
        (Created, Queued)
            | (Queued, Leased)
            | (Leased, Running)
            | (Running, Verifying)
            | (Verifying, Succeeded)
            | (Verifying, Failed)
            | (Running, Failed)
            | (Leased, Queued)
            | (Created | Queued | Leased | Running | Verifying, Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregate() -> TaskAggregate {
        TaskAggregate {
            task_id: "tenant-a/task-1".to_owned(),
            state: TaskState::Created,
            revision: 0,
            fencing_epoch: 7,
        }
    }

    #[test]
    fn accepted_cas_preserves_old_state_in_receipt() {
        let mut task = aggregate();
        let receipt = task.transition_cas(0, 7, TaskState::Queued).unwrap();
        assert_eq!(receipt.previous, TaskState::Created);
        assert_eq!(receipt.next, TaskState::Queued);
        assert_eq!(receipt.previous_revision, 0);
        assert_eq!(receipt.new_revision, 1);
        assert_eq!(task.revision, 1);
    }

    #[test]
    fn stale_cas_and_stale_epoch_leave_aggregate_unchanged() {
        let mut task = aggregate();
        let snapshot = task.clone();
        assert_eq!(task.transition_cas(1, 7, TaskState::Queued), Err(CasError::StaleRevision));
        assert_eq!(task, snapshot);
        assert_eq!(task.transition_cas(0, 6, TaskState::Queued), Err(CasError::StaleFencingEpoch));
        assert_eq!(task, snapshot);
    }

    #[test]
    fn terminal_state_cannot_be_resurrected() {
        let mut task = aggregate();
        task.transition_cas(0, 7, TaskState::Cancelled).unwrap();
        let snapshot = task.clone();
        assert_eq!(task.transition_cas(1, 7, TaskState::Queued), Err(CasError::InvalidTransition));
        assert_eq!(task, snapshot);
    }

    #[test]
    fn revision_overflow_is_fail_closed() {
        let mut task = aggregate();
        task.revision = u64::MAX;
        let snapshot = task.clone();
        assert_eq!(task.transition_cas(u64::MAX, 7, TaskState::Queued), Err(CasError::RevisionExhausted));
        assert_eq!(task, snapshot);
    }

    #[test]
    fn competing_writers_only_first_revision_wins() {
        let mut task = aggregate();
        assert!(task.transition_cas(0, 7, TaskState::Queued).is_ok());
        assert_eq!(task.transition_cas(0, 7, TaskState::Cancelled), Err(CasError::StaleRevision));
    }

    #[test]
    fn epoch_is_strictly_monotonic() {
        let mut task = aggregate();
        assert_eq!(task.observe_fencing_epoch(7), Err(CasError::StaleFencingEpoch));
        task.observe_fencing_epoch(8).unwrap();
        assert_eq!(task.transition_cas(0, 7, TaskState::Queued), Err(CasError::StaleFencingEpoch));
        assert!(task.transition_cas(0, 8, TaskState::Queued).is_ok());
    }
}
