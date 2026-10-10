//! Exact envelope-specific footer predicate. The whole tar controller is not
//! yet refined; callers must prove the entry-stream cursor they pass here.
// Explicit scalar operations preserve the Verus contracts and solver-visible bounds.
#![allow(clippy::len_zero, clippy::manual_is_multiple_of)]
use vstd::prelude::*;
verus! {
pub open spec fn zero_range(input: Seq<u8>, begin: int, end: int) -> bool {
    0 <= begin <= end <= input.len()
        && forall|i: int| begin <= i < end ==> input[i] == 0
}
pub open spec fn footer_model(input: Seq<u8>, cursor: int, mode: u8,
    stored: Seq<u8>, nonfinal: bool) -> bool
{
    if !(0 <= cursor <= input.len()) { false }
    else if mode == 0 || mode == 1 {
        (mode == 0 || cursor < input.len() || input.len() == 0)
            && (input.len() - cursor) % 512 == 0
            && zero_range(input, cursor, input.len() as int)
    } else if mode == 2 {
        let zero_end = cursor + 10240 - cursor % 10240;
        zero_end <= input.len() && zero_range(input, cursor, zero_end)
            && (zero_end == input.len() || (nonfinal
                && input.len() - zero_end <= stored.len()
                && forall|i: int| 0 <= i < input.len() - zero_end
                    ==> input[zero_end + i] == stored[i]))
    } else { false }
}
pub fn zeros(input: &[u8], begin: usize, end: usize) -> (result: bool)
    requires begin <= end <= input.len(),
    ensures result == zero_range(input@, begin as int, end as int),
{
    let mut i = begin;
    while i < end
        invariant begin <= i <= end <= input.len(),
            forall|j: int| begin <= j < i ==> input@[j] == 0,
        decreases end - i,
    {
        if input[i] != 0 { return false; }
        i += 1;
    }
    true
}
pub fn footer_valid(input: &[u8], cursor: usize, mode: u8,
    stored: &[u8], nonfinal: bool) -> (result: bool)
    requires cursor <= input.len(),
    ensures result == footer_model(input@, cursor as int, mode, stored@, nonfinal),
{
    if mode == 0 || mode == 1 {
        if mode == 1 && cursor == input.len() && input.len() != 0 { return false; }
        if (input.len() - cursor) % 512 != 0 { return false; }
        return zeros(input, cursor, input.len());
    }
    if mode != 2 { return false; }
    let count = 10240 - cursor % 10240;
    if input.len() - cursor < count { return false; }
    let end = cursor + count;
    if !zeros(input, cursor, end) { return false; }
    if end == input.len() { return true; }
    if !nonfinal || input.len() - end > stored.len() { return false; }
    let mut i = 0usize;
    while i < input.len() - end
        invariant end <= input.len(), input.len() - end <= stored.len(),
            cursor <= input.len(), mode == 2, nonfinal,
            end as int == cursor as int + 10240 - cursor as int % 10240,
            zero_range(input@, cursor as int, end as int),
            i <= input.len() - end,
            forall|j: int| 0 <= j < i ==> input@[end as int + j] == stored@[j],
        decreases input.len() - end - i,
    {
        if input[end + i] != stored[i] {
            assert(input@[end as int + i as int] != stored@[i as int]);
            assert(!(forall|j: int| 0 <= j < input.len() - end
                ==> input@[end as int + j] == stored@[j]));
            return false;
        }
        i += 1;
    }
    true
}
}
