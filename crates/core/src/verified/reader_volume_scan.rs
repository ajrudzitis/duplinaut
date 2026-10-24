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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeScanReservation {
    pub initial_checks: usize,
    pub planned_checks: usize,
    pub final_checks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeScanCursor {
    pub volume_count: usize,
    pub object_count: usize,
    pub planned_positions: usize,
    pub next_position: usize,
}

pub open spec fn reserve_volume_scan_model(
    checked: usize,
    volume_count: usize,
    object_count: usize,
    limit: usize,
) -> Option<VolumeScanReservation> {
    let planned = volume_count as int * object_count as int;
    let final_checks = checked as int + planned;
    if planned <= usize::MAX as int
        && final_checks <= usize::MAX as int
        && checked <= limit
        && final_checks <= limit as int
    {
        Some(VolumeScanReservation {
            initial_checks: checked,
            planned_checks: planned as usize,
            final_checks: final_checks as usize,
        })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn reserve_volume_scan(
    checked: usize,
    volume_count: usize,
    object_count: usize,
    limit: usize,
) -> (result: Option<VolumeScanReservation>)
    ensures result == reserve_volume_scan_model(checked, volume_count, object_count, limit),
{
    if checked > limit {
        return None;
    }
    let planned_checks = volume_count.checked_mul(object_count)?;
    let final_checks = checked.checked_add(planned_checks)?;
    if final_checks > limit {
        return None;
    }
    Some(VolumeScanReservation {
        initial_checks: checked,
        planned_checks,
        final_checks,
    })
}

pub open spec fn start_volume_scan_cursor_model(
    reservation: VolumeScanReservation,
    volume_count: usize,
    object_count: usize,
) -> Option<VolumeScanCursor> {
    let positions = volume_count as int * object_count as int;
    if positions <= usize::MAX as int && positions == reservation.planned_checks as int {
        Some(VolumeScanCursor {
            volume_count,
            object_count,
            planned_positions: reservation.planned_checks,
            next_position: 0,
        })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn start_volume_scan_cursor(
    reservation: VolumeScanReservation,
    volume_count: usize,
    object_count: usize,
) -> (result: Option<VolumeScanCursor>)
    ensures result == start_volume_scan_cursor_model(reservation, volume_count, object_count),
{
    let planned_positions = volume_count.checked_mul(object_count)?;
    if planned_positions != reservation.planned_checks {
        return None;
    }
    Some(VolumeScanCursor {
        volume_count,
        object_count,
        planned_positions,
        next_position: 0,
    })
}

pub open spec fn advance_volume_scan_cursor_model(
    cursor: VolumeScanCursor,
    volume_index: usize,
    object_index: usize,
) -> Option<VolumeScanCursor> {
    let expected = volume_index as int * cursor.object_count as int + object_index as int;
    if volume_index < cursor.volume_count
        && object_index < cursor.object_count
        && expected == cursor.next_position as int
        && cursor.next_position < cursor.planned_positions
    {
        Some(VolumeScanCursor {
            volume_count: cursor.volume_count,
            object_count: cursor.object_count,
            planned_positions: cursor.planned_positions,
            next_position: (cursor.next_position as int + 1) as usize,
        })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn advance_volume_scan_cursor(
    cursor: VolumeScanCursor,
    volume_index: usize,
    object_index: usize,
) -> (result: Option<VolumeScanCursor>)
    ensures result == advance_volume_scan_cursor_model(cursor, volume_index, object_index),
{
    if volume_index >= cursor.volume_count
        || object_index >= cursor.object_count
        || cursor.next_position >= cursor.planned_positions
    {
        return None;
    }
    let expected = volume_index
        .checked_mul(cursor.object_count)?
        .checked_add(object_index)?;
    if expected != cursor.next_position {
        return None;
    }
    Some(VolumeScanCursor {
        next_position: cursor.next_position.checked_add(1)?,
        ..cursor
    })
}

pub open spec fn volume_scan_cursor_complete_model(cursor: VolumeScanCursor) -> bool {
    cursor.next_position == cursor.planned_positions
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub fn volume_scan_cursor_complete(cursor: VolumeScanCursor) -> (result: bool)
    ensures result == volume_scan_cursor_complete_model(cursor),
{
    cursor.next_position == cursor.planned_positions
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
    use super::{
        advance_volume_scan_cursor, charge_volume_object, record_volume_candidate,
        reserve_volume_scan, start_volume_scan_cursor, volume_scan_cursor_complete, VolumeScan,
        VolumeScanError, VolumeScanReservation,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn volume_scan_reservation_checks_exact_cumulative_work_and_overflow() {
        assert_eq!(
            reserve_volume_scan(7, 3, 4, 19),
            Some(VolumeScanReservation {
                initial_checks: 7,
                planned_checks: 12,
                final_checks: 19,
            })
        );
        assert_eq!(reserve_volume_scan(7, 3, 4, 18), None);
        assert_eq!(reserve_volume_scan(usize::MAX, 1, 1, usize::MAX), None);
        assert_eq!(reserve_volume_scan(0, usize::MAX, 2, usize::MAX), None);
        assert_eq!(
            reserve_volume_scan(2, 0, usize::MAX, 2),
            Some(VolumeScanReservation {
                initial_checks: 2,
                planned_checks: 0,
                final_checks: 2,
            })
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn volume_scan_cursor_accepts_exact_lexicographic_positions() {
        let reservation = VolumeScanReservation {
            initial_checks: 0,
            planned_checks: 6,
            final_checks: 6,
        };
        let mut cursor = start_volume_scan_cursor(reservation, 2, 3).unwrap();
        for (volume, object) in [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)] {
            cursor = advance_volume_scan_cursor(cursor, volume, object).unwrap();
        }
        assert!(volume_scan_cursor_complete(cursor));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn volume_scan_cursor_keeps_incomplete_scan_unfinished() {
        let reservation = VolumeScanReservation {
            initial_checks: 0,
            planned_checks: 6,
            final_checks: 6,
        };
        let initial = start_volume_scan_cursor(reservation, 2, 3).unwrap();
        let incomplete = advance_volume_scan_cursor(initial, 0, 0).unwrap();
        assert!(!volume_scan_cursor_complete(initial));
        assert!(!volume_scan_cursor_complete(incomplete));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=test
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    fn volume_scan_cursor_rejects_skips_reordering_and_extra_positions() {
        let reservation = VolumeScanReservation {
            initial_checks: 0,
            planned_checks: 6,
            final_checks: 6,
        };
        let initial = start_volume_scan_cursor(reservation, 2, 3).unwrap();
        assert_eq!(advance_volume_scan_cursor(initial, 0, 1), None);
        let first = advance_volume_scan_cursor(initial, 0, 0).unwrap();
        assert_eq!(advance_volume_scan_cursor(first, 0, 0), None);
        assert_eq!(advance_volume_scan_cursor(first, 0, 2), None);
        let mut cursor = first;
        for (volume, object) in [(0, 1), (0, 2), (1, 0), (1, 1), (1, 2)] {
            cursor = advance_volume_scan_cursor(cursor, volume, object).unwrap();
        }
        assert_eq!(advance_volume_scan_cursor(cursor, 1, 2), None);
        assert_eq!(start_volume_scan_cursor(reservation, 2, 2), None);
    }

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
