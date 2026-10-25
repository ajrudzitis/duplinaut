//! Exact remaining-byte transition for bounded object streams.
use vstd::prelude::*;

verus! {

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

}

#[cfg(test)]
mod tests {
    use super::advance_checked_read;

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
}
