//! Checked cursor transition for the native Linux regular-file writer.
use vstd::prelude::*;

verus! {

pub open spec fn write_advance_model(offset: usize, written: usize, limit: usize) -> Option<usize> {
    if offset <= limit && written > 0 && written <= limit - offset {
        Some((offset as int + written as int) as usize)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#restore-014
//= type=implementation
//# Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.
pub(crate) fn advance_write_offset(offset: usize, written: usize, limit: usize)
    -> (result: Option<usize>)
    ensures result == write_advance_model(offset, written, limit),
{
    if offset > limit || written == 0 {
        return None;
    }
    let remaining = limit - offset;
    if written > remaining {
        return None;
    }
    Some(offset + written)
}

pub open spec fn write_chunk_end_model(offset: usize, content_len: usize) -> Option<usize> {
    if offset < content_len {
        if content_len - offset <= 65_536 {
            Some(content_len)
        } else {
            Some((offset as int + 65_536) as usize)
        }
    } else {
        None
    }
}

//= docs/parsing-requirements.md#restore-014
//= type=implementation
//# Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.
pub(crate) fn next_write_chunk_end(offset: usize, content_len: usize) -> (end: Option<usize>)
    ensures
        end == write_chunk_end_model(offset, content_len),
        match end {
            Some(end) => offset < end && end <= content_len && end - offset <= 65_536,
            None => offset >= content_len,
        },
{
    if offset >= content_len {
        return None;
    }
    Some(content_len.min(offset.saturating_add(65_536)))
}

}
