//! Checked work accounting for authentication-evidence certificate comparisons.
use vstd::prelude::*;

verus! {

pub open spec fn charge_evidence_comparisons_model(
    current: usize,
    pool_len: usize,
    limit: usize,
) -> Option<usize> {
    let next = current as int + pool_len as int;
    if current <= limit
        && next <= usize::MAX as int
        && next <= limit as int
    {
        Some(next as usize)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=implementation
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
pub fn charge_evidence_comparisons(
    current: usize,
    pool_len: usize,
    limit: usize,
) -> (result: Option<usize>)
    ensures result == charge_evidence_comparisons_model(current, pool_len, limit),
{
    if current > limit {
        return None;
    }
    let next = current.checked_add(pool_len)?;
    if next > limit {
        return None;
    }
    Some(next)
}

}

#[cfg(test)]
mod tests {
    use super::charge_evidence_comparisons;

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn evidence_comparison_charge_checks_exact_limit_and_overflow() {
        assert_eq!(charge_evidence_comparisons(5, 3, 8), Some(8));
        assert_eq!(charge_evidence_comparisons(5, 3, 7), None);
        assert_eq!(charge_evidence_comparisons(usize::MAX, 1, usize::MAX), None);
        assert_eq!(
            charge_evidence_comparisons(0, usize::MAX, usize::MAX),
            Some(usize::MAX)
        );
    }
}
