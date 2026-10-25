//! Checked admission for the authentication-policy summary allocation.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicySummaryReservation {
    pub bytes: usize,
}

pub open spec fn reserve_policy_summary_model(
    signer_count: usize,
    signer_limit: usize,
    summary_bytes: usize,
    fingerprint_bytes: usize,
    metadata_limit: usize,
) -> Option<PolicySummaryReservation> {
    let fingerprint_storage = signer_count as int * fingerprint_bytes as int;
    let total = summary_bytes as int + fingerprint_storage;
    if signer_count <= signer_limit
        && fingerprint_storage <= usize::MAX as int
        && total <= usize::MAX as int
        && total <= metadata_limit as int
    {
        Some(PolicySummaryReservation { bytes: total as usize })
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
pub fn reserve_policy_summary(
    signer_count: usize,
    signer_limit: usize,
    summary_bytes: usize,
    fingerprint_bytes: usize,
    metadata_limit: usize,
) -> (result: Option<PolicySummaryReservation>)
    ensures result == reserve_policy_summary_model(
        signer_count,
        signer_limit,
        summary_bytes,
        fingerprint_bytes,
        metadata_limit,
    ),
{
    if signer_count > signer_limit {
        return None;
    }
    let fingerprint_storage = signer_count.checked_mul(fingerprint_bytes)?;
    let bytes = summary_bytes.checked_add(fingerprint_storage)?;
    if bytes > metadata_limit {
        return None;
    }
    Some(PolicySummaryReservation { bytes })
}

}

#[cfg(test)]
mod tests {
    use super::{reserve_policy_summary, PolicySummaryReservation};

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn policy_summary_reservation_checks_exact_limits_and_overflow() {
        assert_eq!(
            reserve_policy_summary(2, 2, 16, 20, 56),
            Some(PolicySummaryReservation { bytes: 56 })
        );
        assert_eq!(reserve_policy_summary(2, 2, 16, 20, 55), None);
        assert_eq!(reserve_policy_summary(3, 2, 16, 20, 100), None);
        assert_eq!(
            reserve_policy_summary(usize::MAX, usize::MAX, 0, 2, usize::MAX),
            None
        );
        assert_eq!(
            reserve_policy_summary(1, 1, usize::MAX, 1, usize::MAX),
            None
        );
    }
}
