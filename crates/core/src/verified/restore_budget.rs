//! Checked multidimensional resource preflight for Linux restoration.
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestorePreflightNeed {
    pub entry_count: usize,
    pub root_exists: bool,
    pub directory_count: usize,
    pub active_file: bool,
    pub wrapper: bool,
    pub payload_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestorePreflightLimits {
    pub entry_count: usize,
    pub directory_fds: usize,
    pub cleanup_work: usize,
    pub payload_bytes: u64,
    pub staging_attempts: usize,
}

pub open spec fn restore_preflight_model(
    need: RestorePreflightNeed,
    limits: RestorePreflightLimits,
) -> Option<usize> {
    let root_entry = if need.root_exists { 1int } else { 0int };
    let root_directory = if need.root_exists { 0int } else { 1int };
    let wrapper_overhead = if need.wrapper { 2int } else { 1int };
    let active_file_fd = if need.active_file { 1int } else { 0int };
    let created = need.entry_count as int - root_entry + wrapper_overhead;
    let directory_fds = need.directory_count as int + root_directory + wrapper_overhead + active_file_fd;
    let cleanup_work = created * 2;
    if (!need.root_exists || need.entry_count > 0)
        && created >= 0 && created <= usize::MAX as int
        && directory_fds <= usize::MAX as int
        && cleanup_work <= usize::MAX as int
        && created <= limits.entry_count as int
        && directory_fds <= limits.directory_fds as int
        && cleanup_work <= limits.cleanup_work as int
        && need.payload_bytes <= limits.payload_bytes
        && limits.staging_attempts > 0
    {
        Some(cleanup_work as usize)
    } else {
        None
    }
}

//= docs/parsing-requirements.md#restore-012
//= type=implementation
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
pub(crate) fn restore_preflight(
    need: RestorePreflightNeed,
    limits: RestorePreflightLimits,
) -> (result: Option<usize>)
    ensures result == restore_preflight_model(need, limits),
{
    let root_entry = if need.root_exists { 1 } else { 0 };
    let root_directory = if need.root_exists { 0 } else { 1 };
    let wrapper_overhead = if need.wrapper { 2 } else { 1 };
    let active_file_fd = if need.active_file { 1 } else { 0 };
    let created = need.entry_count.checked_sub(root_entry)?;
    let created = created.checked_add(wrapper_overhead)?;
    let directory_fds = need.directory_count.checked_add(root_directory)?;
    let directory_fds = directory_fds.checked_add(wrapper_overhead)?;
    let directory_fds = directory_fds.checked_add(active_file_fd)?;
    let cleanup_work = created.checked_mul(2)?;
    if (need.root_exists && need.entry_count == 0)
        || created > limits.entry_count
        || directory_fds > limits.directory_fds
        || cleanup_work > limits.cleanup_work
        || need.payload_bytes > limits.payload_bytes
        || limits.staging_attempts == 0
    {
        return None;
    }
    Some(cleanup_work)
}

}
