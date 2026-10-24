//! Checked forward-work counter transition for the native Linux restorer.
use vstd::prelude::*;

verus! {

pub open spec fn forward_work_model(left: usize) -> Option<usize> {
    if left == 0 { None } else { Some((left - 1) as usize) }
}

//= docs/parsing-requirements.md#restore-012
//= type=implementation
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
pub(crate) fn decrement_forward_work(left: usize) -> (result: Option<usize>)
    ensures result == forward_work_model(left),
{
    left.checked_sub(1)
}

}
