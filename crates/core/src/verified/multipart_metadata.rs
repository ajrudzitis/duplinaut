//! Checked preflight for multipart assembler discovery metadata.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultipartMetadataReservation {
    pub bytes: usize,
}

pub open spec fn reserve_multipart_metadata_model(
    raw_record_count: usize,
    volume_reference_bytes: usize,
    raw_occurrence_bytes: usize,
    assembler_shell_bytes: usize,
    limit: usize,
) -> Option<MultipartMetadataReservation> {
    let slots = if raw_record_count < 4 { 4int } else { raw_record_count as int };
    let reference_count = slots * 2;
    let reference_bytes = reference_count * volume_reference_bytes as int;
    let occurrence_bytes = slots * raw_occurrence_bytes as int;
    let total = reference_bytes + occurrence_bytes + assembler_shell_bytes as int;
    if reference_count <= usize::MAX as int
        && reference_bytes <= usize::MAX as int
        && occurrence_bytes <= usize::MAX as int
        && total <= usize::MAX as int
        && total <= limit as int
    {
        Some(MultipartMetadataReservation { bytes: total as usize })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn reserve_multipart_metadata(
    raw_record_count: usize,
    volume_reference_bytes: usize,
    raw_occurrence_bytes: usize,
    assembler_shell_bytes: usize,
    limit: usize,
) -> (result: Option<MultipartMetadataReservation>)
    ensures result == reserve_multipart_metadata_model(
        raw_record_count,
        volume_reference_bytes,
        raw_occurrence_bytes,
        assembler_shell_bytes,
        limit,
    ),
{
    let slots = raw_record_count.max(4);
    let reference_count = slots.checked_mul(2)?;
    let reference_bytes = reference_count.checked_mul(volume_reference_bytes)?;
    let occurrence_bytes = slots.checked_mul(raw_occurrence_bytes)?;
    let bytes = reference_bytes
        .checked_add(occurrence_bytes)?
        .checked_add(assembler_shell_bytes)?;
    if bytes > limit {
        return None;
    }
    Some(MultipartMetadataReservation { bytes })
}

}

#[cfg(test)]
mod tests {
    use super::{reserve_multipart_metadata, MultipartMetadataReservation};

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn multipart_metadata_reservation_checks_exact_short_and_overflow() {
        let exact = reserve_multipart_metadata(2, 8, 1, 9, 77);
        assert_eq!(exact, Some(MultipartMetadataReservation { bytes: 77 }));
        assert_eq!(reserve_multipart_metadata(2, 8, 1, 9, 76), None);
        assert_eq!(
            reserve_multipart_metadata(usize::MAX, 1, 1, 0, usize::MAX),
            None
        );
        assert_eq!(
            reserve_multipart_metadata(0, 0, 1, usize::MAX, usize::MAX),
            None
        );
    }
}
