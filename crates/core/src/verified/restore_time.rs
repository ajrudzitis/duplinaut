//! Validated, lossless conversion boundary for Linux restore modification times.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestoreTime {
    pub seconds: i64,
    pub nanoseconds: u32,
}

pub open spec fn restore_time_model(seconds: i64, nanoseconds: u32) -> Option<RestoreTime> {
    if nanoseconds < 1_000_000_000 {
        Some(RestoreTime { seconds, nanoseconds })
    } else {
        None
    }
}

//= docs/parsing-requirements.md#restore-011
//= type=implementation
//# The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.
//= docs/parsing-requirements.md#restore-017
//= type=implementation
//# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
pub(crate) fn validate_restore_time(seconds: i64, nanoseconds: u32)
    -> (result: Option<RestoreTime>)
    ensures result == restore_time_model(seconds, nanoseconds),
{
    if nanoseconds >= 1_000_000_000 {
        return None;
    }
    Some(RestoreTime { seconds, nanoseconds })
}

}
