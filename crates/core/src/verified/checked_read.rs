//! Exact remaining-byte transition for bounded object streams.
use vstd::prelude::*;

verus! {

pub open spec fn checked_size_admission_model(expected: u64, maximum: u64) -> Option<u64> {
    if expected <= maximum {
        Some(expected)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub(crate) fn admit_checked_size(expected: u64, maximum: u64) -> (remaining: Option<u64>)
    ensures
        remaining == checked_size_admission_model(expected, maximum),
        match remaining {
            Some(remaining) => remaining == expected && remaining <= maximum,
            None => expected > maximum,
        },
{
    if expected > maximum {
        return None;
    }
    Some(expected)
}

pub open spec fn checked_read_window_model(remaining: u64, buffer_len: u64) -> Option<u64> {
    if remaining > 0 && buffer_len > 0 {
        if remaining <= buffer_len {
            Some(remaining)
        } else {
            Some(buffer_len)
        }
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub(crate) fn choose_checked_read_window(remaining: u64, buffer_len: u64) -> (window: Option<u64>)
    ensures
        window == checked_read_window_model(remaining, buffer_len),
        match window {
            Some(window) => window > 0 && window <= remaining && window <= buffer_len
                && if remaining <= buffer_len {
                    window == remaining
                } else {
                    window == buffer_len
                },
            None => remaining == 0 || buffer_len == 0,
        },
{
    if remaining == 0 || buffer_len == 0 {
        return None;
    }
    if remaining <= buffer_len {
        Some(remaining)
    } else {
        Some(buffer_len)
    }
}

pub open spec fn checked_read_progress_model(
    remaining: u64,
    requested: u64,
    count: u64,
) -> Option<u64> {
    if remaining > 0 && requested > 0 && count > 0 && count <= requested && count <= remaining {
        Some((remaining - count) as u64)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub(crate) fn advance_checked_read(
    remaining: u64,
    requested: u64,
    count: u64,
) -> (next: Option<u64>)
    ensures
        next == checked_read_progress_model(remaining, requested, count),
        match next {
            Some(next) => requested > 0 && count > 0 && count <= requested
                && count <= remaining && next < remaining
                && next + count == remaining,
            None => remaining == 0 || requested == 0 || count == 0
                || count > requested || count > remaining,
        },
{
    if remaining == 0 || requested == 0 || count == 0 || count > requested || count > remaining {
        return None;
    }
    Some(remaining - count)
}

pub open spec fn commit_checked_read_window_model(
    remaining: u64,
    buffer_len: u64,
    requested: u64,
    count: u64,
) -> Option<u64> {
    match checked_read_window_model(remaining, buffer_len) {
        Some(window) => {
            if requested == window {
                checked_read_progress_model(remaining, window, count)
            } else {
                None
            }
        },
        None => None,
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub(crate) fn commit_checked_read_window(
    remaining: u64,
    buffer_len: u64,
    requested: u64,
    count: u64,
) -> (next: Option<u64>)
    ensures
        next == commit_checked_read_window_model(remaining, buffer_len, requested, count),
        match next {
            Some(next) => requested > 0 && requested <= remaining
                && requested <= buffer_len && count > 0 && count <= requested
                && next < remaining && next + count == remaining,
            None => true,
        },
{
    let window = choose_checked_read_window(remaining, buffer_len)?;
    if requested != window {
        return None;
    }
    advance_checked_read(remaining, requested, count)
}

pub open spec fn checked_eof_probe_model(remaining: u64, count: usize) -> Option<bool> {
    if remaining == 0 && count <= 1 {
        Some(count == 0)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub(crate) fn decide_checked_eof_probe(remaining: u64, count: usize) -> (eof: Option<bool>)
    ensures
        eof == checked_eof_probe_model(remaining, count),
        match eof {
            Some(true) => remaining == 0 && count == 0,
            Some(false) => remaining == 0 && count == 1,
            None => remaining != 0 || count > 1,
        },
{
    if remaining != 0 || count > 1 {
        return None;
    }
    Some(count == 0)
}

}

#[cfg(test)]
mod tests {
    use super::{
        admit_checked_size, advance_checked_read, choose_checked_read_window,
        commit_checked_read_window, decide_checked_eof_probe,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn checked_size_admission_preserves_exact_bounded_initial_remaining() {
        assert_eq!(admit_checked_size(0, 0), Some(0));
        assert_eq!(admit_checked_size(7, 7), Some(7));
        assert_eq!(admit_checked_size(8, 7), None);
        assert_eq!(admit_checked_size(u64::MAX, u64::MAX), Some(u64::MAX));
        assert_eq!(admit_checked_size(u64::MAX, u64::MAX - 1), None);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn checked_read_window_is_exact_min_and_bounded_by_both_inputs() {
        assert_eq!(choose_checked_read_window(0, 3), None);
        assert_eq!(choose_checked_read_window(3, 0), None);
        assert_eq!(choose_checked_read_window(4, 10), Some(4));
        assert_eq!(choose_checked_read_window(10, 4), Some(4));
        assert_eq!(choose_checked_read_window(4, 4), Some(4));
        assert_eq!(
            choose_checked_read_window(u64::MAX, u64::MAX),
            Some(u64::MAX)
        );
        assert_eq!(choose_checked_read_window(u64::MAX, 1), Some(1));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn checked_read_step_requires_the_selected_window_and_valid_progress() {
        assert_eq!(commit_checked_read_window(8, 4, 4, 3), Some(5));
        assert_eq!(commit_checked_read_window(4, 8, 4, 4), Some(0));
        assert_eq!(commit_checked_read_window(8, 4, 3, 2), None);
        assert_eq!(commit_checked_read_window(8, 4, 5, 2), None);
        assert_eq!(commit_checked_read_window(8, 4, 4, 0), None);
        assert_eq!(commit_checked_read_window(8, 4, 4, 5), None);
        assert_eq!(commit_checked_read_window(0, 4, 0, 0), None);
        assert_eq!(
            commit_checked_read_window(u64::MAX, u64::MAX, u64::MAX, u64::MAX),
            Some(0)
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn checked_read_progress_bounds_and_subtracts_only_valid_counts() {
        assert_eq!(advance_checked_read(3, 3, 3), Some(0));
        assert_eq!(advance_checked_read(7, 5, 3), Some(4));
        assert_eq!(advance_checked_read(7, 7, 1), Some(6));
        assert_eq!(advance_checked_read(0, 1, 1), None);
        assert_eq!(advance_checked_read(7, 0, 1), None);
        assert_eq!(advance_checked_read(7, 5, 0), None);
        assert_eq!(advance_checked_read(7, 2, 3), None);
        assert_eq!(advance_checked_read(2, 5, 3), None);
        assert_eq!(advance_checked_read(u64::MAX, u64::MAX, u64::MAX), Some(0));
        assert_eq!(
            advance_checked_read(u64::MAX, u64::MAX, 1),
            Some(u64::MAX - 1)
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn checked_eof_probe_requires_zero_remaining_and_a_one_byte_result() {
        assert_eq!(decide_checked_eof_probe(0, 0), Some(true));
        assert_eq!(decide_checked_eof_probe(0, 1), Some(false));
        assert_eq!(decide_checked_eof_probe(0, 2), None);
        assert_eq!(decide_checked_eof_probe(1, 0), None);
        assert_eq!(decide_checked_eof_probe(u64::MAX, usize::MAX), None);
    }
}
