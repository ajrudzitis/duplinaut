//! Checked admission for one authentication-evidence object record.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceMetadataReservation {
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CertificateVersionAdmission {
    pub version_count: usize,
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

pub open spec fn admit_certificate_version_model(
    current_versions: usize,
    version_limit: usize,
    current_bytes: usize,
    certificate_metadata_bytes: usize,
    byte_limit: usize,
) -> Option<CertificateVersionAdmission> {
    let next_versions = current_versions as int + 1;
    let next_bytes = current_bytes as int + certificate_metadata_bytes as int;
    if current_versions < version_limit
        && current_bytes <= byte_limit
        && next_versions <= usize::MAX as int
        && next_bytes <= usize::MAX as int
        && next_bytes <= byte_limit as int
    {
        Some(CertificateVersionAdmission {
            version_count: next_versions as usize,
            bytes: next_bytes as usize,
        })
    } else {
        None
    }
}

pub open spec fn certificate_metadata_charge_model(
    base_metadata_bytes: usize,
    signature_count: usize,
    signature_metadata_bytes: usize,
) -> Option<usize> {
    let signature_bytes = signature_count as int * signature_metadata_bytes as int;
    let total = base_metadata_bytes as int + signature_bytes;
    if signature_bytes <= usize::MAX as int && total <= usize::MAX as int {
        Some(total as usize)
    } else {
        None
    }
}

pub open spec fn source_metadata_charge_model(
    key_capacity: usize,
    etag_capacity: usize,
    version_id_capacity: usize,
    object_metadata_bytes: usize,
) -> Option<usize> {
    let total = key_capacity as int
        + etag_capacity as int
        + version_id_capacity as int
        + object_metadata_bytes as int;
    if total <= usize::MAX as int {
        Some(total as usize)
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

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=implementation
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
pub fn admit_certificate_version(
    current_versions: usize,
    version_limit: usize,
    current_bytes: usize,
    certificate_metadata_bytes: usize,
    byte_limit: usize,
) -> (result: Option<CertificateVersionAdmission>)
    ensures result == admit_certificate_version_model(
        current_versions,
        version_limit,
        current_bytes,
        certificate_metadata_bytes,
        byte_limit,
    ),
{
    if current_versions >= version_limit || current_bytes > byte_limit {
        return None;
    }
    let version_count = current_versions.checked_add(1)?;
    let bytes = current_bytes.checked_add(certificate_metadata_bytes)?;
    if bytes > byte_limit {
        return None;
    }
    Some(CertificateVersionAdmission {
        version_count,
        bytes,
    })
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=implementation
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
pub fn certificate_metadata_charge(
    base_metadata_bytes: usize,
    signature_count: usize,
    signature_metadata_bytes: usize,
) -> (result: Option<usize>)
    ensures result == certificate_metadata_charge_model(
        base_metadata_bytes,
        signature_count,
        signature_metadata_bytes,
    ),
{
    let signature_bytes = signature_count.checked_mul(signature_metadata_bytes)?;
    let total = base_metadata_bytes.checked_add(signature_bytes)?;
    Some(total)
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
//= docs/parsing-requirements.md#parse-env-022
//= type=implementation
//# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
pub fn source_metadata_charge(
    key_capacity: usize,
    etag_capacity: usize,
    version_id_capacity: usize,
    object_metadata_bytes: usize,
) -> (result: Option<usize>)
    ensures result == source_metadata_charge_model(
        key_capacity,
        etag_capacity,
        version_id_capacity,
        object_metadata_bytes,
    ),
{
    let with_etag = key_capacity.checked_add(etag_capacity)?;
    let with_version = with_etag.checked_add(version_id_capacity)?;
    let total = with_version.checked_add(object_metadata_bytes)?;
    Some(total)
}

}

#[cfg(test)]
mod tests {
    use super::{
        admit_certificate_version, certificate_metadata_charge, reserve_evidence_record,
        source_metadata_charge, CertificateVersionAdmission, EvidenceMetadataReservation,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn certificate_version_admission_checks_exact_limits_and_overflow() {
        assert_eq!(
            admit_certificate_version(2, 3, 10, 4, 14),
            Some(CertificateVersionAdmission {
                version_count: 3,
                bytes: 14,
            })
        );
        assert_eq!(admit_certificate_version(2, 3, 10, 4, 13), None);
        assert_eq!(admit_certificate_version(3, 3, 10, 0, usize::MAX), None);
        assert_eq!(
            admit_certificate_version(0, 1, usize::MAX, 1, usize::MAX),
            None
        );
        assert_eq!(
            admit_certificate_version(usize::MAX, usize::MAX, 0, 0, usize::MAX),
            None
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn certificate_metadata_charge_checks_exact_formula_and_overflow() {
        assert_eq!(certificate_metadata_charge(16, 2, 4), Some(24));
        assert_eq!(certificate_metadata_charge(0, usize::MAX, 2), None);
        assert_eq!(
            certificate_metadata_charge(usize::MAX, 0, 1),
            Some(usize::MAX)
        );
        assert_eq!(certificate_metadata_charge(usize::MAX, 1, 1), None);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=test
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    fn source_metadata_charge_checks_exact_formula_and_overflow() {
        assert_eq!(source_metadata_charge(2, 3, 4, 5), Some(14));
        assert_eq!(source_metadata_charge(usize::MAX, 1, 0, 0), None);
        assert_eq!(source_metadata_charge(1, 2, usize::MAX, 0), None);
        assert_eq!(source_metadata_charge(usize::MAX - 1, 1, 0, 1), None);
        assert_eq!(
            source_metadata_charge(0, 0, 0, usize::MAX),
            Some(usize::MAX)
        );
    }

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
