//! Checked admission for one authentication-evidence object record.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceMetadataReservation {
    pub bytes: usize,
}

pub open spec fn reserve_evidence_record_model(
    current_objects: usize,
    object_limit: usize,
    current_bytes: usize,
    source_metadata_bytes: usize,
    version_count: usize,
    version_index_bytes: usize,
    byte_limit: usize,
) -> Option<EvidenceMetadataReservation> {
    let index_bytes = version_count as int * version_index_bytes as int;
    let record_bytes = source_metadata_bytes as int + index_bytes;
    let total_bytes = current_bytes as int + record_bytes;
    if current_objects < object_limit
        && index_bytes <= usize::MAX as int
        && record_bytes <= usize::MAX as int
        && total_bytes <= usize::MAX as int
        && current_bytes <= byte_limit
        && total_bytes <= byte_limit as int
    {
        Some(EvidenceMetadataReservation { bytes: total_bytes as usize })
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
pub fn reserve_evidence_record(
    current_objects: usize,
    object_limit: usize,
    current_bytes: usize,
    source_metadata_bytes: usize,
    version_count: usize,
    version_index_bytes: usize,
    byte_limit: usize,
) -> (result: Option<EvidenceMetadataReservation>)
    ensures result == reserve_evidence_record_model(
        current_objects,
        object_limit,
        current_bytes,
        source_metadata_bytes,
        version_count,
        version_index_bytes,
        byte_limit,
    ),
{
    if current_objects >= object_limit || current_bytes > byte_limit {
        return None;
    }
    let index_bytes = version_count.checked_mul(version_index_bytes)?;
    let record_bytes = source_metadata_bytes.checked_add(index_bytes)?;
    let bytes = current_bytes.checked_add(record_bytes)?;
    if bytes > byte_limit {
        return None;
    }
    Some(EvidenceMetadataReservation { bytes })
}

}

#[cfg(test)]
mod tests {
    use super::{reserve_evidence_record, EvidenceMetadataReservation};

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn evidence_record_reservation_checks_exact_limits_and_overflow() {
        assert_eq!(
            reserve_evidence_record(2, 3, 10, 4, 2, 4, 22),
            Some(EvidenceMetadataReservation { bytes: 22 })
        );
        assert_eq!(reserve_evidence_record(2, 3, 10, 4, 2, 4, 21), None);
        assert_eq!(reserve_evidence_record(3, 3, 0, 0, 0, 4, usize::MAX), None);
        assert_eq!(
            reserve_evidence_record(0, 1, 0, 0, usize::MAX, 2, usize::MAX),
            None
        );
        assert_eq!(
            reserve_evidence_record(0, 1, usize::MAX, 1, 0, 0, usize::MAX),
            None
        );
    }
}
