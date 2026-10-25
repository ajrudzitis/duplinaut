//! Ordered completion cursor for selected repository sets.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetCompletion {
    pub planned: usize,
    pub completed: usize,
}

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

}

#[cfg(test)]
mod tests {
    use super::{advance_set_completion, set_completion_complete, start_set_completion};

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
}
