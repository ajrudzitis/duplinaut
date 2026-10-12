//! Provisional, bounded in-memory snapshot reconstruction.
//!
//! Only completed assembled archives enter this controller. Chain continuity,
//! manifest provenance, source identity and eventual filesystem publication are
//! separate caller obligations. Controller refinement is pending; immutable Arc
//! ownership, standard containers/allocators remain trusted infrastructure.
use crate::delta::{apply_delta, DeltaError};
use crate::multipart::{AssembledArchive, LogicalEntry, LogicalOperation};
use crate::path::BackupPath;
use crate::tar::{EntryType, TarTime};
use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::Arc;
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug)]
pub struct StateLimits {
    /// Applies to each provisional and resulting snapshot, including unchanged data.
    pub payload_bytes: usize,
    pub metadata_bytes: usize,
    pub entries: usize,
    pub file_bytes: usize,
    /// Conservative transition budget: preceding state + caller archive +
    /// provisional state + current reconstruction temporary. Shared allocations
    /// may be counted twice. Other caller-held states/allocations are excluded.
    pub live_payload_bytes: usize,
    pub live_metadata_bytes: usize,
}
impl Default for StateLimits {
    fn default() -> Self {
        Self {
            payload_bytes: 8 * 1024 * 1024 * 1024,
            metadata_bytes: 256 * 1024 * 1024,
            entries: 10_000_000,
            file_bytes: 8 * 1024 * 1024 * 1024,
            live_payload_bytes: 32 * 1024 * 1024 * 1024,
            live_metadata_bytes: 1024 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    Limit,
    Allocation,
    FullOperation,
    MissingBasis,
    IncompatibleBasis,
    EntryPayload,
    RootType,
    Ancestor,
    Delta(DeltaError),
}
}

#[derive(Debug)]
pub struct SnapshotEntry {
    entry_type: EntryType,
    mode: u32,
    uid: u64,
    gid: u64,
    mtime: TarTime,
    atime: Option<TarTime>,
    ctime: Option<TarTime>,
    linkname: Vec<u8>,
    uname: Vec<u8>,
    gname: Vec<u8>,
    device_major: u64,
    device_minor: u64,
    content: Option<Arc<Vec<u8>>>,
    metadata_charge: usize,
}
impl SnapshotEntry {
    pub fn entry_type(&self) -> EntryType {
        self.entry_type
    }
    pub fn mode(&self) -> u32 {
        self.mode
    }
    pub fn uid(&self) -> u64 {
        self.uid
    }
    pub fn gid(&self) -> u64 {
        self.gid
    }
    pub fn mtime(&self) -> TarTime {
        self.mtime
    }
    pub fn atime(&self) -> Option<TarTime> {
        self.atime
    }
    pub fn ctime(&self) -> Option<TarTime> {
        self.ctime
    }
    pub fn linkname(&self) -> &[u8] {
        &self.linkname
    }
    pub fn uname(&self) -> &[u8] {
        &self.uname
    }
    pub fn gname(&self) -> &[u8] {
        &self.gname
    }
    pub fn device_major(&self) -> u64 {
        self.device_major
    }
    pub fn device_minor(&self) -> u64 {
        self.device_minor
    }
    pub fn content(&self) -> Option<&[u8]> {
        self.content.as_ref().map(|bytes| bytes.as_slice())
    }
    /// Metadata for unsupported filesystem types is retained for browsing;
    /// a later restoration selection must reject these types before publication.
    pub fn supported_restore_type(&self) -> bool {
        matches!(
            self.entry_type,
            EntryType::Regular | EntryType::Directory | EntryType::Symlink
        )
    }
}
#[derive(Debug)]
pub struct SnapshotState {
    entries: BTreeMap<BackupPath, Arc<SnapshotEntry>>,
    payload_bytes: usize,
    capacity_bytes: usize,
    metadata_bytes: usize,
}
impl SnapshotState {
    pub fn entries(&self) -> impl Iterator<Item = (&BackupPath, &SnapshotEntry)> {
        self.entries
            .iter()
            .map(|(path, entry)| (path, entry.as_ref()))
    }
    pub fn get(&self, path: &BackupPath) -> Option<&SnapshotEntry> {
        self.entries.get(path).map(|entry| entry.as_ref())
    }
    pub fn payload_bytes(&self) -> usize {
        self.payload_bytes
    }
    pub fn metadata_bytes(&self) -> usize {
        self.metadata_bytes
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn from_full(archive: &AssembledArchive, limits: StateLimits) -> Result<Self, StateError> {
        let previous = Self {
            entries: BTreeMap::new(),
            payload_bytes: 0,
            capacity_bytes: 0,
            metadata_bytes: std::mem::size_of::<Self>(),
        };
        previous.apply(archive, limits, true)
    }
    pub fn apply_incremental(
        &self,
        archive: &AssembledArchive,
        limits: StateLimits,
    ) -> Result<Self, StateError> {
        self.apply(archive, limits, false)
    }

    //= docs/parsing-requirements.md#parse-state-001
    //= type=implementation
    //# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
    fn apply(
        &self,
        archive: &AssembledArchive,
        limits: StateLimits,
        full: bool,
    ) -> Result<Self, StateError> {
        let (archive_payload, archive_metadata) = archive_charge(archive)?;
        let fixed_payload = self
            .capacity_bytes
            .checked_add(archive_payload)
            .ok_or(StateError::Limit)?;
        let path_scratch = archive
            .entries()
            .iter()
            .map(|entry| path_charge(&entry.path))
            .chain(self.entries.keys().map(path_charge))
            .try_fold(0usize, |maximum, bytes| {
                bytes.map(|bytes| maximum.max(bytes))
            })?;
        let fixed_metadata = self
            .metadata_bytes
            .checked_add(archive_metadata)
            .and_then(|bytes| bytes.checked_add(path_scratch))
            .ok_or(StateError::Limit)?;
        // Check before cloning the provisional map, even though shared content
        // and metadata are conservatively counted more than once.
        let mut next = Self {
            entries: BTreeMap::new(),
            payload_bytes: self.payload_bytes,
            capacity_bytes: self.capacity_bytes,
            metadata_bytes: self.metadata_bytes,
        };
        check_budget(&next, limits, fixed_payload, fixed_metadata, 0, 0, 0)?;
        if self.entries.len() > limits.entries {
            return Err(StateError::Limit);
        }
        for (path, entry) in &self.entries {
            if entry
                .content
                .as_ref()
                .is_some_and(|bytes| bytes.len() > limits.file_bytes)
            {
                return Err(StateError::Limit);
            }
            next.entries.insert(path.clone(), Arc::clone(entry));
        }
        for entry in archive.entries() {
            if full && entry.operation != LogicalOperation::Snapshot {
                return Err(StateError::FullOperation);
            }
            match entry.operation {
                LogicalOperation::Deleted => {
                    if !entry.payload.is_empty() {
                        return Err(StateError::EntryPayload);
                    }
                    next.remove(&entry.path, true)?;
                }
                LogicalOperation::Snapshot | LogicalOperation::Diff => {
                    if entry.entry_type != EntryType::Regular && !entry.payload.is_empty() {
                        return Err(StateError::EntryPayload);
                    }
                    // Validate immutable preceding-SET basis before changing the
                    // provisional map. A prior operation in this archive cannot
                    // become the basis for a subsequent diff.
                    let basis = if entry.operation == LogicalOperation::Diff {
                        if entry.entry_type != EntryType::Regular {
                            return Err(StateError::IncompatibleBasis);
                        }
                        let basis = self
                            .entries
                            .get(&entry.path)
                            .ok_or(StateError::MissingBasis)?;
                        if basis.entry_type != EntryType::Regular {
                            return Err(StateError::IncompatibleBasis);
                        }
                        Some(
                            basis
                                .content
                                .as_ref()
                                .ok_or(StateError::IncompatibleBasis)?
                                .as_slice(),
                        )
                    } else {
                        None
                    };
                    next.remove(&entry.path, entry.entry_type != EntryType::Directory)?;
                    if next.entries.len() >= limits.entries {
                        return Err(StateError::Limit);
                    }
                    let metadata = entry_charge(entry)?;
                    check_budget(&next, limits, fixed_payload, fixed_metadata, 0, metadata, 0)?;
                    let allowed = allowed_file_bytes(
                        next.payload_bytes,
                        next.capacity_bytes,
                        fixed_payload,
                        limits,
                        basis.is_some(),
                    )?;
                    let content = if entry.entry_type == EntryType::Regular {
                        let bytes = if let Some(basis) = basis {
                            //= docs/parsing-requirements.md#parse-state-002
                            //= type=implementation
                            //# A delta entry MUST use the reconstructed regular-file content for that same path from the preceding state as its basis.
                            apply_delta(&entry.payload, basis, allowed).map_err(
                                |error| match error {
                                    DeltaError::OutputLimit => StateError::Limit,
                                    other => StateError::Delta(other),
                                },
                            )?
                        } else {
                            if entry.payload.len() > allowed {
                                return Err(StateError::Limit);
                            }
                            let mut bytes = Vec::new();
                            bytes
                                .try_reserve_exact(entry.payload.len())
                                .map_err(|_| StateError::Allocation)?;
                            bytes.extend_from_slice(&entry.payload);
                            bytes
                        };
                        Some(Arc::new(bytes))
                    } else {
                        None
                    };
                    let content_len = content.as_ref().map_or(0, |bytes| bytes.len());
                    let content_capacity = content.as_ref().map_or(0, |bytes| bytes.capacity());
                    check_budget(
                        &next,
                        limits,
                        fixed_payload,
                        fixed_metadata,
                        content_len,
                        metadata,
                        content_capacity,
                    )?;
                    let owned = SnapshotEntry {
                        entry_type: entry.entry_type,
                        mode: entry.mode,
                        uid: entry.uid,
                        gid: entry.gid,
                        mtime: entry.mtime,
                        atime: entry.atime,
                        ctime: entry.ctime,
                        linkname: entry.linkname.clone(),
                        uname: entry.uname.clone(),
                        gname: entry.gname.clone(),
                        device_major: entry.device_major,
                        device_minor: entry.device_minor,
                        content,
                        metadata_charge: metadata,
                    };
                    next.payload_bytes = next
                        .payload_bytes
                        .checked_add(content_len)
                        .ok_or(StateError::Limit)?;
                    next.capacity_bytes = next
                        .capacity_bytes
                        .checked_add(content_capacity)
                        .ok_or(StateError::Limit)?;
                    next.metadata_bytes = next
                        .metadata_bytes
                        .checked_add(metadata)
                        .ok_or(StateError::Limit)?;
                    next.entries.insert(entry.path.clone(), Arc::new(owned));
                }
            }
        }
        // Applying the whole set first permits type replacement followed by
        // emitted descendant deletions. Empty state has no invented root metadata.
        let depth = next
            .entries
            .keys()
            .map(|path| path.components().len())
            .max();
        let stack_len = depth.map_or(0, |depth| depth.saturating_add(1));
        let stack_bytes = stack_len
            .checked_mul(std::mem::size_of::<&BackupPath>())
            .ok_or(StateError::Limit)?;
        check_budget(
            &next,
            limits,
            fixed_payload,
            fixed_metadata,
            0,
            stack_bytes,
            0,
        )?;
        let mut ancestors: Vec<&BackupPath> = Vec::new();
        ancestors
            .try_reserve_exact(stack_len)
            .map_err(|_| StateError::Allocation)?;
        for (path, entry) in &next.entries {
            if path.components().is_empty() && entry.entry_type != EntryType::Directory {
                return Err(StateError::RootType);
            }
            while ancestors.last().is_some_and(|parent| {
                parent.components().len() >= path.components().len()
                    || !path.components().starts_with(parent.components())
            }) {
                ancestors.pop();
            }
            if ancestors.len() != path.components().len() {
                return Err(StateError::Ancestor);
            }
            if entry.entry_type == EntryType::Directory {
                ancestors.push(path);
            }
        }
        Ok(next)
    }
    fn remove(&mut self, path: &BackupPath, descendants: bool) -> Result<(), StateError> {
        if !descendants {
            if let Some(entry) = self.entries.remove(path) {
                self.remove_charge(&entry)?;
            }
            return Ok(());
        }
        loop {
            // Component lexicographic order makes a subtree contiguous. One
            // transient copied key was charged in the transition's fixed budget.
            let key = match self
                .entries
                .range((Bound::Included(path), Bound::Unbounded))
                .next()
            {
                Some((candidate, _)) if candidate.components().starts_with(path.components()) => {
                    candidate.clone()
                }
                _ => break,
            };
            let entry = self.entries.remove(&key).ok_or(StateError::Ancestor)?;
            self.remove_charge(&entry)?;
        }
        Ok(())
    }
    fn remove_charge(&mut self, entry: &SnapshotEntry) -> Result<(), StateError> {
        let payload = entry.content.as_ref().map_or(0, |bytes| bytes.len());
        let capacity = entry.content.as_ref().map_or(0, |bytes| bytes.capacity());
        let metadata = entry.metadata_charge;
        self.payload_bytes = self
            .payload_bytes
            .checked_sub(payload)
            .ok_or(StateError::Limit)?;
        self.metadata_bytes = self
            .metadata_bytes
            .checked_sub(metadata)
            .ok_or(StateError::Limit)?;
        self.capacity_bytes = self
            .capacity_bytes
            .checked_sub(capacity)
            .ok_or(StateError::Limit)?;
        Ok(())
    }
}
fn check_budget(
    state: &SnapshotState,
    limits: StateLimits,
    fixed_payload: usize,
    fixed_metadata: usize,
    new_payload: usize,
    new_metadata: usize,
    new_capacity: usize,
) -> Result<(), StateError> {
    check_counts(
        Charges {
            payload: state.payload_bytes,
            metadata: state.metadata_bytes,
            capacity: state.capacity_bytes,
        },
        limits,
        fixed_payload,
        fixed_metadata,
        new_payload,
        new_metadata,
        new_capacity,
    )
}

verus! {
/// Scalar projection of the same private native accounting counters. This is
/// neither a container invariant nor an allocator-capacity specification.
struct Charges {
    payload: usize,
    metadata: usize,
    capacity: usize,
}

/// Mathematical acceptance criterion: natural-number totals must both fit the
/// machine word and satisfy their independent logical/live resource ceilings.
spec fn budget_fits(state: Charges, limits: StateLimits, fixed_payload: usize,
    fixed_metadata: usize, new_payload: usize, new_metadata: usize, new_capacity: usize)
    -> bool
{
    let payload = state.payload as nat + new_payload as nat;
    let metadata = state.metadata as nat + new_metadata as nat;
    let live_payload = fixed_payload as nat + state.capacity as nat + new_capacity as nat;
    let live_metadata = fixed_metadata as nat + metadata;
    payload <= usize::MAX && payload <= limits.payload_bytes
        && metadata <= usize::MAX && metadata <= limits.metadata_bytes
        && live_payload <= usize::MAX && live_payload <= limits.live_payload_bytes
        && live_metadata <= usize::MAX && live_metadata <= limits.live_metadata_bytes
}

//= docs/parsing-requirements.md#parse-gen-003
//= type=implementation
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
fn check_counts(state: Charges, limits: StateLimits, fixed_payload: usize,
    fixed_metadata: usize, new_payload: usize, new_metadata: usize, new_capacity: usize)
    -> (result: Result<(), StateError>)
    ensures result == if budget_fits(state, limits, fixed_payload, fixed_metadata,
        new_payload, new_metadata, new_capacity) { Ok(()) } else { Err(StateError::Limit) },
{
    let payload = match state.payload.checked_add(new_payload) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    let metadata = match state.metadata.checked_add(new_metadata) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    if payload > limits.payload_bytes || metadata > limits.metadata_bytes {
        return Err(StateError::Limit);
    }
    let retained_live_payload = match fixed_payload.checked_add(state.capacity) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    let live_payload = match retained_live_payload.checked_add(new_capacity) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    if live_payload > limits.live_payload_bytes { return Err(StateError::Limit); }
    let live_metadata = match fixed_metadata.checked_add(metadata) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    if live_metadata > limits.live_metadata_bytes { return Err(StateError::Limit); }
    Ok(())
}

spec fn smaller(left: nat, right: nat) -> nat { if left < right { left } else { right } }

/// Independent natural-number allowance, including the conservative nonempty
/// byte-vector reallocation ceiling used for delta reconstruction.
spec fn allowance_model(payload: usize, capacity: usize, fixed_payload: usize,
    limits: StateLimits, delta: bool) -> Result<nat, StateError>
{
    let held = fixed_payload as nat + capacity as nat;
    if held > usize::MAX || held > limits.live_payload_bytes || payload > limits.payload_bytes {
        Err(StateError::Limit)
    } else {
        let remaining = (limits.live_payload_bytes as nat - held) as nat;
        let allocation = if delta {
            if remaining < 8 { 0nat } else { remaining / 3 }
        } else { remaining };
        Ok(smaller(smaller(limits.file_bytes as nat,
            (limits.payload_bytes as nat - payload as nat) as nat), allocation))
    }
}

//= docs/parsing-requirements.md#parse-gen-003
//= type=implementation
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
//= docs/parsing-requirements.md#parse-gen-005
//= type=implementation
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
fn allowed_file_bytes(payload: usize, capacity: usize, fixed_payload: usize,
    limits: StateLimits, delta: bool) -> (result: Result<usize, StateError>)
    ensures
        match result {
            Ok(allowed) => allowance_model(payload, capacity, fixed_payload, limits, delta)
                == Ok(allowed as nat)
                && allowed <= limits.file_bytes
                && payload as nat + allowed as nat <= limits.payload_bytes
                && fixed_payload as nat + capacity as nat <= limits.live_payload_bytes
                && (!delta ==> fixed_payload as nat + capacity as nat + allowed as nat
                    <= limits.live_payload_bytes)
                && (delta && allowed > 0 ==>
                    fixed_payload as nat + capacity as nat + 8 <= limits.live_payload_bytes
                    && fixed_payload as nat + capacity as nat + 3 * (allowed as nat)
                        <= limits.live_payload_bytes),
            Err(e) => allowance_model(payload, capacity, fixed_payload, limits, delta) == Err(e),
        },
{
    let held = match fixed_payload.checked_add(capacity) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    let live_remaining = match limits.live_payload_bytes.checked_sub(held) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    // Native delta output Vec growth can temporarily hold old/new buffers.
    // Pinned byte Vec growth charges max(8,3*bound) for nonempty output.
    let allocation_bound = if delta {
        if live_remaining < 8 { 0 } else { live_remaining / 3 }
    } else { live_remaining };
    let logical_remaining = match limits.payload_bytes.checked_sub(payload) {
        Some(n) => n,
        None => return Err(StateError::Limit),
    };
    let allowed = limits.file_bytes.min(logical_remaining).min(allocation_bound);
    Ok(allowed)
}

}

fn path_charge(path: &BackupPath) -> Result<usize, StateError> {
    let mut bytes = path
        .components()
        .capacity()
        .checked_mul(std::mem::size_of::<Vec<u8>>())
        .ok_or(StateError::Limit)?;
    for component in path.components() {
        bytes = bytes
            .checked_add(component.capacity())
            .ok_or(StateError::Limit)?;
    }
    bytes
        .checked_add(std::mem::size_of::<BackupPath>())
        .ok_or(StateError::Limit)
}
fn entry_charge(entry: &LogicalEntry) -> Result<usize, StateError> {
    // Two whole pinned std1.98 internal BTree nodes per output entry. Model key,
    // Arc value, 12 edges and rounded parent/index/length header explicitly.
    let index = 2
        * (11 * (std::mem::size_of::<BackupPath>() + std::mem::size_of::<Arc<SnapshotEntry>>())
            + 16 * std::mem::size_of::<usize>());
    path_charge(&entry.path)?
        .checked_add(index)
        .and_then(|bytes| {
            bytes.checked_add(
                std::mem::size_of::<SnapshotEntry>()
                    + 4 * std::mem::size_of::<usize>()
                    + std::mem::size_of::<Vec<u8>>(),
            )
        })
        .and_then(|bytes| bytes.checked_add(entry.linkname.len()))
        .and_then(|bytes| bytes.checked_add(entry.uname.len()))
        .and_then(|bytes| bytes.checked_add(entry.gname.len()))
        .ok_or(StateError::Limit)
}
fn archive_charge(archive: &AssembledArchive) -> Result<(usize, usize), StateError> {
    let mut payload = 0usize;
    let mut metadata = archive
        .entries()
        .len()
        .max(4)
        .checked_mul(2 * std::mem::size_of::<LogicalEntry>())
        .ok_or(StateError::Limit)?;
    for entry in archive.entries() {
        payload = payload
            .checked_add(entry.payload.capacity())
            .ok_or(StateError::Limit)?;
        metadata = metadata
            .checked_add(path_charge(&entry.path)?)
            .and_then(|bytes| bytes.checked_add(entry.linkname.capacity()))
            .and_then(|bytes| bytes.checked_add(entry.uname.capacity()))
            .and_then(|bytes| bytes.checked_add(entry.gname.capacity()))
            .ok_or(StateError::Limit)?;
    }
    Ok((payload, metadata))
}
