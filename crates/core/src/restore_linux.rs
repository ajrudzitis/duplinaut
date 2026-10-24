//! Native Linux publication controller. Safe syscall integration is trusted;
//! no complete controller or filesystem formal proof is claimed.
#![forbid(unsafe_code)]
use crate::{
    restore_plan::RestorePlan,
    source::Cancellation,
    state::SnapshotEntry,
    tar::{EntryType, TarTime},
};
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags, Stat, Timestamps},
    io::Errno,
    time::Timespec,
};
use std::{ffi::CString, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    PrivateParentSameParent,
    ProtectedParentWrapper,
}
#[derive(Clone, Copy, Debug)]
pub struct RestoreLimits {
    pub entries: usize,
    pub directory_fds: usize,
    pub metadata_bytes: usize,
    pub name_bytes: usize,
    pub payload_bytes: u64,
    pub forward_work: usize,
    pub staging_attempts: usize,
    pub cleanup_work: usize,
    pub profile: Option<Profile>,
}
impl Default for RestoreLimits {
    fn default() -> Self {
        Self {
            entries: 1_000_000,
            directory_fds: 100_000,
            metadata_bytes: 256 * 1024 * 1024,
            name_bytes: 64 * 1024 * 1024,
            payload_bytes: 8 * 1024 * 1024 * 1024,
            forward_work: 32_000_000,
            staging_attempts: 16,
            cleanup_work: 4_000_000,
            profile: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Limit,
    Allocation,
    Cancelled,
    InvalidPlan,
    InvalidDestination,
    ParentPolicy,
    Filesystem,
    Random,
    Fidelity,
    Syscall(Errno),
}
impl From<Errno> for Cause {
    fn from(e: Errno) -> Self {
        Self::Syscall(e)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Cleanup {
    pub attempted: usize,
    pub failures: usize,
    pub first_error: Option<Errno>,
    pub budget_exhausted: bool,
    pub diagnostics_truncated: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Identity {
    pub device: u64,
    pub inode: u64,
}
fn identity(s: &Stat) -> Identity {
    Identity {
        device: s.st_dev,
        inode: s.st_ino,
    }
}
#[derive(Debug)]
pub struct Receipt {
    pub destination: CString,
    pub staging: CString,
    pub profile: Profile,
    pub parent: Identity,
    pub root: Identity,
    pub filesystem: i64,
    pub unspecified_empty_root_mtime: bool,
    pub cleanup: Cleanup,
}
#[derive(Debug)]
pub struct Unpublished {
    pub cause: Cause,
    pub cleanup: Cleanup,
    pub uncertain_outer: Option<CString>,
}
#[derive(Debug)]
pub struct Indeterminate {
    pub error: Errno,
    pub receipt: Receipt,
}
#[derive(Debug)]
pub enum RestoreOutcome {
    Published(Receipt),
    Unpublished(Unpublished),
    PublicationIndeterminate(Indeterminate),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    OpenParent,
    Stat,
    CreateOuter,
    CreateDirectory,
    CreateFile,
    CreateLink,
    OpenDirectory,
    Chmod,
    AclRemove,
    AclCheck,
    Write,
    SetTime,
    ReadLink,
    Publish,
    CleanupMode,
    CleanupUnlink,
}
trait Hooks {
    fn write(&mut self, fd: BorrowedFd<'_>, bytes: &[u8]) -> Result<usize, Errno> {
        rustix::io::write(fd, bytes)
    }
    fn before(&mut self, _: Op) -> Result<(), Errno> {
        Ok(())
    }
    fn after(&mut self, _: Op) -> Result<(), Errno> {
        Ok(())
    }
}
struct Native;
impl Hooks for Native {}
struct Forward<'a, H> {
    left: usize,
    cancel: &'a Cancellation,
    hooks: &'a mut H,
}
impl<H: Hooks> Forward<'_, H> {
    fn tick(&mut self) -> Result<(), Cause> {
        if self.cancel.is_cancelled() {
            return Err(Cause::Cancelled);
        }
        self.left = self.left.checked_sub(1).ok_or(Cause::Limit)?;
        Ok(())
    }
    fn create<T>(
        &mut self,
        op: Op,
        state: &mut Journal,
        f: impl FnOnce() -> Result<T, Errno>,
    ) -> Result<T, Cause> {
        *state = Journal::PossiblyCreated;
        self.tick()?;
        self.hooks.before(op)?;
        let value = f()?;
        *state = Journal::Created;
        self.hooks.after(op)?;
        Ok(value)
    }
    fn write(&mut self, fd: BorrowedFd<'_>, bytes: &[u8]) -> Result<usize, Cause> {
        self.tick()?;
        self.hooks.before(Op::Write)?;
        let n = self.hooks.write(fd, bytes)?;
        self.hooks.after(Op::Write).map_err(|e| {
            if e == Errno::INTR {
                Cause::Syscall(Errno::IO)
            } else {
                Cause::Syscall(e)
            }
        })?;
        Ok(n)
    }
    fn call<T>(&mut self, op: Op, f: impl FnOnce() -> Result<T, Errno>) -> Result<T, Cause> {
        self.tick()?;
        self.hooks.before(op)?;
        let result = f()?;
        self.hooks.after(op)?;
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Journal {
    Unattempted,
    PossiblyCreated,
    Created,
}
struct Node<'a> {
    path: &'a crate::path::BackupPath,
    entry: &'a SnapshotEntry,
    parent: usize,
    name: CString,
    target: Option<CString>,
    journal: Journal,
}
struct Prepared<'a> {
    nodes: Vec<Node<'a>>,
    dirs: Vec<Option<OwnedFd>>,
    root: Option<&'a SnapshotEntry>,
    destination: CString,
    staging: CString,
    profile: Profile,
    cleanup_required: usize,
}
fn component(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes != b"."
        && bytes != b".."
        && !bytes.contains(&0)
        && !bytes.contains(&b'/')
}
fn checked_add(sum: &mut usize, n: usize, max: usize) -> Result<(), Cause> {
    *sum = sum
        .checked_add(n)
        .filter(|n| *n <= max)
        .ok_or(Cause::Limit)?;
    Ok(())
}
fn times(t: TarTime) -> Result<Timestamps, Cause> {
    if t.nanoseconds >= 1_000_000_000 {
        return Err(Cause::InvalidPlan);
    }
    Ok(Timestamps {
        last_access: Timespec {
            tv_sec: 0,
            tv_nsec: fs::UTIME_OMIT,
        },
        last_modification: Timespec {
            // Pinned Linux rustix Timespec seconds and TarTime seconds are i64.
            tv_sec: t.seconds,
            tv_nsec: t.nanoseconds.into(),
        },
    })
}
//= docs/parsing-requirements.md#restore-011
//= type=implementation
//# The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.
//= docs/parsing-requirements.md#restore-012
//= type=implementation
//# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
//= docs/parsing-requirements.md#restore-025
//= type=implementation
//# The backend MUST select and validate its publication profile before creating staging, without changing destination-parent permissions to qualify for either profile.
fn prepare<'a>(
    plan: &'a RestorePlan<'a>,
    dest: &[u8],
    parent_bytes: usize,
    profile: Profile,
    limits: RestoreLimits,
    work: &mut Forward<'_, impl Hooks>,
) -> Result<Prepared<'a>, Cause> {
    if !component(dest) || dest.len() > 255 {
        return Err(Cause::InvalidDestination);
    }
    let entries = plan.entries();
    if entries.len() > limits.entries.saturating_add(1) {
        return Err(Cause::Limit);
    }
    let root = entries
        .iter()
        .find(|e| e.path().components().is_empty())
        .map(|e| e.entry());
    if !entries.is_empty() && root.is_none() {
        return Err(Cause::InvalidPlan);
    }
    let root_mode = root.map_or(0o700, |e| e.mode() & 0o777);
    if root.is_some_and(|e| e.entry_type() != EntryType::Directory)
        || (profile == Profile::ProtectedParentWrapper && root_mode & 0o200 == 0)
    {
        return Err(Cause::InvalidPlan);
    }
    let overhead = if profile == Profile::ProtectedParentWrapper {
        2usize
    } else {
        1
    };
    let created = entries
        .len()
        .saturating_sub(usize::from(root.is_some()))
        .checked_add(overhead)
        .ok_or(Cause::Limit)?;
    if created > limits.entries || limits.staging_attempts == 0 {
        return Err(Cause::Limit);
    }
    let directories = entries
        .iter()
        .filter(|e| e.entry().entry_type() == EntryType::Directory)
        .count()
        .checked_add(usize::from(root.is_none()))
        .ok_or(Cause::Limit)?;
    let active = usize::from(
        entries
            .iter()
            .any(|e| e.entry().entry_type() == EntryType::Regular),
    );
    if directories
        .checked_add(overhead)
        .and_then(|n| n.checked_add(active))
        .filter(|n| *n <= limits.directory_fds)
        .is_none()
    {
        return Err(Cause::Limit);
    }
    let cleanup_required = created.checked_mul(2).ok_or(Cause::Limit)?;
    if cleanup_required > limits.cleanup_work {
        return Err(Cause::Limit);
    }
    let mut names = (dest.len() + 1 + 128)
        .checked_add(parent_bytes)
        .ok_or(Cause::Limit)?;
    let mut metadata = std::mem::size_of::<Prepared<'_>>()
        + std::mem::size_of::<Receipt>()
        + std::mem::size_of::<CString>()
        + std::mem::size_of::<OwnedFd>();
    checked_add(&mut names, 5, limits.name_bytes)?;
    checked_add(&mut metadata, names, limits.metadata_bytes)?;
    checked_add(
        &mut metadata,
        entries
            .len()
            .checked_mul(std::mem::size_of::<Node<'_>>() + std::mem::size_of::<Option<OwnedFd>>())
            .ok_or(Cause::Limit)?,
        limits.metadata_bytes,
    )?;
    checked_add(
        &mut metadata,
        std::mem::size_of::<Option<OwnedFd>>(),
        limits.metadata_bytes,
    )?;
    let mut payload = 0u64;
    // Validate everything and budget every CString before allocation/staging.
    for entry in entries {
        work.tick()?;
        times(entry.entry().mtime())?;
        for part in entry.path().components() {
            work.tick()?;
            if !component(part) || part.len() > 255 {
                return Err(Cause::InvalidPlan);
            }
        }
        let parts = entry.path().components();
        if let Some(name) = parts.last() {
            checked_add(&mut names, name.len() + 1, limits.name_bytes)?;
            checked_add(&mut metadata, name.len() + 1, limits.metadata_bytes)?;
        }
        match entry.entry().entry_type() {
            EntryType::Directory => {}
            EntryType::Regular => {
                payload = payload
                    .checked_add(entry.entry().content().ok_or(Cause::InvalidPlan)?.len() as u64)
                    .filter(|n| *n <= limits.payload_bytes)
                    .ok_or(Cause::Limit)?;
            }
            EntryType::Symlink => {
                let target = entry.entry().linkname();
                if target.is_empty()
                    || target.contains(&0)
                    || target.len() > 4095
                    || entry.entry().mode() & 0o777 != 0o777
                {
                    return Err(Cause::InvalidPlan);
                }
                checked_add(&mut names, target.len() + 1, limits.name_bytes)?;
                checked_add(&mut metadata, target.len() + 1, limits.metadata_bytes)?;
            }
            _ => return Err(Cause::InvalidPlan),
        }
        if !parts.is_empty() {
            let parent = &parts[..parts.len() - 1];
            let mut found = false;
            for candidate in entries {
                work.tick()?;
                if candidate.path().components() == parent
                    && candidate.entry().entry_type() == EntryType::Directory
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Cause::InvalidPlan);
            }
        }
    }
    let mut nodes: Vec<Node<'_>> = Vec::new();
    nodes
        .try_reserve_exact(entries.len())
        .map_err(|_| Cause::Allocation)?;
    let mut dirs = Vec::new();
    dirs.try_reserve_exact(entries.len() + 1)
        .map_err(|_| Cause::Allocation)?;
    dirs.resize_with(entries.len() + 1, || None);
    // Plan order is byte-component lexical: each directory precedes descendants.
    for entry in entries {
        work.tick()?;
        let parts = entry.path().components();
        if parts.is_empty() {
            continue;
        }
        let parent_parts = &parts[..parts.len() - 1];
        let parent = if parent_parts.is_empty() {
            0
        } else {
            {
                let mut index = None;
                for (i, n) in nodes.iter().enumerate() {
                    work.tick()?;
                    if n.path.components() == parent_parts {
                        index = Some(i + 1);
                        break;
                    }
                }
                index.ok_or(Cause::InvalidPlan)?
            }
        };
        nodes.push(Node {
            path: entry.path(),
            entry: entry.entry(),
            parent,
            name: CString::new(parts.last().unwrap().as_slice()).map_err(|_| Cause::InvalidPlan)?,
            target: if entry.entry().entry_type() == EntryType::Symlink {
                Some(CString::new(entry.entry().linkname()).map_err(|_| Cause::InvalidPlan)?)
            } else {
                None
            },
            journal: Journal::Unattempted,
        });
    }
    Ok(Prepared {
        nodes,
        dirs,
        root,
        destination: CString::new(dest).map_err(|_| Cause::InvalidDestination)?,
        staging: CString::new(vec![b'x'; 63]).map_err(|_| Cause::Allocation)?,
        profile,
        cleanup_required,
    })
}
fn fd(dirs: &[Option<OwnedFd>], index: usize) -> Result<BorrowedFd<'_>, Cause> {
    Ok(dirs
        .get(index)
        .and_then(Option::as_ref)
        .ok_or(Cause::InvalidPlan)?
        .as_fd())
}
//= docs/parsing-requirements.md#restore-016
//= type=implementation
//# The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.
fn clear_acl(
    fd: BorrowedFd<'_>,
    directory: bool,
    w: &mut Forward<'_, impl Hooks>,
) -> Result<(), Cause> {
    for name in [c"system.posix_acl_access", c"system.posix_acl_default"]
        .into_iter()
        .take(if directory { 2 } else { 1 })
    {
        let removal = w.call(Op::AclRemove, || fs::fremovexattr(fd, name));
        match removal {
            Ok(()) | Err(Cause::Syscall(Errno::NODATA | Errno::OPNOTSUPP)) => {}
            Err(e) => return Err(e),
        }
        let mut buffer = [0u8; 1];
        match w.call(Op::AclCheck, || fs::fgetxattr(fd, name, &mut buffer[..])) {
            Err(Cause::Syscall(Errno::NODATA | Errno::OPNOTSUPP)) => {}
            Ok(_) => return Err(Cause::Fidelity),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
fn init_dir(fd: BorrowedFd<'_>, w: &mut Forward<'_, impl Hooks>) -> Result<(), Cause> {
    clear_acl(fd, true, w)?;
    w.call(Op::Chmod, || fs::fchmod(fd, Mode::from_raw_mode(0o700)))?;
    let stat = w.call(Op::Stat, || fs::fstat(fd))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_uid != rustix::process::geteuid().as_raw()
        || stat.st_mode & 0o777 != 0o700
    {
        return Err(Cause::Fidelity);
    }
    Ok(())
}
fn verify(stat: &Stat, entry: &SnapshotEntry) -> Result<(), Cause> {
    let kind = match entry.entry_type() {
        EntryType::Directory => FileType::Directory,
        EntryType::Regular => FileType::RegularFile,
        EntryType::Symlink => FileType::Symlink,
        _ => return Err(Cause::InvalidPlan),
    };
    if FileType::from_raw_mode(stat.st_mode) != kind
        || stat.st_mode & 0o777 != entry.mode() & 0o777
        || stat.st_mtime != entry.mtime().seconds
        || stat.st_mtime_nsec != u64::from(entry.mtime().nanoseconds)
    {
        return Err(Cause::Fidelity);
    }
    if entry.entry_type() == EntryType::Regular
        && u64::try_from(stat.st_size).ok() != entry.content().map(|v| v.len() as u64)
    {
        return Err(Cause::Fidelity);
    }
    Ok(())
}
fn finish_fd(
    fd: BorrowedFd<'_>,
    entry: &SnapshotEntry,
    w: &mut Forward<'_, impl Hooks>,
) -> Result<(), Cause> {
    clear_acl(fd, entry.entry_type() == EntryType::Directory, w)?;
    w.call(Op::Chmod, || {
        fs::fchmod(fd, Mode::from_raw_mode(entry.mode() & 0o777))
    })?;
    let ts = times(entry.mtime())?;
    w.call(Op::SetTime, || fs::futimens(fd, &ts))?;
    verify(&w.call(Op::Stat, || fs::fstat(fd))?, entry)
}
//= docs/parsing-requirements.md#restore-001
//= type=implementation
//# A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.
//= docs/parsing-requirements.md#restore-003
//= type=implementation
//# Restoration filesystem operations MUST prevent traversal through symlinks outside the private staging tree.
//= docs/parsing-requirements.md#restore-004
//= type=implementation
//# The reader MUST restore supported regular files, directories, and symlinks with ordinary permissions and supported modification times.
//= docs/parsing-requirements.md#restore-010
//= type=implementation
//# All operations on backup entries MUST use their original byte-valued single path components relative to retained directory FDs, without reopening a directory by a joined path or following a restored symlink.
//= docs/parsing-requirements.md#restore-013
//= type=implementation
//# The backend MUST pre-reserve a journal state for every attempted creation and preserve possibly-created as well as definitely-created entries through handled failure cleanup.
//= docs/parsing-requirements.md#restore-014
//= type=implementation
//# Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.
//= docs/parsing-requirements.md#restore-015
//= type=implementation
//# Symlinks MUST preserve the exact supported target bytes and receive their exact mtime through a no-follow operation relative to their retained parent FD.
//= docs/parsing-requirements.md#restore-017
//= type=implementation
//# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
//= docs/parsing-requirements.md#restore-018
//= type=implementation
//# Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.
//= docs/parsing-requirements.md#restore-019
//= type=implementation
//# When a genuinely empty complete plan has no source-root metadata, the backend MUST publish an empty root with explicit policy mode `0700` without reporting an invented source-root mtime.
fn populate(p: &mut Prepared<'_>, w: &mut Forward<'_, impl Hooks>) -> Result<(), Cause> {
    for i in 0..p.nodes.len() {
        w.tick()?;
        let node = &mut p.nodes[i];
        let parent = fd(&p.dirs, node.parent)?;
        match node.entry.entry_type() {
            EntryType::Directory => {
                w.create(Op::CreateDirectory, &mut node.journal, || {
                    fs::mkdirat(parent, node.name.as_c_str(), Mode::from_raw_mode(0o700))
                })?;
                // Parent namespace is exclusively owned; creation umask can
                // remove owner search/read, so initialize this unique leaf.
                w.call(Op::Chmod, || {
                    fs::chmodat(
                        parent,
                        node.name.as_c_str(),
                        Mode::from_raw_mode(0o700),
                        AtFlags::empty(),
                    )
                })?;
                let child = w.call(Op::OpenDirectory, || {
                    fs::openat(
                        parent,
                        node.name.as_c_str(),
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                })?;
                p.dirs[i + 1] = Some(child);
                init_dir(fd(&p.dirs, i + 1)?, w)?;
            }
            EntryType::Regular => {
                let file = w.create(Op::CreateFile, &mut node.journal, || {
                    fs::openat(
                        parent,
                        node.name.as_c_str(),
                        OFlags::CREATE
                            | OFlags::EXCL
                            | OFlags::NOFOLLOW
                            | OFlags::CLOEXEC
                            | OFlags::WRONLY,
                        Mode::from_raw_mode(0o600),
                    )
                })?;
                let content = node.entry.content().ok_or(Cause::InvalidPlan)?;
                let mut offset = 0;
                while offset < content.len() {
                    let end = content.len().min(offset.saturating_add(65536));
                    match w.write(file.as_fd(), &content[offset..end]) {
                        Ok(n) => {
                            offset = crate::verified::restore_write::advance_write_offset(
                                offset, n, end,
                            )
                            .ok_or(Cause::Fidelity)?;
                        }
                        Err(Cause::Syscall(Errno::INTR)) => continue,
                        Err(e) => return Err(e),
                    }
                }
                finish_fd(file.as_fd(), node.entry, w)?;
            }
            EntryType::Symlink => {
                let target = node.target.as_ref().ok_or(Cause::InvalidPlan)?;
                w.create(Op::CreateLink, &mut node.journal, || {
                    fs::symlinkat(target.as_c_str(), parent, node.name.as_c_str())
                })?;
                let ts = times(node.entry.mtime())?;
                w.call(Op::SetTime, || {
                    fs::utimensat(parent, node.name.as_c_str(), &ts, AtFlags::SYMLINK_NOFOLLOW)
                })?;
                verify(
                    &w.call(Op::Stat, || {
                        fs::statat(parent, node.name.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)
                    })?,
                    node.entry,
                )?;
                let mut buffer = [0u8; 4096];
                let n = w.call(Op::ReadLink, || {
                    fs::readlinkat_raw(parent, node.name.as_c_str(), &mut buffer[..])
                })?;
                if &buffer[..n] != target.to_bytes() {
                    return Err(Cause::Fidelity);
                }
            }
            _ => return Err(Cause::InvalidPlan),
        }
    }
    // Child directories before parents; cached FDs survive restrictive modes.
    for i in (0..p.nodes.len()).rev() {
        w.tick()?;
        if p.nodes[i].entry.entry_type() == EntryType::Directory {
            finish_fd(fd(&p.dirs, i + 1)?, p.nodes[i].entry, w)?;
        }
    }
    if let Some(root) = p.root {
        finish_fd(fd(&p.dirs, 0)?, root, w)?;
    } else {
        let root = fd(&p.dirs, 0)?;
        clear_acl(root, true, w)?;
        w.call(Op::Chmod, || fs::fchmod(root, Mode::from_raw_mode(0o700)))?;
        let s = w.call(Op::Stat, || fs::fstat(root))?;
        if s.st_mode & 0o777 != 0o700 || FileType::from_raw_mode(s.st_mode) != FileType::Directory {
            return Err(Cause::Fidelity);
        }
    }
    Ok(())
}
fn cleanup_call<H: Hooks>(
    report: &mut Cleanup,
    left: &mut usize,
    hooks: &mut H,
    op: Op,
    f: impl FnOnce() -> Result<(), Errno>,
) {
    if *left == 0 {
        report.budget_exhausted = true;
        return;
    }
    *left -= 1;
    report.attempted += 1;
    let result = hooks
        .before(op)
        .and_then(|()| f())
        .and_then(|()| hooks.after(op));
    if let Err(error) = result {
        if error != Errno::NOENT {
            report.failures += 1;
            report.diagnostics_truncated = report.failures > 1;
            if report.first_error.is_none() {
                report.first_error = Some(error);
            }
        }
    }
}
//= docs/parsing-requirements.md#restore-006
//= type=implementation
//# Cancellation observed before a publication attempt or a handled failure before that attempt MUST leave the restored tree unpublished at the requested destination and attempt removal of temporary plaintext from exclusively owned staging.
//= docs/parsing-requirements.md#restore-022
//= type=implementation
//# On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.
fn cleanup<H: Hooks>(
    p: &Prepared<'_>,
    parent: BorrowedFd<'_>,
    wrapper: Option<&OwnedFd>,
    root_attempted: bool,
    hooks: &mut H,
) -> Cleanup {
    let mut report = Cleanup::default();
    let mut left = p.cleanup_required;
    // Parent-first mode repair before relative child unlink operations.
    if let Some(wrapper) = wrapper {
        cleanup_call(&mut report, &mut left, hooks, Op::CleanupMode, || {
            fs::fchmod(wrapper, Mode::from_raw_mode(0o700))
        });
    }
    if let Some(root) = &p.dirs[0] {
        cleanup_call(&mut report, &mut left, hooks, Op::CleanupMode, || {
            fs::fchmod(root, Mode::from_raw_mode(0o700))
        });
    }
    for i in 0..p.nodes.len() {
        if let Some(dir) = &p.dirs[i + 1] {
            cleanup_call(&mut report, &mut left, hooks, Op::CleanupMode, || {
                fs::fchmod(dir, Mode::from_raw_mode(0o700))
            });
        }
    }
    for node in p
        .nodes
        .iter()
        .rev()
        .filter(|n| n.journal != Journal::Unattempted)
    {
        if let Ok(fd) = fd(&p.dirs, node.parent) {
            let flags = if node.entry.entry_type() == EntryType::Directory {
                AtFlags::REMOVEDIR
            } else {
                AtFlags::empty()
            };
            cleanup_call(&mut report, &mut left, hooks, Op::CleanupUnlink, || {
                fs::unlinkat(fd, node.name.as_c_str(), flags)
            });
        }
    }
    if p.profile == Profile::PrivateParentSameParent {
        cleanup_call(&mut report, &mut left, hooks, Op::CleanupUnlink, || {
            fs::unlinkat(parent, p.staging.as_c_str(), AtFlags::REMOVEDIR)
        });
    } else {
        if root_attempted {
            if let Some(wrapper) = wrapper {
                cleanup_call(&mut report, &mut left, hooks, Op::CleanupUnlink, || {
                    fs::unlinkat(wrapper, c"root", AtFlags::REMOVEDIR)
                });
            }
        }
        cleanup_call(&mut report, &mut left, hooks, Op::CleanupUnlink, || {
            fs::unlinkat(parent, p.staging.as_c_str(), AtFlags::REMOVEDIR)
        });
    }
    report
}
fn failed(cause: Cause) -> RestoreOutcome {
    RestoreOutcome::Unpublished(Unpublished {
        cause,
        cleanup: Cleanup::default(),
        uncertain_outer: None,
    })
}
// Public API accepts only an already-built immutable plan; no source/network IO.
//= docs/parsing-requirements.md#restore-024
//= type=implementation
//# Restoration assurance evidence MUST separate pure plan/controller guarantees from trusted syscall, rustix, kernel, filesystem and allocator assumptions.
pub fn restore(
    plan: &RestorePlan<'_>,
    parent: &Path,
    destination: &[u8],
    cancel: &Cancellation,
    limits: RestoreLimits,
) -> RestoreOutcome {
    restore_with(plan, parent, destination, cancel, limits, &mut Native)
}
//= docs/parsing-requirements.md#restore-002
//= type=implementation
//# A restoration MUST write into private staging on the destination filesystem and publish the completed tree without replacing an existing destination.
//= docs/parsing-requirements.md#restore-007
//= type=implementation
//# The initial Linux backend MUST reject destination parents whose filesystem type is outside the reviewed local allowlist before creating staging.
//= docs/parsing-requirements.md#restore-008
//= type=implementation
//# The backend MUST reject a destination parent unless its opened directory is owned by the effective UID, has owner write/search permissions, and has neither group-class nor other-class write permission.
//= docs/parsing-requirements.md#restore-009
//= type=implementation
//# The backend MUST keep every provisional restored entry behind its selected profile's private directory boundary until publication or cleanup.
//= docs/parsing-requirements.md#restore-020
//= type=implementation
//# The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.
//= docs/parsing-requirements.md#restore-021
//= type=implementation
//# Cancellation MUST be observed before each entry, each bounded write attempt and the final publication attempt, with successful publication taking precedence over a racing cancellation request.
//= docs/parsing-requirements.md#restore-023
//= type=implementation
//# The result MUST distinguish unpublished failure/cancellation, published success, and publication-indeterminate outcomes, retaining post-success wrapper-removal faults as cleanup warnings on published success.
//= docs/parsing-requirements.md#restore-026
//= type=implementation
//# If a publication rename is attempted and returns an error, the backend MUST report publication uncertainty and refrain from modifying cleanup of the restore root, its descendants or its staging wrapper.
//= docs/parsing-requirements.md#restore-027
//= type=implementation
//# If creation of the first outer staging leaf returns an error other than a definite existing-entry collision, the backend MUST report its candidate as uncertain and leave that candidate untouched because exclusive ownership was not established.
fn restore_with<H: Hooks>(
    plan: &RestorePlan<'_>,
    parent_path: &Path,
    dest: &[u8],
    cancel: &Cancellation,
    limits: RestoreLimits,
    hooks: &mut H,
) -> RestoreOutcome {
    if limits.directory_fds == 0 {
        return failed(Cause::Limit);
    }
    use std::os::unix::ffi::OsStrExt;
    let parent_bytes = parent_path.as_os_str().as_bytes();
    let charged_parent = match parent_bytes.len().checked_add(1) {
        Some(n) => n,
        None => return failed(Cause::Limit),
    };
    if charged_parent > limits.name_bytes
        || charged_parent
            .checked_add(std::mem::size_of::<CString>() + std::mem::size_of::<OwnedFd>())
            .filter(|n| *n <= limits.metadata_bytes)
            .is_none()
    {
        return failed(Cause::Limit);
    }
    if cancel.is_cancelled() {
        return failed(Cause::Cancelled);
    }
    let parent_name = match CString::new(parent_bytes) {
        Ok(n) => n,
        Err(_) => return failed(Cause::ParentPolicy),
    };
    let mut w = Forward {
        left: limits.forward_work,
        cancel,
        hooks,
    };
    let parent = match w.call(Op::OpenParent, || {
        fs::open(
            parent_name.as_c_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
    }) {
        Ok(v) => v,
        Err(e) => return failed(e),
    };
    let stat = match w.call(Op::Stat, || fs::fstat(&parent)) {
        Ok(s) => s,
        Err(e) => return failed(e),
    };
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_uid != rustix::process::geteuid().as_raw()
        || stat.st_mode & 0o300 != 0o300
        || stat.st_mode & 0o022 != 0
    {
        return failed(Cause::ParentPolicy);
    }
    let filesystem = match w.call(Op::Stat, || fs::fstatfs(&parent)) {
        Ok(s) => s.f_type,
        Err(e) => return failed(e),
    };
    if ![0xef53, 0x9123683e, 0x58465342, 0x01021994].contains(&filesystem) {
        return failed(Cause::Filesystem);
    }
    let profile = limits.profile.unwrap_or(if stat.st_mode & 0o077 == 0 {
        Profile::PrivateParentSameParent
    } else {
        Profile::ProtectedParentWrapper
    });
    if profile == Profile::PrivateParentSameParent && stat.st_mode & 0o077 != 0 {
        return failed(Cause::ParentPolicy);
    }
    let mut p = match prepare(plan, dest, charged_parent, profile, limits, &mut w) {
        Ok(p) => p,
        Err(e) => return failed(e),
    };
    let mut outer_owned = false;
    for _ in 0..limits.staging_attempts {
        if let Err(e) = w.tick() {
            return failed(e);
        }
        let mut random = [0u8; 16];
        if getrandom::fill(&mut random).is_err() {
            return failed(Cause::Random);
        }
        let mut name = [b'0'; 43];
        name[..11].copy_from_slice(b".duplicity-");
        let hex = b"0123456789abcdef";
        for (i, b) in random.iter().enumerate() {
            name[11 + i * 2] = hex[(b >> 4) as usize];
            name[12 + i * 2] = hex[(b & 15) as usize];
        }
        p.staging = CString::new(name).expect("fixed staging bytes have no NUL");
        if p.staging.to_bytes() == dest {
            continue;
        }
        match w.call(Op::CreateOuter, || {
            fs::mkdirat(&parent, p.staging.as_c_str(), Mode::from_raw_mode(0o700))
        }) {
            Ok(()) => {
                outer_owned = true;
                break;
            }
            Err(Cause::Syscall(Errno::EXIST)) => continue,
            Err(cause) => {
                return RestoreOutcome::Unpublished(Unpublished {
                    cause,
                    cleanup: Cleanup::default(),
                    uncertain_outer: Some(p.staging),
                })
            }
        }
    }
    if !outer_owned {
        return failed(Cause::Limit);
    }
    let mut wrapper = None;
    let mut root_journal = if profile == Profile::PrivateParentSameParent {
        Journal::Created
    } else {
        Journal::Unattempted
    };
    let setup = (|| -> Result<(), Cause> {
        w.call(Op::Chmod, || {
            fs::chmodat(
                &parent,
                p.staging.as_c_str(),
                Mode::from_raw_mode(0o700),
                AtFlags::empty(),
            )
        })?;
        let outer = w.call(Op::OpenDirectory, || {
            fs::openat(
                &parent,
                p.staging.as_c_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
        })?;
        if profile == Profile::PrivateParentSameParent {
            p.dirs[0] = Some(outer);
            init_dir(fd(&p.dirs, 0)?, &mut w)?;
        } else {
            wrapper = Some(outer);
            let outer = wrapper.as_ref().unwrap();
            init_dir(outer.as_fd(), &mut w)?;
            w.create(Op::CreateDirectory, &mut root_journal, || {
                fs::mkdirat(outer, c"root", Mode::from_raw_mode(0o700))
            })?;
            w.call(Op::Chmod, || {
                fs::chmodat(outer, c"root", Mode::from_raw_mode(0o700), AtFlags::empty())
            })?;
            let root = w.call(Op::OpenDirectory, || {
                fs::openat(
                    outer,
                    c"root",
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
            })?;
            p.dirs[0] = Some(root);
            init_dir(fd(&p.dirs, 0)?, &mut w)?;
        }
        populate(&mut p, &mut w)
    })();
    if let Err(cause) = setup {
        let cleanup = cleanup(
            &p,
            parent.as_fd(),
            wrapper.as_ref(),
            root_journal != Journal::Unattempted,
            w.hooks,
        );
        return RestoreOutcome::Unpublished(Unpublished {
            cause,
            cleanup,
            uncertain_outer: None,
        });
    }
    let root_stat = match w.call(Op::Stat, || fs::fstat(p.dirs[0].as_ref().unwrap())) {
        Ok(s) => s,
        Err(cause) => {
            let cleanup = cleanup(
                &p,
                parent.as_fd(),
                wrapper.as_ref(),
                root_journal != Journal::Unattempted,
                w.hooks,
            );
            return RestoreOutcome::Unpublished(Unpublished {
                cause,
                cleanup,
                uncertain_outer: None,
            });
        }
    };
    if let Err(cause) = w.tick() {
        let cleanup = cleanup(
            &p,
            parent.as_fd(),
            wrapper.as_ref(),
            root_journal != Journal::Unattempted,
            w.hooks,
        );
        return RestoreOutcome::Unpublished(Unpublished {
            cause,
            cleanup,
            uncertain_outer: None,
        });
    }
    let mut receipt = Receipt {
        destination: p.destination,
        staging: p.staging,
        profile,
        parent: identity(&stat),
        root: identity(&root_stat),
        filesystem,
        unspecified_empty_root_mtime: p.root.is_none(),
        cleanup: Cleanup::default(),
    };
    // Last cancellation check is above. No post-rename cancellation can undo success.
    let renamed = w
        .hooks
        .before(Op::Publish)
        .and_then(|()| {
            if let Some(wrapper) = &wrapper {
                fs::renameat_with(
                    wrapper,
                    c"root",
                    &parent,
                    receipt.destination.as_c_str(),
                    RenameFlags::NOREPLACE,
                )
            } else {
                fs::renameat_with(
                    &parent,
                    receipt.staging.as_c_str(),
                    &parent,
                    receipt.destination.as_c_str(),
                    RenameFlags::NOREPLACE,
                )
            }
        })
        .and_then(|()| w.hooks.after(Op::Publish));
    match renamed {
        Err(error) => RestoreOutcome::PublicationIndeterminate(Indeterminate { error, receipt }),
        Ok(()) => {
            if wrapper.is_some() {
                let mut left = 1;
                cleanup_call(
                    &mut receipt.cleanup,
                    &mut left,
                    w.hooks,
                    Op::CleanupUnlink,
                    || fs::unlinkat(&parent, receipt.staging.as_c_str(), AtFlags::REMOVEDIR),
                );
            }
            RestoreOutcome::Published(receipt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        manifest::{Manifest, ManifestLimits},
        multipart::{AssemblyLimits, MultipartAssembler},
        names::Envelope,
        restore_plan::{RestorePlan, RestorePlanLimits},
        state::{SnapshotState, StateLimits},
    };
    use sha1::Digest;
    use std::{
        fs as stdfs,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    //= docs/parsing-requirements.md#restore-014
    //= type=test
    //# Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.
    fn write_offset_transition_rejects_zero_and_out_of_range_progress() {
        use crate::verified::restore_write::advance_write_offset;

        assert_eq!(advance_write_offset(0, 1, 1), Some(1));
        assert_eq!(advance_write_offset(2, 3, 10), Some(5));
        assert_eq!(
            advance_write_offset(usize::MAX - 2, 2, usize::MAX),
            Some(usize::MAX)
        );
        assert_eq!(advance_write_offset(0, 0, 1), None);
        assert_eq!(advance_write_offset(2, 1, 1), None);
        assert_eq!(advance_write_offset(4, 2, 5), None);
        assert_eq!(advance_write_offset(1, 4, 4), None);
        assert_eq!(advance_write_offset(5, 1, 5), None);
    }

    struct Temp(PathBuf);
    impl Temp {
        fn new(mode: u32) -> Self {
            let path = std::env::temp_dir().join(format!(
                "duplicity-restore-fault-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            stdfs::create_dir(&path).unwrap();
            stdfs::set_permissions(&path, stdfs::Permissions::from_mode(mode)).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fn repair(path: &Path) {
                if let Ok(meta) = stdfs::symlink_metadata(path) {
                    if meta.is_dir() {
                        stdfs::set_permissions(path, stdfs::Permissions::from_mode(0o700)).unwrap();
                        for entry in stdfs::read_dir(path).unwrap() {
                            repair(&entry.unwrap().path());
                        }
                    }
                }
            }
            repair(&self.0);
            stdfs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn state(root_mode: u32, dir_mode: u32, link_mode: u32) -> SnapshotState {
        state_time(root_mode, dir_mode, link_mode, None)
    }
    fn state_time(
        root_mode: u32,
        dir_mode: u32,
        link_mode: u32,
        time: Option<&str>,
    ) -> SnapshotState {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, kind, mode, content, target) in [
            (
                "snapshot/.",
                tar::EntryType::Directory,
                root_mode,
                b"".as_slice(),
                "",
            ),
            ("snapshot/d", tar::EntryType::Directory, dir_mode, b"", ""),
            (
                "snapshot/d/file",
                tar::EntryType::Regular,
                0o640,
                b"content",
                "",
            ),
            (
                "snapshot/link",
                tar::EntryType::Symlink,
                link_mode,
                b"",
                "../../outside",
            ),
        ] {
            if let Some(time) = time {
                builder
                    .append_pax_extensions([("mtime", time.as_bytes())])
                    .unwrap();
            }
            let mut h = tar::Header::new_ustar();
            h.set_entry_type(kind);
            h.set_mode(mode);
            h.set_mtime(1);
            h.set_size(content.len() as u64);
            if !target.is_empty() {
                h.set_link_name(target).unwrap();
            }
            h.as_mut_bytes()[..100].fill(0);
            h.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
            h.set_cksum();
            builder.append(&h, content).unwrap();
        }
        let bytes = builder.into_inner().unwrap();
        let digest: String = sha1::Sha1::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = Manifest::parse(
            format!("Volume 1:\n StartingPath .\n EndingPath link\n Hash SHA1 {digest}\n")
                .as_bytes(),
            ManifestLimits::default(),
        )
        .unwrap();
        let mut assembly = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
        assembly
            .push_stored_volume(1, &bytes, Envelope::Plain)
            .unwrap();
        SnapshotState::from_full(&assembly.finish().unwrap(), StateLimits::default()).unwrap()
    }
    #[derive(Default)]
    struct Fault {
        before: Option<Op>,
        after: Option<Op>,
        fired: bool,
        seen: Vec<Op>,
        cancel: Option<(Op, Cancellation)>,
        zero: bool,
        short: bool,
        interrupt: bool,
        cleanup_error: bool,
    }
    impl Hooks for Fault {
        fn before(&mut self, op: Op) -> Result<(), Errno> {
            self.seen.push(op);
            if self.cleanup_error && op == Op::CleanupUnlink {
                return Err(Errno::IO);
            }
            if let Some((at, cancel)) = &self.cancel {
                if *at == op {
                    cancel.cancel();
                }
            }
            if self.before == Some(op) && !self.fired {
                self.fired = true;
                return Err(Errno::IO);
            }
            Ok(())
        }
        fn after(&mut self, op: Op) -> Result<(), Errno> {
            if self.after == Some(op) && !self.fired {
                self.fired = true;
                return Err(Errno::IO);
            }
            Ok(())
        }
        fn write(&mut self, fd: BorrowedFd<'_>, bytes: &[u8]) -> Result<usize, Errno> {
            if self.zero {
                return Ok(0);
            }
            if self.interrupt {
                self.interrupt = false;
                return Err(Errno::INTR);
            }
            rustix::io::write(
                fd,
                if self.short {
                    &bytes[..bytes.len().min(2)]
                } else {
                    bytes
                },
            )
        }
    }
    fn run(
        state: &SnapshotState,
        temp: &Temp,
        fault: &mut Fault,
        cancel: &Cancellation,
    ) -> RestoreOutcome {
        let plan = RestorePlan::all(state, RestorePlanLimits::default()).unwrap();
        restore_with(
            &plan,
            &temp.0,
            b"dest",
            cancel,
            RestoreLimits::default(),
            fault,
        )
    }
    #[test]
    //= docs/parsing-requirements.md#restore-013
    //= type=test
    //# The backend MUST pre-reserve a journal state for every attempted creation and preserve possibly-created as well as definitely-created entries through handled failure cleanup.
    //= docs/parsing-requirements.md#restore-022
    //= type=test
    //# On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.
    //= docs/parsing-requirements.md#restore-027
    //= type=test
    //# If creation of the first outer staging leaf returns an error other than a definite existing-entry collision, the backend MUST report its candidate as uncertain and leave that candidate untouched because exclusive ownership was not established.
    fn attempted_descendant_side_effect_errors_cleanup_but_outer_uncertainty_is_untouched() {
        let state = state(0o700, 0o000, 0o777);
        for op in [Op::CreateDirectory, Op::CreateFile, Op::CreateLink] {
            let temp = Temp::new(0o700);
            let mut fault = Fault {
                after: Some(op),
                ..Fault::default()
            };
            let result = run(&state, &temp, &mut fault, &Cancellation::default());
            assert!(
                matches!(result,RestoreOutcome::Unpublished(ref e)if e.cleanup.failures==0&&e.uncertain_outer.is_none())
            );
            assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
        }
        let temp = Temp::new(0o755);
        let mut fault = Fault {
            after: Some(Op::CreateOuter),
            ..Fault::default()
        };
        let result = run(&state, &temp, &mut fault, &Cancellation::default());
        assert!(
            matches!(result,RestoreOutcome::Unpublished(ref e)if e.uncertain_outer.is_some()&&e.cleanup.attempted==0)
        );
        assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 1);
        assert!(!fault.seen.contains(&Op::CleanupMode));
        assert!(!fault.seen.contains(&Op::CleanupUnlink));
    }
    #[test]
    //= docs/parsing-requirements.md#restore-020
    //= type=test
    //# The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.
    //= docs/parsing-requirements.md#restore-026
    //= type=test
    //# If a publication rename is attempted and returns an error, the backend MUST report publication uncertainty and refrain from modifying cleanup of the restore root, its descendants or its staging wrapper.
    fn publication_side_effect_errors_never_run_modifying_cleanup() {
        let state = state(0o777, 0o000, 0o777);
        for mode in [0o700, 0o755] {
            for after in [false, true] {
                let temp = Temp::new(mode);
                let mut fault = if after {
                    Fault {
                        after: Some(Op::Publish),
                        ..Fault::default()
                    }
                } else {
                    Fault {
                        before: Some(Op::Publish),
                        ..Fault::default()
                    }
                };
                let outcome = run(&state, &temp, &mut fault, &Cancellation::default());
                assert!(matches!(
                    outcome,
                    RestoreOutcome::PublicationIndeterminate(_)
                ));
                assert_eq!(temp.0.join("dest").exists(), after);
                let publish = fault.seen.iter().position(|op| *op == Op::Publish).unwrap();
                assert!(!fault.seen[publish..]
                    .iter()
                    .any(|op| matches!(op, Op::CleanupMode | Op::CleanupUnlink)));
            }
        }
    }
    #[test]
    //= docs/parsing-requirements.md#restore-006
    //= type=test
    //# Cancellation observed before a publication attempt or a handled failure before that attempt MUST leave the restored tree unpublished at the requested destination and attempt removal of temporary plaintext from exclusively owned staging.
    //= docs/parsing-requirements.md#restore-021
    //= type=test
    //# Cancellation MUST be observed before each entry, each bounded write attempt and the final publication attempt, with successful publication taking precedence over a racing cancellation request.
    //= docs/parsing-requirements.md#restore-023
    //= type=test
    //# The result MUST distinguish unpublished failure/cancellation, published success, and publication-indeterminate outcomes, retaining post-success wrapper-removal faults as cleanup warnings on published success.
    fn cancellation_rename_race_success_wins_and_wrapper_cleanup_warns() {
        let state = state(0o777, 0o000, 0o777);
        let temp = Temp::new(0o755);
        let cancel = Cancellation::default();
        let mut fault = Fault {
            cancel: Some((Op::Publish, cancel.clone())),
            before: Some(Op::CleanupUnlink),
            ..Fault::default()
        };
        let outcome = run(&state, &temp, &mut fault, &cancel);
        assert!(matches!(outcome,RestoreOutcome::Published(ref r)if r.cleanup.failures==1));
        assert!(cancel.is_cancelled());
        assert!(temp.0.join("dest").is_dir());
        assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 2);
        let temp = Temp::new(0o700);
        let cancel = Cancellation::default();
        let mut fault = Fault {
            cancel: Some((Op::Write, cancel.clone())),
            ..Fault::default()
        };
        assert!(
            matches!(run(&state,&temp,&mut fault,&cancel),RestoreOutcome::Unpublished(ref e)if e.cause==Cause::Cancelled&&e.cleanup.failures==0)
        );
        assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
    }
    #[test]
    //= docs/parsing-requirements.md#restore-014
    //= type=test
    //# Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.
    fn short_interrupted_zero_writes_and_forward_faults_keep_cleanup_reserved() {
        let state = state(0o700, 0o000, 0o777);
        let temp = Temp::new(0o700);
        let mut fault = Fault {
            short: true,
            interrupt: true,
            ..Fault::default()
        };
        assert!(matches!(
            run(&state, &temp, &mut fault, &Cancellation::default()),
            RestoreOutcome::Published(_)
        ));
        assert!(fault.seen.iter().filter(|op| **op == Op::Write).count() >= 5);
        for fault in [
            Fault {
                zero: true,
                ..Fault::default()
            },
            Fault {
                before: Some(Op::SetTime),
                ..Fault::default()
            },
            Fault {
                before: Some(Op::AclRemove),
                ..Fault::default()
            },
            Fault {
                before: Some(Op::ReadLink),
                ..Fault::default()
            },
        ] {
            let temp = Temp::new(0o700);
            let mut fault = fault;
            assert!(
                matches!(run(&state,&temp,&mut fault,&Cancellation::default()),RestoreOutcome::Unpublished(ref e)if e.cleanup.failures==0)
            );
            assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
        }
        let temp = Temp::new(0o700);
        let plan = RestorePlan::all(&state, RestorePlanLimits::default()).unwrap();
        let cancel = Cancellation::default();
        let mut fault = Fault::default();
        let result = restore_with(
            &plan,
            &temp.0,
            b"dest",
            &cancel,
            RestoreLimits {
                forward_work: 40,
                ..RestoreLimits::default()
            },
            &mut fault,
        );
        assert!(
            matches!(result,RestoreOutcome::Unpublished(ref e)if e.cause==Cause::Limit&&e.cleanup.failures==0)
        );
        assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
    }
    #[test]
    //= docs/parsing-requirements.md#restore-011
    //= type=test
    //# The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.
    //= docs/parsing-requirements.md#restore-018
    //= type=test
    //# Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.
    //= docs/parsing-requirements.md#restore-025
    //= type=test
    //# The backend MUST select and validate its publication profile before creating staging, without changing destination-parent permissions to qualify for either profile.
    fn root_profiles_restrictive_descendants_and_preflight_mode_checks() {
        for root in [0o000, 0o555, 0o777] {
            let state = state(root, 0o000, 0o777);
            let temp = Temp::new(0o700);
            let result = run(
                &state,
                &temp,
                &mut Fault::default(),
                &Cancellation::default(),
            );
            assert!(
                matches!(result,RestoreOutcome::Published(ref r)if r.profile==Profile::PrivateParentSameParent)
            );
            assert_eq!(
                stdfs::metadata(temp.0.join("dest")).unwrap().mode() & 0o777,
                root
            );
            let temp = Temp::new(0o755);
            let result = run(
                &state,
                &temp,
                &mut Fault::default(),
                &Cancellation::default(),
            );
            if root & 0o200 == 0 {
                assert!(
                    matches!(result,RestoreOutcome::Unpublished(ref e)if e.cause==Cause::InvalidPlan)
                );
                assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
            } else {
                assert!(matches!(result, RestoreOutcome::Published(_)));
            }
        }
        let invalid = state(0o700, 0o700, 0o700);
        let temp = Temp::new(0o700);
        assert!(
            matches!(run(&invalid,&temp,&mut Fault::default(),&Cancellation::default()),RestoreOutcome::Unpublished(ref e)if e.cause==Cause::InvalidPlan)
        );
        assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
    }
    #[test]
    //= docs/parsing-requirements.md#restore-016
    //= type=test
    //# The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.
    fn inherited_acl_is_removed_before_widened_final_permissions() {
        let temp = Temp::new(0o700);
        let parent = fs::open(&temp.0, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty()).unwrap();
        let mut acl = 2u32.to_le_bytes().to_vec();
        for (tag, permission, id) in [
            (1u16, 7u16, u32::MAX),
            (2, 7, rustix::process::geteuid().as_raw() + 1),
            (4, 7, u32::MAX),
            (16, 7, u32::MAX),
            (32, 7, u32::MAX),
        ] {
            acl.extend_from_slice(&tag.to_le_bytes());
            acl.extend_from_slice(&permission.to_le_bytes());
            acl.extend_from_slice(&id.to_le_bytes());
        }
        fs::fsetxattr(
            &parent,
            c"system.posix_acl_default",
            &acl,
            fs::XattrFlags::empty(),
        )
        .unwrap();
        let snapshot = state(0o777, 0o777, 0o777);
        assert!(matches!(
            run(
                &snapshot,
                &temp,
                &mut Fault::default(),
                &Cancellation::default()
            ),
            RestoreOutcome::Published(_)
        ));
        for name in ["dest", "dest/d", "dest/d/file"] {
            let fd = fs::open(
                temp.0.join(name),
                OFlags::RDONLY | OFlags::NOFOLLOW,
                Mode::empty(),
            )
            .unwrap();
            for attribute in [c"system.posix_acl_access", c"system.posix_acl_default"] {
                let mut buffer = [0u8; 1];
                assert_eq!(
                    fs::fgetxattr(&fd, attribute, &mut buffer[..]).unwrap_err(),
                    Errno::NODATA
                );
            }
        }
    }
    #[test]
    //= docs/parsing-requirements.md#restore-017
    //= type=test
    //# Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.
    fn fractional_negative_time_is_exact_or_fails_before_publication() {
        for time in ["-1.123456789", "1.000000001", "253402300799.999999999"] {
            let snapshot = state_time(0o700, 0o000, 0o777, Some(time));
            let temp = Temp::new(0o700);
            let result = run(
                &snapshot,
                &temp,
                &mut Fault::default(),
                &Cancellation::default(),
            );
            match result {
                RestoreOutcome::Published(_) => {
                    let metadata = stdfs::symlink_metadata(temp.0.join("dest/link")).unwrap();
                    let entry = snapshot
                        .entries()
                        .find(|(path, _)| path.components().as_slice() == [b"link".to_vec()])
                        .unwrap()
                        .1;
                    assert_eq!(metadata.mtime(), entry.mtime().seconds);
                    assert_eq!(metadata.mtime_nsec(), i64::from(entry.mtime().nanoseconds));
                }
                RestoreOutcome::Unpublished(error) => {
                    assert_eq!(error.cause, Cause::Fidelity);
                    assert_eq!(error.cleanup.failures, 0);
                    assert_eq!(stdfs::read_dir(&temp.0).unwrap().count(), 0);
                }
                other => panic!("{other:?}"),
            }
        }
    }
    #[test]
    //= docs/parsing-requirements.md#restore-012
    //= type=test
    //# The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.
    fn exact_creation_fd_payload_cleanup_and_metadata_edges() {
        let snapshot = state(0o700, 0o700, 0o777);
        let plan = RestorePlan::all(&snapshot, RestorePlanLimits::default()).unwrap();
        let cancel = Cancellation::default();
        for mode in [0o700, 0o755] {
            let temp = Temp::new(mode);
            let overhead = if mode == 0o700 { 1 } else { 2 };
            let limits = RestoreLimits {
                entries: 3 + overhead,
                directory_fds: 2 + overhead + 1,
                payload_bytes: 7,
                cleanup_work: (3 + overhead) * 2,
                ..RestoreLimits::default()
            };
            assert!(matches!(
                restore(&plan, &temp.0, b"dest", &cancel, limits),
                RestoreOutcome::Published(_)
            ));
            for lowered in [
                RestoreLimits {
                    entries: limits.entries - 1,
                    ..limits
                },
                RestoreLimits {
                    directory_fds: limits.directory_fds - 1,
                    ..limits
                },
                RestoreLimits {
                    payload_bytes: 6,
                    ..limits
                },
                RestoreLimits {
                    cleanup_work: limits.cleanup_work - 1,
                    ..limits
                },
            ] {
                let result = restore(&plan, &temp.0, b"other", &cancel, lowered);
                assert!(
                    matches!(result,RestoreOutcome::Unpublished(ref error)if error.cause==Cause::Limit)
                );
                assert!(!temp.0.join("other").exists());
            }
        }
        let temp = Temp::new(0o700);
        for metadata in [false, true] {
            let mut lo = 0;
            let mut hi = 4096;
            while lo < hi {
                let mid = (lo + hi) / 2;
                let limits = if metadata {
                    RestoreLimits {
                        metadata_bytes: mid,
                        ..RestoreLimits::default()
                    }
                } else {
                    RestoreLimits {
                        name_bytes: mid,
                        ..RestoreLimits::default()
                    }
                };
                let mut hooks = Native;
                let mut forward = Forward {
                    left: limits.forward_work,
                    cancel: &cancel,
                    hooks: &mut hooks,
                };
                if prepare(
                    &plan,
                    b"dest",
                    temp.0.as_os_str().as_encoded_bytes().len() + 1,
                    Profile::PrivateParentSameParent,
                    limits,
                    &mut forward,
                )
                .is_ok()
                {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            for (budget, ok) in [(lo - 1, false), (lo, true)] {
                let limits = if metadata {
                    RestoreLimits {
                        metadata_bytes: budget,
                        ..RestoreLimits::default()
                    }
                } else {
                    RestoreLimits {
                        name_bytes: budget,
                        ..RestoreLimits::default()
                    }
                };
                let mut hooks = Native;
                let mut forward = Forward {
                    left: limits.forward_work,
                    cancel: &cancel,
                    hooks: &mut hooks,
                };
                assert_eq!(
                    prepare(
                        &plan,
                        b"dest",
                        temp.0.as_os_str().as_encoded_bytes().len() + 1,
                        Profile::PrivateParentSameParent,
                        limits,
                        &mut forward
                    )
                    .is_ok(),
                    ok
                );
            }
        }
        let mut fault = Fault::default();
        let result = restore_with(
            &plan,
            &temp.0,
            b"dest",
            &cancel,
            RestoreLimits {
                directory_fds: 0,
                ..RestoreLimits::default()
            },
            &mut fault,
        );
        assert!(matches!(result, RestoreOutcome::Unpublished(_)));
        assert!(fault.seen.is_empty());
    }
    #[test]
    fn cleanup_failures_count_and_continue_without_hiding_original_error() {
        let snapshot = state(0o700, 0o700, 0o777);
        let temp = Temp::new(0o700);
        let mut fault = Fault {
            before: Some(Op::SetTime),
            cleanup_error: true,
            ..Fault::default()
        };
        let result = run(&snapshot, &temp, &mut fault, &Cancellation::default());
        assert!(
            matches!(result,RestoreOutcome::Unpublished(ref error)if error.cause==Cause::Syscall(Errno::IO)&&error.cleanup.failures>=3&&error.cleanup.diagnostics_truncated)
        );
        assert!(
            fault
                .seen
                .iter()
                .filter(|op| **op == Op::CleanupUnlink)
                .count()
                >= 3
        );
        assert!(!temp.0.join("dest").exists());
    }
    #[test]
    //= docs/parsing-requirements.md#restore-019
    //= type=test
    //# When a genuinely empty complete plan has no source-root metadata, the backend MUST publish an empty root with explicit policy mode `0700` without reporting an invented source-root mtime.
    fn genuinely_empty_tree_has_explicit_policy_root_without_invented_mtime() {
        let digest: String = sha1::Sha1::digest([])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest = Manifest::parse(
            format!("Volume 1:\n StartingPath .\n EndingPath .\n Hash SHA1 {digest}\n").as_bytes(),
            ManifestLimits::default(),
        )
        .unwrap();
        let mut assembly = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
        assembly
            .push_stored_volume(1, &[], Envelope::Plain)
            .unwrap();
        let snapshot =
            SnapshotState::from_full(&assembly.finish().unwrap(), StateLimits::default()).unwrap();
        assert!(snapshot.is_empty());
        for mode in [0o700, 0o755] {
            let temp = Temp::new(mode);
            assert!(
                matches!(run(&snapshot,&temp,&mut Fault::default(),&Cancellation::default()),RestoreOutcome::Published(ref r)if r.unspecified_empty_root_mtime)
            );
            assert_eq!(
                stdfs::metadata(temp.0.join("dest")).unwrap().mode() & 0o777,
                0o700
            );
            assert_eq!(stdfs::read_dir(temp.0.join("dest")).unwrap().count(), 0);
        }
    }
    #[test]
    //= docs/parsing-requirements.md#restore-002
    //= type=test
    //# A restoration MUST write into private staging on the destination filesystem and publish the completed tree without replacing an existing destination.
    //= docs/parsing-requirements.md#restore-009
    //= type=test
    //# The backend MUST keep every provisional restored entry behind its selected profile's private directory boundary until publication or cleanup.
    //= docs/parsing-requirements.md#restore-010
    //= type=test
    //# All operations on backup entries MUST use their original byte-valued single path components relative to retained directory FDs, without reopening a directory by a joined path or following a restored symlink.
    fn concurrent_destination_and_parent_path_changes_preserve_anchored_semantics() {
        struct Namespace {
            parent: PathBuf,
            replace_parent: bool,
            fired: bool,
        }
        impl Hooks for Namespace {
            fn before(&mut self, op: Op) -> Result<(), Errno> {
                if self.replace_parent && op == Op::CreateOuter && !self.fired {
                    self.fired = true;
                    stdfs::rename(&self.parent, self.parent.with_file_name("old-parent")).unwrap();
                    stdfs::create_dir(&self.parent).unwrap();
                    stdfs::set_permissions(&self.parent, stdfs::Permissions::from_mode(0o700))
                        .unwrap();
                }
                if !self.replace_parent && op == Op::Publish {
                    stdfs::create_dir(self.parent.join("dest")).unwrap();
                    let stage = stdfs::read_dir(&self.parent)
                        .unwrap()
                        .map(|e| e.unwrap().path())
                        .find(|p| {
                            p.file_name()
                                .unwrap()
                                .as_encoded_bytes()
                                .starts_with(b".duplicity-")
                        })
                        .unwrap();
                    assert_eq!(stdfs::metadata(&stage).unwrap().mode() & 0o777, 0o700);
                    assert_eq!(
                        stdfs::metadata(stage.join("root")).unwrap().mode() & 0o777,
                        0o777
                    );
                }
                Ok(())
            }
        }
        let snapshot = state(0o777, 0o700, 0o777);
        let plan = RestorePlan::all(&snapshot, RestorePlanLimits::default()).unwrap();
        let temp = Temp::new(0o700);
        let parent = temp.0.join("parent");
        stdfs::create_dir(&parent).unwrap();
        stdfs::set_permissions(&parent, stdfs::Permissions::from_mode(0o700)).unwrap();
        let original_inode = stdfs::metadata(&parent).unwrap().ino();
        let mut hook = Namespace {
            parent: parent.clone(),
            replace_parent: true,
            fired: false,
        };
        let outcome = restore_with(
            &plan,
            &parent,
            b"dest",
            &Cancellation::default(),
            RestoreLimits::default(),
            &mut hook,
        );
        assert!(
            matches!(outcome,RestoreOutcome::Published(ref receipt)if receipt.parent.inode==original_inode)
        );
        assert!(temp.0.join("old-parent/dest/d/file").exists());
        assert!(!parent.join("dest").exists());
        let temp = Temp::new(0o755);
        let mut hook = Namespace {
            parent: temp.0.clone(),
            replace_parent: false,
            fired: false,
        };
        let outcome = restore_with(
            &plan,
            &temp.0,
            b"dest",
            &Cancellation::default(),
            RestoreLimits::default(),
            &mut hook,
        );
        assert!(matches!(
            outcome,
            RestoreOutcome::PublicationIndeterminate(_)
        ));
        assert_eq!(stdfs::read_dir(temp.0.join("dest")).unwrap().count(), 0);
    }
}
