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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestorePreflightCharge {
    pub created_entries: usize,
    pub cleanup_work: usize,
}

pub open spec fn restore_preflight_model(
    need: RestorePreflightNeed,
    limits: RestorePreflightLimits,
) -> Option<RestorePreflightCharge> {
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
        Some(RestorePreflightCharge {
            created_entries: created as usize,
            cleanup_work: cleanup_work as usize,
        })
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
) -> (result: Option<RestorePreflightCharge>)
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
    Some(RestorePreflightCharge {
        created_entries: created,
        cleanup_work,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CleanupSchedule {
    pub created_entries: usize,
    pub reserved_work: usize,
    pub planned_nodes: usize,
    pub opened_node_directories: usize,
    pub journaled_nodes: usize,
    pub root_opened: bool,
    pub wrapper_opened: bool,
    pub protected_profile: bool,
    pub root_attempted: bool,
}

pub open spec fn cleanup_schedule_model(schedule: CleanupSchedule) -> Option<usize> {
    let overhead = if schedule.protected_profile { 2int } else { 1int };
    let node_and_root_repairs = schedule.opened_node_directories as int
        + (if schedule.root_opened { 1int } else { 0int });
    let mode_repairs = node_and_root_repairs
        + (if schedule.wrapper_opened { 1int } else { 0int });
    let root_removal = if schedule.protected_profile
        && schedule.root_attempted && schedule.wrapper_opened
    { 1int } else { 0int };
    let attempts_after_nodes = mode_repairs + schedule.journaled_nodes as int;
    let attempts_after_staging = attempts_after_nodes + 1;
    let attempts = attempts_after_staging + root_removal;
    let reservation = schedule.created_entries as int * 2;
    if schedule.created_entries as int == schedule.planned_nodes as int + overhead
        && schedule.opened_node_directories <= schedule.planned_nodes
        && schedule.journaled_nodes <= schedule.planned_nodes
        && (!schedule.wrapper_opened || schedule.protected_profile)
        && schedule.reserved_work as int == reservation
        && node_and_root_repairs <= usize::MAX as int
        && mode_repairs <= usize::MAX as int
        && attempts_after_nodes <= usize::MAX as int
        && attempts_after_staging <= usize::MAX as int
        && attempts <= reservation
        && attempts <= usize::MAX as int
    {
        Some(attempts as usize)
    } else {
        None
    }
}

proof fn cleanup_repair_count_bound(schedule: CleanupSchedule)
    requires
        schedule.created_entries as int
            >= (if schedule.protected_profile { 2int } else { 1int }),
        schedule.planned_nodes as int
            == schedule.created_entries as int
                - (if schedule.protected_profile { 2int } else { 1int }),
        schedule.opened_node_directories <= schedule.planned_nodes,
        !schedule.wrapper_opened || schedule.protected_profile,
    ensures
        schedule.opened_node_directories as int
            + (if schedule.root_opened { 1int } else { 0int })
            + (if schedule.wrapper_opened { 1int } else { 0int })
            <= schedule.created_entries as int,
{
    if schedule.protected_profile {
        assert(schedule.planned_nodes as int + 2 == schedule.created_entries as int);
        assert((if schedule.root_opened { 1int } else { 0int })
            + (if schedule.wrapper_opened { 1int } else { 0int }) <= 2);
        assert(schedule.opened_node_directories as int <= schedule.planned_nodes as int);
        assert(schedule.opened_node_directories as int
            + (if schedule.root_opened { 1int } else { 0int })
            + (if schedule.wrapper_opened { 1int } else { 0int })
            <= schedule.planned_nodes as int + 2);
    } else {
        assert(!schedule.wrapper_opened);
        assert((if schedule.wrapper_opened { 1int } else { 0int }) == 0);
        assert((if schedule.root_opened { 1int } else { 0int }) <= 1);
        assert(schedule.planned_nodes as int + 1 == schedule.created_entries as int);
        assert(schedule.opened_node_directories as int <= schedule.planned_nodes as int);
        assert(schedule.opened_node_directories as int
            + (if schedule.root_opened { 1int } else { 0int }) <= schedule.planned_nodes as int + 1);
    }
}

//= docs/parsing-requirements.md#restore-012
//= type=implementation
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
//= docs/parsing-requirements.md#restore-022
//= type=implementation
//# On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.
pub(crate) fn cleanup_schedule_bound(schedule: CleanupSchedule) -> (result: Option<usize>)
    ensures result == cleanup_schedule_model(schedule),
{
    let overhead = if schedule.protected_profile { 2usize } else { 1usize };
    if schedule.created_entries < overhead {
        return None;
    }
    let planned_from_charge = schedule.created_entries - overhead;
    if schedule.planned_nodes != planned_from_charge
        || schedule.created_entries > usize::MAX / 2
        || schedule.opened_node_directories > schedule.planned_nodes
        || schedule.journaled_nodes > schedule.planned_nodes
        || (schedule.wrapper_opened && !schedule.protected_profile)
        || schedule.reserved_work != schedule.created_entries * 2
    {
        return None;
    }
    let reservation = schedule.created_entries * 2;
    proof {
        cleanup_repair_count_bound(schedule);
    }
    let root_repair = schedule.root_opened as usize;
    assert(root_repair as int == if schedule.root_opened { 1int } else { 0int });
    assert(schedule.opened_node_directories as int + root_repair as int
        + (if schedule.wrapper_opened { 1int } else { 0int })
        <= schedule.created_entries as int);
    assert(schedule.created_entries as int <= usize::MAX as int);
    let node_and_root_repairs = schedule.opened_node_directories + root_repair;
    let wrapper_repair = schedule.wrapper_opened as usize;
    assert(wrapper_repair as int == if schedule.wrapper_opened { 1int } else { 0int });
    assert(node_and_root_repairs as int + wrapper_repair as int <= schedule.created_entries as int);
    let mode_repairs = node_and_root_repairs + wrapper_repair;
    let root_removal =
        (schedule.protected_profile && schedule.root_attempted && schedule.wrapper_opened) as usize;
    assert(mode_repairs as int + schedule.journaled_nodes as int + 1 + root_removal as int
        <= schedule.reserved_work as int);
    assert(mode_repairs as int + schedule.journaled_nodes as int <= usize::MAX as int);
    let attempts_after_nodes = mode_repairs + schedule.journaled_nodes;
    assert(attempts_after_nodes as int + 1 <= usize::MAX as int);
    let attempts_after_staging = attempts_after_nodes + 1;
    assert(attempts_after_staging as int + root_removal as int <= usize::MAX as int);
    let attempts = attempts_after_staging + root_removal;
    if attempts > reservation { return None; }
    Some(attempts)
}

}
