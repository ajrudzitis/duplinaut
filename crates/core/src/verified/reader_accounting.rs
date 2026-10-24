//! Bounded per-object accounting step used by the repository volume matcher.
//! This proves one controller transition, not the whole reader or total heap use.
use vstd::prelude::*;

verus! {

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
pub fn charge_object_check(checked: usize, limit: usize) -> (result: Option<usize>)
    ensures match result {
        Some(next) => checked < limit && next as int == checked as int + 1 && next <= limit,
        None => checked >= limit || checked == usize::MAX,
    },
{
    if checked >= limit {
        None
    } else {
        match checked.checked_add(1) {
            Some(next) if next <= limit => Some(next),
            _ => None,
        }
    }
}
}
