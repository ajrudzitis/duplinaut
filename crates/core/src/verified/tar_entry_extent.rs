//! Checked payload slicing and next-cursor selection for one tar entry.
#[cfg(verus_keep_ghost)]
use super::tar_helpers::extent_model;
use super::tar_helpers::{extent, ExtentError};
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryExtentError {
    Limit,
    Truncated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedEntryError {
    Numeric,
    Limit,
    Truncated,
}

pub type SelectedEntryExtent<'a> = (usize, &'a [u8], usize);
pub type SelectedEntryExtentResult<'a> = Result<SelectedEntryExtent<'a>, SelectedEntryError>;

pub open spec fn entry_extent_model(cursor: int, size: int, total: int)
    -> Result<(int, int, int), EntryExtentError>
{
    match extent_model(cursor, size, total) {
        Ok((data_start, data_end, next)) => Ok((data_start, data_end, next)),
        Err(ExtentError::Overflow) => Err(EntryExtentError::Limit),
        Err(ExtentError::Truncated) => Err(EntryExtentError::Truncated),
    }
}

/// Return the entry payload slice and cursor after its padded extent.
pub fn take_entry_payload(input: &[u8], cursor: usize, size: usize)
    -> (result: Result<(&[u8], usize), EntryExtentError>)
    requires cursor <= input.len(),
    ensures match result {
        Ok((payload, next)) => match extent_model(cursor as int, size as int, input.len() as int) {
            Ok((data_start, data_end, expected_next)) =>
                payload@ == input@.subrange(data_start, data_end)
                    && payload.len() as int == size as int
                    && next as int == expected_next
                    && (cursor as int) < data_start
                    && data_start <= data_end
                    && data_end <= next as int
                    && next as int <= input.len() as int
                    && (next as int - data_end) < 512
                    && ((cursor as int) % 512 == 0 ==> (next as int) % 512 == 0),
            Err(_) => false,
        },
        Err(error) => entry_extent_model(cursor as int, size as int, input.len() as int)
            == Err(error),
    },
{
    match extent(cursor, size, input.len()) {
        Ok((data_start, data_end, next)) => Ok((&input[data_start..data_end], next)),
        Err(ExtentError::Overflow) => Err(EntryExtentError::Limit),
        Err(ExtentError::Truncated) => Err(EntryExtentError::Truncated),
    }
}

//= docs/parsing-requirements.md#parse-tar-008
//# After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary.
/// Select a PAX/raw header size, enforce its entry limit, then take its exact extent.
//= docs/parsing-requirements.md#parse-tar-008
//# It MUST reject configured entry-size excess and truncated extents with explicit errors.
pub fn take_selected_entry_payload(
    input: &[u8],
    cursor: usize,
    selected_size: Result<u64, super::tar_pax::PaxSizeError>,
    max_entry_bytes: usize,
) -> (result: Result<(usize, &[u8], usize), SelectedEntryError>)
    requires cursor <= input.len(),
    ensures match selected_size {
        Err(_) => result == Err(SelectedEntryError::Numeric),
        Ok(size) => if size > usize::MAX as u64 {
            result == Err(SelectedEntryError::Limit)
        } else if size as usize > max_entry_bytes {
            result == Err(SelectedEntryError::Limit)
        } else {
            match result {
                Ok((selected, payload, next)) => {
                    selected == size as usize
                        && match extent_model(cursor as int, size as int, input.len() as int) {
                            Ok((data_start, data_end, expected_next)) =>
                                payload@ == input@.subrange(data_start, data_end)
                                    && payload.len() as int == size as int
                                    && next as int == expected_next
                                    && (cursor as int) < data_start
                                    && data_start <= data_end
                                    && data_end <= next as int
                                    && next as int <= input.len() as int
                                    && (next as int - data_end) < 512
                                    && ((cursor as int) % 512 == 0 ==> (next as int) % 512 == 0),
                            Err(_) => false,
                        }
                },
                Err(SelectedEntryError::Limit) =>
                    entry_extent_model(cursor as int, size as int, input.len() as int)
                        == Err(EntryExtentError::Limit),
                Err(SelectedEntryError::Truncated) =>
                    entry_extent_model(cursor as int, size as int, input.len() as int)
                        == Err(EntryExtentError::Truncated),
                Err(SelectedEntryError::Numeric) => false,
            }
        },
    },
{
    let size = match selected_size {
        Ok(size) => size,
        Err(_) => return Err(SelectedEntryError::Numeric),
    };
    let size = match usize::try_from(size) {
        Ok(size) => size,
        Err(_) => return Err(SelectedEntryError::Limit),
    };
    if size > max_entry_bytes {
        return Err(SelectedEntryError::Limit);
    }
    match take_entry_payload(input, cursor, size) {
        Ok((payload, next)) => Ok((size, payload, next)),
        Err(EntryExtentError::Limit) => Err(SelectedEntryError::Limit),
        Err(EntryExtentError::Truncated) => Err(SelectedEntryError::Truncated),
    }
}
}
