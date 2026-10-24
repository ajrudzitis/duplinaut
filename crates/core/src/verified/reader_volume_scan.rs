//! Budget-before-inspection and unique-candidate transitions for volume scans.
use crate::verified::reader_accounting::charge_object_check;
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeScanError {
    Limit,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeScan {
    pub checked_objects: usize,
    pub candidate_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChargedVolumeObject {
    pub scan: VolumeScan,
    pub object_index: usize,
}

pub open spec fn charge_volume_object_model(
    scan: VolumeScan,
    limit: usize,
    object_index: usize,
    object_count: usize,
) -> Result<ChargedVolumeObject, VolumeScanError> {
    if object_index >= object_count {
        Err(VolumeScanError::Limit)
    } else if scan.checked_objects >= limit {
        Err(VolumeScanError::Limit)
    } else {
        Ok(ChargedVolumeObject {
            scan: VolumeScan {
                checked_objects: (scan.checked_objects as int + 1) as usize,
                candidate_index: scan.candidate_index,
            },
            object_index,
        })
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn charge_volume_object(
    scan: VolumeScan,
    limit: usize,
    object_index: usize,
    object_count: usize,
) -> (result: Result<ChargedVolumeObject, VolumeScanError>)
    ensures result == charge_volume_object_model(scan, limit, object_index, object_count),
{
    if object_index >= object_count || scan.checked_objects >= limit {
        return Err(VolumeScanError::Limit);
    }
    let checked_objects = match charge_object_check(scan.checked_objects, limit) {
        Some(next) => next,
        None => return Err(VolumeScanError::Limit),
    };
    Ok(ChargedVolumeObject {
        scan: VolumeScan {
            checked_objects,
            candidate_index: scan.candidate_index,
        },
        object_index,
    })
}

pub open spec fn record_volume_candidate_model(
    charged: ChargedVolumeObject,
    matches: bool,
) -> Result<VolumeScan, VolumeScanError> {
    if matches && charged.scan.candidate_index.is_some() {
        Err(VolumeScanError::Duplicate)
    } else {
        Ok(VolumeScan {
            checked_objects: charged.scan.checked_objects,
            candidate_index: if matches {
                Some(charged.object_index)
            } else {
                charged.scan.candidate_index
            },
        })
    }
}

//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
pub fn record_volume_candidate(
    charged: ChargedVolumeObject,
    matches: bool,
) -> (result: Result<VolumeScan, VolumeScanError>)
    ensures result == record_volume_candidate_model(charged, matches),
{
    if matches && charged.scan.candidate_index.is_some() {
        return Err(VolumeScanError::Duplicate);
    }
    Ok(VolumeScan {
        checked_objects: charged.scan.checked_objects,
        candidate_index: if matches {
            Some(charged.object_index)
        } else {
            charged.scan.candidate_index
        },
    })
}

}

#[cfg(test)]
mod tests {
    use super::{charge_volume_object, record_volume_candidate, VolumeScan, VolumeScanError};

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=test
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    fn volume_scan_charges_before_matching_and_rejects_a_second_candidate() {
        let initial = VolumeScan {
            checked_objects: 0,
            candidate_index: None,
        };
        let first = charge_volume_object(initial, 3, 0, 3).unwrap();
        let scan = record_volume_candidate(first, false).unwrap();
        let second = charge_volume_object(scan, 3, 1, 3).unwrap();
        let scan = record_volume_candidate(second, true).unwrap();
        assert_eq!(scan.checked_objects, 2);
        assert_eq!(scan.candidate_index, Some(1));

        let third = charge_volume_object(scan, 3, 2, 3).unwrap();
        assert_eq!(
            record_volume_candidate(third, true),
            Err(VolumeScanError::Duplicate)
        );
        assert_eq!(
            charge_volume_object(scan, 2, 2, 3),
            Err(VolumeScanError::Limit)
        );
    }
}
