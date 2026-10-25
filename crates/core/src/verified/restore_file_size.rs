//! Exact regular-file size comparison used by native restore verification.
use vstd::prelude::*;

verus! {

pub open spec fn file_size_matches_model(expected: u64, observed: Option<u64>) -> bool {
    match observed {
        Some(size) => size == expected,
        None => false,
    }
}

//= docs/parsing-requirements.md#restore-017
//= type=implementation
//# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
pub(crate) fn file_size_matches(expected: u64, observed: Option<u64>) -> (matches: bool)
    ensures matches == file_size_matches_model(expected, observed),
{
    match observed {
        Some(size) => size == expected,
        None => false,
    }
}

}

#[cfg(test)]
mod tests {
    use super::file_size_matches;

    #[test]
    //= docs/parsing-requirements.md#restore-017
    //= type=test
    //# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
    fn file_size_comparison_requires_exact_nonnegative_observation() {
        assert!(file_size_matches(0, Some(0)));
        assert!(file_size_matches(u64::MAX, Some(u64::MAX)));
        assert!(!file_size_matches(8, Some(7)));
        assert!(!file_size_matches(8, Some(9)));
        assert!(!file_size_matches(0, None));

        let negative_stat_size = u64::try_from(-1_i64).ok();
        assert_eq!(negative_stat_size, None);
        let overflowing_expected_size = u64::try_from(u128::MAX).ok();
        assert_eq!(overflowing_expected_size, None);
    }
}
