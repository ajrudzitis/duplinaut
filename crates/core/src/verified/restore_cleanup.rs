//! Atomic admission of one cleanup operation against its reserved work budget.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanupAttemptState {
    pub remaining: usize,
    pub attempted: usize,
}

pub open spec fn cleanup_attempt_model(
    remaining: usize,
    attempted: usize,
) -> Option<CleanupAttemptState> {
    if remaining > 0 && attempted < usize::MAX {
        Some(CleanupAttemptState {
            remaining: (remaining - 1) as usize,
            attempted: (attempted + 1) as usize,
        })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#restore-012
//= type=implementation
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
//= docs/parsing-requirements.md#restore-022
//= type=implementation
//# On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.
pub(crate) fn admit_cleanup_attempt(
    remaining: usize,
    attempted: usize,
) -> (result: Option<CleanupAttemptState>)
    ensures result == cleanup_attempt_model(remaining, attempted),
{
    if remaining == 0 {
        return None;
    }
    let next_attempted = attempted.checked_add(1)?;
    let next_remaining = remaining.checked_sub(1)?;
    Some(CleanupAttemptState {
        remaining: next_remaining,
        attempted: next_attempted,
    })
}

}
