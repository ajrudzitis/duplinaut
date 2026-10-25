//! Ordered completion cursor for selected repository sets.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetCompletion {
    pub planned: usize,
    pub completed: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotPublicationReady;

pub open spec fn advance_set_completion_model(
    current: SetCompletion,
    position: usize,
    state_transition_succeeded: bool,
) -> Option<SetCompletion> {
    let next = current.completed as int + 1;
    if state_transition_succeeded
        && current.completed <= current.planned
        && current.completed < current.planned
        && position == current.completed
        && next <= usize::MAX as int
    {
        Some(SetCompletion {
            planned: current.planned,
            completed: next as usize,
        })
    } else {
        None
    }
}

pub open spec fn snapshot_publication_readiness_model(
    completion: SetCompletion,
    state_exists: bool,
    cancelled: bool,
) -> Option<SnapshotPublicationReady> {
    if completion.completed == completion.planned && state_exists && !cancelled {
        Some(SnapshotPublicationReady)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
pub fn start_set_completion(planned: usize) -> (result: SetCompletion)
    ensures result.planned == planned && result.completed == 0,
{
    SetCompletion {
        planned,
        completed: 0,
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
pub fn advance_set_completion(
    current: SetCompletion,
    position: usize,
    state_transition_succeeded: bool,
) -> (result: Option<SetCompletion>)
    ensures result == advance_set_completion_model(current, position, state_transition_succeeded),
{
    if !state_transition_succeeded
        || current.completed > current.planned
        || current.completed >= current.planned
        || position != current.completed
    {
        return None;
    }
    let completed = current.completed.checked_add(1)?;
    Some(SetCompletion {
        planned: current.planned,
        completed,
    })
}

// This gate consumes one cancellation observation made after a selected set's
// state transition. It does not make claims about AtomicBool timing or races.
//= docs/parsing-requirements.md#parse-env-024
//= type=implementation
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
pub fn commit_set_completion(
    current: SetCompletion,
    position: usize,
    state_transition_succeeded: bool,
    cancelled: bool,
) -> (result: Option<SetCompletion>)
    ensures result == if cancelled { None } else {
        advance_set_completion_model(current, position, state_transition_succeeded)
    },
{
    if cancelled { return None; }
    advance_set_completion(current, position, state_transition_succeeded)
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
pub fn set_completion_complete(current: SetCompletion) -> (result: bool)
    ensures result == (current.completed == current.planned),
{
    current.completed == current.planned
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
//= docs/parsing-requirements.md#parse-env-024
//= type=implementation
//# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
pub fn snapshot_publication_readiness(
    completion: SetCompletion,
    state_exists: bool,
    cancelled: bool,
) -> (result: Option<SnapshotPublicationReady>)
    ensures result == snapshot_publication_readiness_model(completion, state_exists, cancelled),
{
    if completion.completed != completion.planned || !state_exists || cancelled {
        return None;
    }
    Some(SnapshotPublicationReady)
}

}

#[cfg(test)]
mod tests {
    use super::{
        advance_set_completion, commit_set_completion, set_completion_complete,
        snapshot_publication_readiness, start_set_completion,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=test
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    //= docs/parsing-requirements.md#parse-env-024
    //= type=test
    //# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
    fn publication_readiness_requires_completion_state_and_clear_cancellation() {
        let complete = advance_set_completion(start_set_completion(1), 0, true).unwrap();
        assert!(snapshot_publication_readiness(complete, true, false).is_some());
        assert!(snapshot_publication_readiness(complete, false, false).is_none());
        assert!(snapshot_publication_readiness(complete, true, true).is_none());

        let incomplete = start_set_completion(1);
        assert!(snapshot_publication_readiness(incomplete, true, false).is_none());
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=test
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    fn set_completion_cursor_requires_every_successful_ordered_transition() {
        let mut cursor = start_set_completion(2);
        assert!(!set_completion_complete(cursor));
        cursor = advance_set_completion(cursor, 0, true).unwrap();
        assert!(!set_completion_complete(cursor));
        cursor = advance_set_completion(cursor, 1, true).unwrap();
        assert!(set_completion_complete(cursor));
        assert!(advance_set_completion(cursor, 2, true).is_none());
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=test
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    fn set_completion_cursor_rejects_skips_reordering_and_failed_transition() {
        let initial = start_set_completion(3);
        assert!(advance_set_completion(initial, 1, true).is_none());
        assert!(advance_set_completion(initial, 0, false).is_none());
        let after_first = advance_set_completion(initial, 0, true).unwrap();
        assert!(advance_set_completion(after_first, 2, true).is_none());
        assert!(advance_set_completion(after_first, 0, true).is_none());
    }

    #[test]
    //= docs/parsing-requirements.md#parse-env-024
    //= type=test
    //# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
    fn set_completion_commit_requires_clear_post_transition_cancellation_observation() {
        let initial = start_set_completion(2);
        assert!(commit_set_completion(initial, 0, true, true).is_none());
        assert!(commit_set_completion(initial, 1, true, false).is_none());
        assert!(commit_set_completion(initial, 0, false, false).is_none());
        assert_eq!(
            commit_set_completion(initial, 0, true, false),
            advance_set_completion(initial, 0, true)
        );
    }
}
