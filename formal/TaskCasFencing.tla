---------------------------- MODULE TaskCasFencing ----------------------------
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS Tasks, EpochLimit, RevisionLimit
ASSUME /\ Tasks # {}
       /\ EpochLimit \in Nat
       /\ RevisionLimit \in Nat
       /\ EpochLimit > 0
       /\ RevisionLimit > 0

States == {"Created", "Queued", "Leased", "Running", "Verifying",
           "Succeeded", "Failed", "Cancelled"}
Terminal == {"Succeeded", "Failed", "Cancelled"}
Allowed == {
  <<"Created", "Queued">>,
  <<"Queued", "Leased">>,
  <<"Leased", "Running">>,
  <<"Running", "Verifying">>,
  <<"Verifying", "Succeeded">>,
  <<"Verifying", "Failed">>,
  <<"Running", "Failed">>,
  <<"Leased", "Queued">>,
  <<"Created", "Cancelled">>,
  <<"Queued", "Cancelled">>,
  <<"Leased", "Cancelled">>,
  <<"Running", "Cancelled">>,
  <<"Verifying", "Cancelled">>
}

VARIABLES state, revision, epoch, receipts
vars == <<state, revision, epoch, receipts>>

Init ==
  /\ state = [t \in Tasks |-> "Created"]
  /\ revision = [t \in Tasks |-> 0]
  /\ epoch = [t \in Tasks |-> 0]
  /\ receipts = {}

Transition(t, expectedRevision, expectedEpoch, nextState) ==
  /\ t \in Tasks
  /\ expectedRevision = revision[t]
  /\ expectedEpoch = epoch[t]
  /\ <<state[t], nextState>> \in Allowed
  /\ revision[t] < RevisionLimit
  /\ state' = [state EXCEPT ![t] = nextState]
  /\ revision' = [revision EXCEPT ![t] = @ + 1]
  /\ UNCHANGED epoch
  /\ receipts' = receipts \cup
       { [task |-> t, before |-> state[t], after |-> nextState,
          oldRevision |-> revision[t], newRevision |-> revision[t] + 1,
          fencing |-> epoch[t]] }

AdvanceEpoch(t, nextEpoch) ==
  /\ t \in Tasks
  /\ nextEpoch \in 0..EpochLimit
  /\ nextEpoch > epoch[t]
  /\ epoch' = [epoch EXCEPT ![t] = nextEpoch]
  /\ UNCHANGED <<state, revision, receipts>>

Next ==
  \/ \E t \in Tasks, r \in 0..RevisionLimit, e \in 0..EpochLimit,
        s \in States : Transition(t, r, e, s)
  \/ \E t \in Tasks, e \in 0..EpochLimit : AdvanceEpoch(t, e)

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ state \in [Tasks -> States]
  /\ revision \in [Tasks -> 0..RevisionLimit]
  /\ epoch \in [Tasks -> 0..EpochLimit]

ReceiptConsistency ==
  \A r \in receipts :
    /\ r.task \in Tasks
    /\ <<r.before, r.after>> \in Allowed
    /\ r.newRevision = r.oldRevision + 1
    /\ r.newRevision <= revision[r.task]
    /\ r.fencing <= epoch[r.task]

TerminalIrreversible ==
  \A t \in Tasks : state[t] \in Terminal =>
    ~\E s \in States : <<state[t], s>> \in Allowed

NoDuplicateRevision ==
  \A a, b \in receipts :
    (a.task = b.task /\ a.newRevision = b.newRevision) => a = b

=============================================================================
