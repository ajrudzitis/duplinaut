//! Budget-before-inspection and unique-candidate transitions for volume scans.
use crate::chain::{SetDescriptor, SetKind};
#[cfg(verus_keep_ghost)]
use crate::names::Envelope;
use crate::names::{ObjectKind, ObjectName};
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

pub open spec fn volume_scan_row_major_position(
    volume_index: int,
    object_index: int,
    object_count: int,
) -> int {
    volume_index * object_count + object_index
}

pub open spec fn volume_scan_row_major_before(
    left_volume: int,
    left_object: int,
    right_volume: int,
    right_object: int,
) -> bool {
    left_volume < right_volume
        || (left_volume == right_volume && left_object < right_object)
}

pub open spec fn volume_scan_row_major_has_pair(
    position: int,
    volume_count: int,
    object_count: int,
) -> bool {
    exists |volume: int, object: int|
        0 <= volume
            && volume < volume_count
            && 0 <= object
            && object < object_count
            && volume_scan_row_major_position(volume, object, object_count) == position
}

pub open spec fn volume_scan_row_major_domain_model(
    volume_count: int,
    object_count: int,
) -> bool {
    let total = volume_count * object_count;
    0 <= volume_count
        && 0 <= object_count
        && total <= usize::MAX as int
        && (forall |volume: int, object: int|
            0 <= volume
                && volume < volume_count
                && 0 <= object
                && object < object_count
                ==> 0 <= volume_scan_row_major_position(volume, object, object_count)
                    && volume_scan_row_major_position(volume, object, object_count) < total)
        && (forall |position: int|
            0 <= position && position < total
                ==> #[trigger] volume_scan_row_major_has_pair(position, volume_count, object_count))
        && (forall |left_volume: int, left_object: int, right_volume: int, right_object: int|
            0 <= left_volume
                && left_volume < volume_count
                && 0 <= left_object
                && left_object < object_count
                && 0 <= right_volume
                && right_volume < volume_count
                && 0 <= right_object
                && right_object < object_count
                && volume_scan_row_major_position(
                    left_volume,
                    left_object,
                    object_count,
                ) == volume_scan_row_major_position(
                    right_volume,
                    right_object,
                    object_count,
                )
                ==> left_volume == right_volume && left_object == right_object)
        && (forall |left_volume: int, left_object: int, right_volume: int, right_object: int|
            0 <= left_volume
                && left_volume < volume_count
                && 0 <= left_object
                && left_object < object_count
                && 0 <= right_volume
                && right_volume < volume_count
                && 0 <= right_object
                && right_object < object_count
                && volume_scan_row_major_before(
                    left_volume,
                    left_object,
                    right_volume,
                    right_object,
                )
                ==> volume_scan_row_major_position(
                    left_volume,
                    left_object,
                    object_count,
                ) < volume_scan_row_major_position(
                    right_volume,
                    right_object,
                    object_count,
                ))
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_pair_bounds(
    volume_count: int,
    object_count: int,
    volume_index: int,
    object_index: int,
)
    requires
        0 <= volume_count,
        0 <= object_count,
        volume_count * object_count <= usize::MAX as int,
        0 <= volume_index < volume_count,
        0 <= object_index < object_count,
    ensures
        0 <= volume_scan_row_major_position(volume_index, object_index, object_count),
        volume_scan_row_major_position(volume_index, object_index, object_count)
            < volume_count * object_count,
{
    reveal(volume_scan_row_major_position);
    assert(0 < object_count);
    assert(volume_index + 1 <= volume_count);
    assert(volume_index * object_count + object_index < volume_index * object_count + object_count)
        by (nonlinear_arith)
        requires 0 <= volume_index, 0 <= object_index, object_index < object_count,
            0 < object_count;
    assert(volume_index * object_count + object_count == (volume_index + 1) * object_count)
        by (nonlinear_arith) requires 0 <= volume_index, 0 <= object_count;
    assert((volume_index + 1) * object_count <= volume_count * object_count)
        by (nonlinear_arith) requires volume_index + 1 <= volume_count, 0 <= object_count;
    assert(volume_index * object_count + object_index < volume_count * object_count)
        by (nonlinear_arith)
        requires volume_index * object_count + object_index
                < volume_index * object_count + object_count,
            volume_index * object_count + object_count == (volume_index + 1) * object_count,
            (volume_index + 1) * object_count <= volume_count * object_count;
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_position_has_pair(
    volume_count: int,
    object_count: int,
    position: int,
)
    requires
        0 <= volume_count,
        0 < object_count,
        volume_count * object_count <= usize::MAX as int,
        0 <= position,
        position < volume_count * object_count,
    ensures
        volume_scan_row_major_has_pair(position, volume_count, object_count),
        exists |volume: int, object: int|
            0 <= volume
                && volume < volume_count
                && 0 <= object
                && object < object_count
                && volume_scan_row_major_position(volume, object, object_count) == position,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(position, object_count);
    reveal(volume_scan_row_major_position);
    let volume = position / object_count;
    let object = position % object_count;
    assert(volume == position / object_count);
    assert(object == position % object_count);
    vstd::arithmetic::div_mod::lemma_div_pos_is_pos(position, object_count);
    vstd::arithmetic::div_mod::lemma_mod_division_less_than_divisor(position, object_count);
    vstd::arithmetic::div_mod::lemma_div_by_multiple_is_strongly_ordered(
        position,
        volume_count * object_count,
        volume_count,
        object_count,
    );
    assert(object_count * volume + object == position);
    assert(0 <= object && object < object_count);
    assert(0 <= volume);
    assert(volume < volume_count) by (nonlinear_arith)
        requires volume == position / object_count,
            position < volume_count * object_count,
            0 <= volume_count,
            0 < object_count;
    assert(volume_scan_row_major_has_pair(position, volume_count, object_count)) by {
        reveal(volume_scan_row_major_has_pair);
        assert(exists |candidate_volume: int, candidate_object: int|
            0 <= candidate_volume
                && candidate_volume < volume_count
                && 0 <= candidate_object
                && candidate_object < object_count
                && volume_scan_row_major_position(
                    candidate_volume,
                    candidate_object,
                    object_count,
                ) == position) by {
            assert(0 <= volume);
            assert(volume < volume_count);
            assert(0 <= object);
            assert(object < object_count);
            assert(volume_scan_row_major_position(volume, object, object_count) == position)
                by (nonlinear_arith)
                requires object_count * volume + object == position;
        }
        assert(volume_scan_row_major_position(volume, object, object_count) == position)
            by (nonlinear_arith)
            requires object_count * volume + object == position;
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_positions_are_injective(
    object_count: int,
    left_volume: int,
    left_object: int,
    right_volume: int,
    right_object: int,
)
    requires
        0 <= left_volume,
        0 <= right_volume,
        0 <= left_object,
        0 <= right_object,
        left_object < object_count,
        right_object < object_count,
        volume_scan_row_major_position(left_volume, left_object, object_count)
            == volume_scan_row_major_position(right_volume, right_object, object_count),
    ensures
        left_volume == right_volume && left_object == right_object,
{
    reveal(volume_scan_row_major_position);
    assert(0 < object_count);
    if left_volume < right_volume {
        assert(left_volume + 1 <= right_volume);
        assert((left_volume + 1) * object_count <= right_volume * object_count)
            by (nonlinear_arith)
            requires left_volume + 1 <= right_volume, 0 <= object_count;
        assert(left_volume * object_count + left_object
            < left_volume * object_count + object_count) by (nonlinear_arith)
            requires 0 <= left_object, left_object < object_count, 0 < object_count;
        assert(right_volume * object_count <= right_volume * object_count + right_object)
            by (nonlinear_arith) requires 0 <= right_object;
        assert(left_volume * object_count + object_count
            == (left_volume + 1) * object_count) by (nonlinear_arith)
            requires 0 <= left_volume, 0 <= object_count;
        assert(left_volume * object_count + left_object
            < right_volume * object_count + right_object) by (nonlinear_arith)
            requires left_volume * object_count + left_object
                    < left_volume * object_count + object_count,
                left_volume * object_count + object_count
                    == (left_volume + 1) * object_count,
                (left_volume + 1) * object_count <= right_volume * object_count,
                right_volume * object_count <= right_volume * object_count + right_object;
        assert(false);
    } else if right_volume < left_volume {
        assert(right_volume + 1 <= left_volume);
        assert((right_volume + 1) * object_count <= left_volume * object_count)
            by (nonlinear_arith)
            requires right_volume + 1 <= left_volume, 0 <= object_count;
        assert(right_volume * object_count + right_object
            < right_volume * object_count + object_count) by (nonlinear_arith)
            requires 0 <= right_object, right_object < object_count, 0 < object_count;
        assert(left_volume * object_count <= left_volume * object_count + left_object)
            by (nonlinear_arith) requires 0 <= left_object;
        assert(right_volume * object_count + object_count
            == (right_volume + 1) * object_count) by (nonlinear_arith)
            requires 0 <= right_volume, 0 <= object_count;
        assert(right_volume * object_count + right_object
            < left_volume * object_count + left_object) by (nonlinear_arith)
            requires right_volume * object_count + right_object
                    < right_volume * object_count + object_count,
                right_volume * object_count + object_count
                    == (right_volume + 1) * object_count,
                (right_volume + 1) * object_count <= left_volume * object_count,
                left_volume * object_count <= left_volume * object_count + left_object;
        assert(false);
    } else {
        assert(left_volume == right_volume);
        assert(left_object == right_object);
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_pairs_are_ordered(
    object_count: int,
    left_volume: int,
    left_object: int,
    right_volume: int,
    right_object: int,
)
    requires
        0 <= left_volume,
        0 <= right_volume,
        0 <= left_object,
        0 <= right_object,
        left_object < object_count,
        right_object < object_count,
        volume_scan_row_major_before(left_volume, left_object, right_volume, right_object),
    ensures
        volume_scan_row_major_position(left_volume, left_object, object_count)
            < volume_scan_row_major_position(right_volume, right_object, object_count),
{
    reveal(volume_scan_row_major_position);
    reveal(volume_scan_row_major_before);
    assert(0 < object_count);
    if left_volume < right_volume {
        assert(left_volume + 1 <= right_volume);
        assert((left_volume + 1) * object_count <= right_volume * object_count)
            by (nonlinear_arith)
            requires left_volume + 1 <= right_volume, 0 <= object_count;
        assert(left_volume * object_count + left_object
            < left_volume * object_count + object_count) by (nonlinear_arith)
            requires 0 <= left_object, left_object < object_count, 0 < object_count;
        assert(right_volume * object_count <= right_volume * object_count + right_object)
            by (nonlinear_arith) requires 0 <= right_object;
        assert(left_volume * object_count + object_count
            == (left_volume + 1) * object_count) by (nonlinear_arith)
            requires 0 <= left_volume, 0 <= object_count;
        assert(left_volume * object_count + left_object
            < right_volume * object_count + right_object) by (nonlinear_arith)
            requires left_volume * object_count + left_object
                    < left_volume * object_count + object_count,
                left_volume * object_count + object_count
                    == (left_volume + 1) * object_count,
                (left_volume + 1) * object_count <= right_volume * object_count,
                right_volume * object_count <= right_volume * object_count + right_object;
    } else {
        assert(left_volume == right_volume);
        assert(left_object < right_object);
        assert(left_volume * object_count + left_object
            < left_volume * object_count + right_object) by (nonlinear_arith)
            requires left_object < right_object;
        assert(volume_scan_row_major_position(left_volume, left_object, object_count)
            < volume_scan_row_major_position(right_volume, right_object, object_count))
            by (nonlinear_arith)
            requires left_volume == right_volume, left_object < right_object,
                0 <= left_volume, 0 <= object_count;
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_domain_is_exact(
    volume_count: int,
    object_count: int,
)
    requires
        0 <= volume_count,
        0 <= object_count,
        volume_count * object_count <= usize::MAX as int,
    ensures volume_scan_row_major_domain_model(volume_count, object_count),
{
    reveal(volume_scan_row_major_position);
    assert forall |volume: int, object: int|
        0 <= volume
            && volume < volume_count
            && 0 <= object
            && object < object_count
        implies 0 <= volume_scan_row_major_position(volume, object, object_count)
            && volume_scan_row_major_position(volume, object, object_count)
                < volume_count * object_count by {
        volume_scan_row_major_pair_bounds(volume_count, object_count, volume, object);
    }
    assert forall |position: int|
        0 <= position && position < volume_count * object_count
        implies #[trigger] volume_scan_row_major_has_pair(position, volume_count, object_count)
        by {
        if object_count > 0 {
            volume_scan_row_major_position_has_pair(volume_count, object_count, position);
        } else {
            assert(object_count == 0);
            assert(volume_count * object_count == 0);
            assert(position < 0);
            assert(false);
        }
    }
    reveal(volume_scan_row_major_position);
    reveal(volume_scan_row_major_before);
    assert forall |left_volume: int, left_object: int, right_volume: int, right_object: int|
        0 <= left_volume
            && left_volume < volume_count
            && 0 <= left_object
            && left_object < object_count
            && 0 <= right_volume
            && right_volume < volume_count
            && 0 <= right_object
            && right_object < object_count
            && volume_scan_row_major_position(
                left_volume,
                left_object,
                object_count,
            ) == volume_scan_row_major_position(
                right_volume,
                right_object,
                object_count,
            )
        implies left_volume == right_volume && left_object == right_object by {
        volume_scan_row_major_positions_are_injective(
            object_count,
            left_volume,
            left_object,
            right_volume,
            right_object,
        );
    }
    assert forall |left_volume: int, left_object: int, right_volume: int, right_object: int|
        0 <= left_volume
            && left_volume < volume_count
            && 0 <= left_object
            && left_object < object_count
            && 0 <= right_volume
            && right_volume < volume_count
            && 0 <= right_object
            && right_object < object_count
            && volume_scan_row_major_before(
                left_volume,
                left_object,
                right_volume,
                right_object,
            )
        implies volume_scan_row_major_position(
            left_volume,
            left_object,
            object_count as int,
        ) < volume_scan_row_major_position(
            right_volume,
            right_object,
            object_count as int,
        ) by {
        volume_scan_row_major_pairs_are_ordered(
            object_count,
            left_volume,
            left_object,
            right_volume,
            right_object,
        );
    }
}

//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
pub proof fn volume_scan_row_major_step_refines_cursor(
    volume_count: usize,
    object_count: usize,
    volume_index: usize,
    object_index: usize,
)
    requires
        volume_count as int * object_count as int <= usize::MAX as int,
        volume_index < volume_count,
        object_index < object_count,
    ensures
        advance_volume_scan_cursor_model(
            VolumeScanCursor {
                volume_count,
                object_count,
                planned_positions: (volume_count as int * object_count as int) as usize,
                next_position: volume_scan_row_major_position(
                    volume_index as int,
                    object_index as int,
                    object_count as int,
                ) as usize,
            },
            volume_index,
            object_index,
        ) == Some(VolumeScanCursor {
            volume_count,
            object_count,
            planned_positions: (volume_count as int * object_count as int) as usize,
            next_position: (volume_scan_row_major_position(
                volume_index as int,
                object_index as int,
                object_count as int,
            ) + 1) as usize,
        }),
{
    volume_scan_row_major_pair_bounds(
        volume_count as int,
        object_count as int,
        volume_index as int,
        object_index as int,
    );
    reveal(advance_volume_scan_cursor_model);
    assert(volume_index as int * object_count as int + object_index as int
        == volume_scan_row_major_position(
            volume_index as int,
            object_index as int,
            object_count as int,
        ));
}

/// Fieldwise model for one already-parsed catalog name and one required
/// manifest volume. Name parsing, manifest parsing, source identity, and later
/// envelope handling are outside this predicate.
pub open spec fn required_volume_identity_model(
    name: ObjectName,
    descriptor: SetDescriptor,
    required_volume: u64,
) -> bool {
    !name.partial
        && name.envelope != Envelope::Pkcs7
        && name.start.0 == descriptor.start_value().0
        && name.end.0 == descriptor.end_value().0
        && name.volume == Some(required_volume)
        && match (descriptor.kind_value(), name.kind) {
            (SetKind::Full, ObjectKind::FullVolume)
            | (SetKind::Incremental, ObjectKind::IncrementalVolume) => true,
            _ => false,
        }
}

//= docs/parsing-requirements.md#parse-chain-002
//= type=implementation
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
//= docs/parsing-requirements.md#parse-name-005
//= type=implementation
//# Objects marked partial MUST be excluded from completed restore sets.
/// Match an already-parsed object name to the exact required manifest volume.
/// The name's envelope is not otherwise part of identity; `completed()` only
/// excludes partial and PKCS#7 names. Authentication and decoding occur later.
pub fn required_volume_identity_matches(
    name: ObjectName,
    descriptor: SetDescriptor,
    required_volume: u64,
) -> (result: bool)
    ensures result == required_volume_identity_model(name, descriptor, required_volume),
{
    let completed = name.completed();
    let start_matches = name.start.0 == descriptor.start().0;
    let end_matches = name.end.0 == descriptor.end().0;
    let ordinal_matches = name.volume == Some(required_volume);
    let kind_matches = matches!(
        (descriptor.kind(), name.kind),
        (SetKind::Full, ObjectKind::FullVolume)
            | (SetKind::Incremental, ObjectKind::IncrementalVolume)
    );
    let result = completed && start_matches && end_matches && ordinal_matches && kind_matches;
    proof {
        reveal(required_volume_identity_model);
        assert(completed == (!name.partial && name.envelope != Envelope::Pkcs7));
        assert(start_matches == (name.start.0 == descriptor.start_value().0));
        assert(end_matches == (name.end.0 == descriptor.end_value().0));
        assert(kind_matches == match (descriptor.kind_value(), name.kind) {
            (SetKind::Full, ObjectKind::FullVolume)
            | (SetKind::Incremental, ObjectKind::IncrementalVolume) => true,
            _ => false,
        });
    }
    result
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
        required_volume_identity_matches, reserve_volume_scan, start_volume_scan_cursor,
        volume_scan_cursor_complete, VolumeScan, VolumeScanError, VolumeScanReservation,
    };
    use crate::chain::{SetDescriptor, SetKind};
    use crate::names::{Envelope, ObjectKind, ObjectName, Timestamp};

    fn volume_name(
        kind: ObjectKind,
        start: i128,
        end: i128,
        volume: u64,
        envelope: Envelope,
        partial: bool,
    ) -> ObjectName {
        ObjectName {
            kind,
            start: Timestamp(start),
            end: Timestamp(end),
            volume: Some(volume),
            envelope,
            partial,
            short: false,
        }
    }

    fn descriptor(kind: SetKind, start: i128, end: i128) -> SetDescriptor {
        let name = ObjectName {
            kind: match kind {
                SetKind::Full => ObjectKind::FullManifest,
                SetKind::Incremental => ObjectKind::IncrementalManifest,
            },
            start: Timestamp(start),
            end: Timestamp(end),
            volume: None,
            envelope: Envelope::Plain,
            partial: false,
            short: false,
        };
        SetDescriptor::from_manifest_name(name).unwrap()
    }

    #[test]
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=test
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    //= docs/parsing-requirements.md#parse-name-005
    //= type=test
    //# Objects marked partial MUST be excluded from completed restore sets.
    fn required_volume_identity_matches_exact_project_fields() {
        let full = descriptor(SetKind::Full, 100, 100);
        let full_name = volume_name(ObjectKind::FullVolume, 100, 100, 3, Envelope::Plain, false);
        assert!(required_volume_identity_matches(full_name, full, 3));

        // Each supported envelope retains the same post-parse identity rule.
        for envelope in [Envelope::Plain, Envelope::Gzip, Envelope::OpenPgp] {
            assert!(required_volume_identity_matches(
                volume_name(ObjectKind::FullVolume, 100, 100, 3, envelope, false),
                full,
                3
            ));
        }

        // Isolated full-set field mismatches: kind, start, end, and ordinal.
        assert!(!required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                100,
                100,
                3,
                Envelope::Plain,
                false
            ),
            full,
            3
        ));
        assert!(!required_volume_identity_matches(
            volume_name(ObjectKind::FullVolume, 99, 100, 3, Envelope::Plain, false),
            full,
            3
        ));
        assert!(!required_volume_identity_matches(
            volume_name(ObjectKind::FullVolume, 100, 101, 3, Envelope::Plain, false),
            full,
            3
        ));
        assert!(!required_volume_identity_matches(
            volume_name(ObjectKind::FullVolume, 100, 100, 4, Envelope::Plain, false),
            full,
            3
        ));

        let incremental = descriptor(SetKind::Incremental, 100, 120);
        assert!(required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                100,
                120,
                7,
                Envelope::Plain,
                false
            ),
            incremental,
            7
        ));
        for envelope in [Envelope::Plain, Envelope::Gzip, Envelope::OpenPgp] {
            assert!(required_volume_identity_matches(
                volume_name(ObjectKind::IncrementalVolume, 100, 120, 7, envelope, false),
                incremental,
                7
            ));
        }

        // Isolated incremental-set field mismatches: kind, start, end, ordinal.
        assert!(!required_volume_identity_matches(
            volume_name(ObjectKind::FullVolume, 100, 120, 7, Envelope::Plain, false),
            incremental,
            7
        ));
        assert!(!required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                99,
                120,
                7,
                Envelope::Plain,
                false
            ),
            incremental,
            7
        ));
        assert!(!required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                100,
                121,
                7,
                Envelope::Plain,
                false
            ),
            incremental,
            7
        ));
        assert!(!required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                100,
                120,
                8,
                Envelope::Plain,
                false
            ),
            incremental,
            7
        ));

        assert!(!required_volume_identity_matches(
            volume_name(ObjectKind::FullVolume, 100, 100, 3, Envelope::Plain, true),
            full,
            3
        ));
        assert!(!required_volume_identity_matches(
            volume_name(
                ObjectKind::IncrementalVolume,
                100,
                120,
                7,
                Envelope::Gzip,
                true
            ),
            incremental,
            7
        ));
    }

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
    fn volume_scan_cursor_matches_independent_row_major_dimension_matrix() {
        type RowMajorCase = (usize, usize, &'static [(usize, usize)]);
        let cases: &[RowMajorCase] = &[
            (0, 0, &[]),
            (0, 3, &[]),
            (2, 0, &[]),
            (1, 1, &[(0, 0)]),
            (1, 3, &[(0, 0), (0, 1), (0, 2)]),
            (3, 1, &[(0, 0), (1, 0), (2, 0)]),
            (2, 3, &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)]),
            (3, 2, &[(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1)]),
        ];

        for (volume_count, object_count, expected_pairs) in cases {
            let planned = volume_count.checked_mul(*object_count).unwrap();
            let reservation = VolumeScanReservation {
                initial_checks: 0,
                planned_checks: planned,
                final_checks: planned,
            };
            let mut cursor =
                start_volume_scan_cursor(reservation, *volume_count, *object_count).unwrap();
            assert_eq!(cursor.next_position, 0);
            assert_eq!(
                volume_scan_cursor_complete(cursor),
                expected_pairs.is_empty()
            );

            for (position, &(volume, object)) in expected_pairs.iter().enumerate() {
                assert_eq!(cursor.next_position, position);
                cursor = advance_volume_scan_cursor(cursor, volume, object)
                    .expect("the independent row-major pair is accepted at its exact position");
                assert_eq!(cursor.next_position, position + 1);
                assert_eq!(volume_scan_cursor_complete(cursor), position + 1 == planned);
            }
            assert!(volume_scan_cursor_complete(cursor));

            if expected_pairs.is_empty() {
                assert_eq!(advance_volume_scan_cursor(cursor, 0, 0), None);
                assert_eq!(advance_volume_scan_cursor(cursor, *volume_count, 0), None);
            }
        }

        let mismatched_reservation = VolumeScanReservation {
            initial_checks: 0,
            planned_checks: 5,
            final_checks: 5,
        };
        assert_eq!(start_volume_scan_cursor(mismatched_reservation, 2, 3), None);

        let overflow_reservation = VolumeScanReservation {
            initial_checks: 0,
            planned_checks: usize::MAX,
            final_checks: usize::MAX,
        };
        assert_eq!(
            start_volume_scan_cursor(overflow_reservation, usize::MAX, 2),
            None
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
