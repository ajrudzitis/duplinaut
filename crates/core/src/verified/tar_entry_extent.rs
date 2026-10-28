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
}
