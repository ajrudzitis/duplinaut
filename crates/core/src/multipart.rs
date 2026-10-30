//! Bounded, fail-closed assembly of the reviewed duplicity 3.0.7 profile.
//!
//! Outputs remain provisional until all normalized manifest volumes pass
//! original-byte integrity, envelope and complete-container validation. Formal
//! refinement of this controller is pending; digest/compression are trusted.
use crate::envelope::{decode_verified_volume, EnvelopeError, EnvelopeLimits};
use crate::manifest::{Boundary, Manifest, VolumeRecord};
use crate::names::Envelope;
use crate::openpgp::{self, Credentials, CryptoLimits};
use crate::path::BackupPath;
use crate::tar::{
    ArchiveKind, EntryType, Operation, TarEntry, TarError, TarLimits, TarTime, TarVolume,
    Termination,
};
use crate::verified::multipart_metadata::reserve_multipart_metadata;
use crate::verified::openpgp_admission::{
    AuthenticatedPermit, OpenPgpAdmission, TarValidatedPermit,
};
use std::collections::BTreeMap;
use vstd::prelude::*;

const CHUNK_BYTES: usize = 65536;

// This observation point exists only in unit-test builds. It can request
// cancellation at the two concrete wrapper boundaries below; it cannot
// provide decoded data or replace any production callback.
#[cfg(test)]
mod openpgp_cancel_test_hook {
    use crate::source::Cancellation;
    use std::cell::Cell;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Checkpoint {
        AfterStoredDigest,
        AfterNativeDecode,
    }

    thread_local! {
        static CANCEL_AT: Cell<Option<Checkpoint>> = const { Cell::new(None) };
        static HITS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) struct Guard;

    impl Guard {
        pub(super) fn cancel_at(checkpoint: Checkpoint) -> Self {
            CANCEL_AT.with(|slot| assert_eq!(slot.replace(Some(checkpoint)), None));
            HITS.with(|hits| hits.set(0));
            Self
        }

        pub(super) fn hits(&self) -> usize {
            HITS.with(Cell::get)
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            CANCEL_AT.with(|slot| slot.set(None));
            HITS.with(|hits| hits.set(0));
        }
    }

    pub(super) fn observe(checkpoint: Checkpoint, cancellation: &Cancellation) {
        if CANCEL_AT.with(Cell::get) == Some(checkpoint) {
            HITS.with(|hits| hits.set(hits.get() + 1));
            cancellation.cancel();
        }
    }
}
// Logical Rust buffer capacities, excluding allocator bookkeeping. The pinned
// std Vec growth profile starts byte buffers at eight and non-byte buffers at
// four elements, then doubles. Path clones are charged the same upper bound.
fn path_capacity_charge(bytes: usize, components: usize) -> Result<usize, AssemblyError> {
    let outer = components
        .checked_mul(2)
        .map(|n| n.max(4))
        .and_then(|n| n.checked_mul(std::mem::size_of::<Vec<u8>>()))
        .ok_or(AssemblyError::Limit)?;
    let inner = components
        .checked_mul(8)
        .and_then(|n| n.checked_add(bytes.checked_mul(2)?))
        .ok_or(AssemblyError::Limit)?;
    outer
        .checked_add(inner)
        .and_then(|n| n.checked_add(std::mem::size_of::<BackupPath>()))
        .ok_or(AssemblyError::Limit)
}
// std 1.98 BTree nodes: 11 keys/values, up to 12 edges, parent/index/length
// header. Round their header/alignment upward to four machine words and charge
// two whole internal nodes per distinct path; the retained tree has fewer nodes
// than keys, with spare allowance for an insertion split. This source-dependent
// capacity model is reviewed infrastructure, not a formal allocator proof.
fn index_capacity_charge() -> usize {
    2 * (11 * (std::mem::size_of::<BackupPath>() + std::mem::size_of::<bool>())
        + 16 * std::mem::size_of::<usize>())
}
verus! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LogicalOperation {
        Snapshot,
        Diff,
        Deleted,
    }
}
#[derive(Debug)]
pub struct LogicalEntry {
    pub operation: LogicalOperation,
    pub path: BackupPath,
    pub entry_type: EntryType,
    pub mode: u32,
    pub uid: u64,
    pub gid: u64,
    pub mtime: TarTime,
    pub atime: Option<TarTime>,
    pub ctime: Option<TarTime>,
    pub linkname: Vec<u8>,
    pub uname: Vec<u8>,
    pub gname: Vec<u8>,
    pub device_major: u64,
    pub device_minor: u64,
    pub payload: Vec<u8>,
}
#[derive(Clone, Copy, Debug)]
pub struct AssemblyLimits {
    pub envelope: EnvelopeLimits,
    pub tar: TarLimits,
    pub max_volumes: usize,
    pub max_entries: usize,
    pub max_payload_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_path_components: usize,
}
impl Default for AssemblyLimits {
    fn default() -> Self {
        Self {
            envelope: EnvelopeLimits::default(),
            tar: TarLimits::default(),
            max_volumes: 1_000_000,
            max_entries: 10_000_000,
            max_payload_bytes: 8 * 1024 * 1024 * 1024,
            max_metadata_bytes: 256 * 1024 * 1024,
            max_path_components: 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblyError {
    Limit,
    Allocation,
    Poisoned,
    VolumeOrder,
    Boundary,
    UnsupportedBoundaryProfile,
    PartOrder,
    Interleaved,
    PartLength,
    Incomplete,
    DuplicateEntry,
    Envelope(EnvelopeError),
    Tar(TarError),
    Crypto(crate::openpgp::CryptoError),
    Cancelled,
}
struct Pending {
    entry: LogicalEntry,
    next_part: u64,
}
/// Successful results establish assembly and adapter checks, not signer identity.
/// The manifest itself must still be tied to the intended backup chain/policy.
pub struct AssembledArchive {
    entries: Vec<LogicalEntry>,
}
impl AssembledArchive {
    pub fn entries(&self) -> &[LogicalEntry] {
        &self.entries
    }
    pub fn into_entries(self) -> Vec<LogicalEntry> {
        self.entries
    }
}
pub struct MultipartAssembler<'a> {
    volumes: Vec<&'a VolumeRecord>,
    /// Saturating raw-record counts (0, 1, or >=2), indexed by volume ordinal.
    raw_occurrences: Vec<u8>,
    limits: AssemblyLimits,
    admission: OpenPgpAdmission,
    pending: Option<Pending>,
    entries: Vec<LogicalEntry>,
    /// Value true denotes a multipart logical start, false an ordinary start.
    started_paths: BTreeMap<BackupPath, bool>,
    previous_boundary: Option<Boundary>,
    payload_bytes: usize,
    metadata_bytes: usize,
    scratch_bytes: usize,
    failed: bool,
}
impl<'a> MultipartAssembler<'a> {
    //= docs/parsing-requirements.md#parse-gen-005
    //= type=implementation
    //# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
    pub fn new(manifest: &'a Manifest, limits: AssemblyLimits) -> Result<Self, AssemblyError> {
        if manifest.records.is_empty() {
            return Err(AssemblyError::Incomplete);
        }
        // Charge original record count before allocating the normalized view;
        // superseded restart records cannot evade the input-volume bound.
        if manifest.records.len() > limits.max_volumes {
            return Err(AssemblyError::Limit);
        }
        let metadata_bytes = reserve_multipart_metadata(
            manifest.records.len(),
            std::mem::size_of::<&VolumeRecord>(),
            std::mem::size_of::<u8>(),
            std::mem::size_of::<Self>(),
            limits.max_metadata_bytes,
        )
        .ok_or(AssemblyError::Limit)?;
        let metadata_bytes = metadata_bytes.bytes;
        let volumes = manifest.volumes();
        if volumes.is_empty() {
            return Err(AssemblyError::Incomplete);
        }
        for (index, record) in volumes.iter().enumerate() {
            if record.number
                != u64::try_from(index)
                    .map_err(|_| AssemblyError::Limit)?
                    .checked_add(1)
                    .ok_or(AssemblyError::Limit)?
            {
                return Err(AssemblyError::VolumeOrder);
            }
        }
        // Capacity was charged before normalization or this allocation. A raw
        // repeated record for this specific volume is required for an exception.
        let mut raw_occurrences = vec![0u8; volumes.len()];
        for record in &manifest.records {
            if let Ok(number) = usize::try_from(record.number) {
                if number > 0 && number <= volumes.len() {
                    raw_occurrences[number - 1] = (raw_occurrences[number - 1] + 1).min(2);
                }
            }
        }
        let normalized_volume_count = volumes.len();
        Ok(Self {
            volumes,
            raw_occurrences,
            limits,
            admission: OpenPgpAdmission::new(normalized_volume_count),
            pending: None,
            entries: Vec::new(),
            started_paths: BTreeMap::new(),
            previous_boundary: None,
            payload_bytes: 0,
            metadata_bytes,
            scratch_bytes: 0,
            failed: false,
        })
    }

    //= docs/parsing-requirements.md#parse-multi-003
    //= type=implementation
    //# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
    //= docs/parsing-requirements.md#parse-man-008
    //= type=implementation
    //# A supported nonempty volume MUST have an EndingPath matching its actual last classified logical-data entry path and pending multipart block state.
    //= docs/parsing-requirements.md#parse-man-009
    //= type=implementation
    //# A reader accepting queued-restart compatibility MUST restrict a nonempty volume's StartingPath exception to the source-derived one-entry root/None or second-entry boundary forms with retained raw restart-rewrite evidence.
    //= docs/parsing-requirements.md#parse-multi-004
    //= type=implementation
    //# Assembly MUST preserve pending logical-entry state across an empty volume while validating its supported empty-boundary marker shape.
    //= docs/parsing-requirements.md#parse-env-017
    //= type=implementation
    //# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
    //= docs/parsing-requirements.md#parse-env-020
    //= type=implementation
    //# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
    pub fn push_stored_volume(
        &mut self,
        number: u64,
        stored: &[u8],
        envelope: Envelope,
    ) -> Result<(), AssemblyError> {
        if self.failed {
            return Err(AssemblyError::Poisoned);
        }
        let result = self.push_checked(number, stored, envelope);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    //= docs/parsing-requirements.md#parse-multi-003
    //= type=implementation
    //# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
    //= docs/parsing-requirements.md#parse-env-017
    //= type=implementation
    //# OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.
    //= docs/parsing-requirements.md#parse-env-020
    //= type=implementation
    //# Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.
    /// Internal authenticated OpenPGP path. The caller cannot provide decoded
    /// plaintext or an authentication receipt; this method checks the manifest
    /// digest and invokes the concrete native adapter itself.
    pub(crate) fn push_openpgp_volume(
        &mut self,
        number: u64,
        stored: &[u8],
        credentials: Credentials<'_>,
        crypto_limits: CryptoLimits,
        cancel: &crate::source::Cancellation,
    ) -> Result<openpgp::DecodedObject, AssemblyError> {
        if self.failed {
            return Err(AssemblyError::Poisoned);
        }
        let position = self
            .admission
            .position()
            .ok_or(AssemblyError::VolumeOrder)?;
        let record = self
            .volumes
            .get(position.ordinal)
            .copied()
            .ok_or(AssemblyError::VolumeOrder)?;
        if number != record.number {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::VolumeOrder);
        }
        if cancel.is_cancelled() {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::Cancelled);
        }
        let final_volume = position.final_volume;
        let digest_permit = match self.admission.verify_stored_digest(
            position,
            stored,
            &record.hashes,
            crypto_limits.stored_bytes,
        ) {
            Ok(permit) => permit,
            Err(error) => {
                self.failed = true;
                self.admission.reject();
                return Err(AssemblyError::Envelope(error));
            }
        };
        #[cfg(test)]
        openpgp_cancel_test_hook::observe(
            openpgp_cancel_test_hook::Checkpoint::AfterStoredDigest,
            cancel,
        );
        if cancel.is_cancelled() {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::Cancelled);
        }
        let (decoded, authenticated_permit) =
            match crate::verified::openpgp_admission::decode_after_digest(
                &mut self.admission,
                credentials,
                crypto_limits,
                digest_permit,
            ) {
                Ok(decoded) => decoded,
                Err(error) => {
                    self.failed = true;
                    self.admission.reject();
                    return Err(AssemblyError::Crypto(error));
                }
            };
        #[cfg(test)]
        openpgp_cancel_test_hook::observe(
            openpgp_cancel_test_hook::Checkpoint::AfterNativeDecode,
            cancel,
        );
        if cancel.is_cancelled() {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::Cancelled);
        }
        let validated = match self.push_openpgp_decoded_checked(
            number,
            &decoded.bytes,
            crate::tar::Termination::Encrypted {
                stored_ciphertext: stored,
                nonfinal: !final_volume,
            },
            true,
            authenticated_permit,
        ) {
            Ok(validated) => validated,
            Err(error) => {
                self.failed = true;
                self.admission.reject();
                return Err(error);
            }
        };
        if cancel.is_cancelled() {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::Cancelled);
        }
        if !self.admission.commit_openpgp(validated) {
            self.failed = true;
            self.admission.reject();
            return Err(AssemblyError::Poisoned);
        }
        Ok(decoded)
    }
    fn push_checked(
        &mut self,
        number: u64,
        stored: &[u8],
        envelope: Envelope,
    ) -> Result<(), AssemblyError> {
        let ordinal = self.admission.ordinal();
        let record = self
            .volumes
            .get(ordinal)
            .copied()
            .ok_or(AssemblyError::VolumeOrder)?;
        if number != record.number {
            return Err(AssemblyError::VolumeOrder);
        }
        if !matches!(envelope, Envelope::Plain | Envelope::Gzip) {
            return Err(AssemblyError::Envelope(EnvelopeError::UnsupportedEnvelope));
        }
        let decoded =
            decode_verified_volume(stored, envelope, &record.hashes, self.limits.envelope)
                .map_err(AssemblyError::Envelope)?;
        // OpenPGP is explicitly rejected by the envelope adapter. Its eventual
        // integration must supply the stricter encrypted footer profile here.
        self.push_decoded_checked(number, &decoded, Termination::Data, false)?;
        self.admission.commit_plain();
        Ok(())
    }
    fn push_decoded_checked(
        &mut self,
        number: u64,
        decoded: &[u8],
        termination: Termination<'_>,
        encrypted: bool,
    ) -> Result<(), AssemblyError> {
        let ordinal = self.admission.ordinal();
        let record = self
            .volumes
            .get(ordinal)
            .copied()
            .ok_or(AssemblyError::VolumeOrder)?;
        if number != record.number {
            return Err(AssemblyError::VolumeOrder);
        }
        let volume =
            TarVolume::parse(decoded, termination, self.limits.tar).map_err(AssemblyError::Tar)?;
        let mut first = None;
        let mut second = None;
        let mut last = None;
        for entry in &volume.entries {
            // Bound parser growth and first/second/last/current boundary clones before
            // classification allocates any path. Counting the operation prefix
            // and part suffix as components deliberately overestimates the path.
            let component_bound = entry
                .name
                .iter()
                .filter(|b| **b == b'/')
                .count()
                .checked_add(1)
                .ok_or(AssemblyError::Limit)?;
            let scratch = path_capacity_charge(entry.name.len(), component_bound)?
                .checked_mul(6)
                .ok_or(AssemblyError::Limit)?;
            self.scratch_bytes = self.scratch_bytes.max(scratch);
            if self
                .metadata_bytes
                .checked_add(self.scratch_bytes)
                .is_none_or(|n| n > self.limits.max_metadata_bytes)
            {
                return Err(AssemblyError::Limit);
            }
            let classified = entry
                .classify(
                    ArchiveKind::Data,
                    self.limits.tar.max_path_bytes,
                    self.limits.max_path_components,
                )
                .map_err(AssemblyError::Tar)?;
            // Only the first lower-bound check may be deferred. Every later
            // entry still lies in the declared interval; every entry remains
            // below its ending boundary. No queued-start decision occurs yet.
            if (first.is_some() && classified.path.compare(&record.start.path) < 0)
                || classified.path.compare(&record.end.path) > 0
            {
                return Err(AssemblyError::Boundary);
            }
            let marker = if classified.part.is_some() && entry.payload.len() == CHUNK_BYTES {
                classified.part
            } else {
                None
            };
            let boundary = Boundary {
                path: classified.path.clone(),
                block: marker,
            };
            if first.is_none() {
                first = Some(boundary.clone());
            } else if second.is_none() {
                second = Some(boundary.clone());
            }
            last = Some(boundary);
            self.accept_entry(
                entry,
                classified.operation,
                classified.path,
                classified.part,
            )?;
        }
        if let Some(first) = first {
            if last.as_ref() != Some(&record.end) {
                return Err(AssemblyError::Boundary);
            }
            if first != record.start {
                let rewritten = self.raw_occurrences[ordinal] >= 2;
                let queued_start = if let Some(second) = second {
                    second == record.start
                } else {
                    record.start.path.components().is_empty() && record.start.block.is_none()
                };
                if !rewritten || !queued_start {
                    return Err(AssemblyError::Boundary);
                }
                if encrypted {
                    return Err(AssemblyError::UnsupportedBoundaryProfile);
                }
                // Initial compatibility evidence covers only entry-aligned
                // raw/gzip EOF. Reviewed normal starts retain normal footers.
                if volume.entry_stream_bytes != decoded.len() {
                    return Err(AssemblyError::UnsupportedBoundaryProfile);
                }
            }
        } else {
            // Pinned producer's no-emission fallback uses root as start and
            // the previous emitted end as end; it does not finish pending parts.
            let root = BackupPath::parse(
                b".",
                false,
                self.limits.tar.max_path_bytes,
                self.limits.max_path_components,
            )
            .map_err(|error| AssemblyError::Tar(TarError::Path(error)))?;
            let expected_end = self.previous_boundary.clone().unwrap_or(Boundary {
                path: root.clone(),
                block: None,
            });
            if record.start
                != (Boundary {
                    path: root,
                    block: None,
                })
                || record.end != expected_end
            {
                return Err(AssemblyError::Boundary);
            }
        }
        match &self.pending {
            Some(pending) => {
                if record.end.path != pending.entry.path
                    || record.end.block != pending.next_part.checked_sub(1)
                {
                    return Err(AssemblyError::Boundary);
                }
            }
            None => {
                if record.end.block.is_some() {
                    return Err(AssemblyError::Boundary);
                }
            }
        }
        self.previous_boundary = Some(record.end.clone());
        self.scratch_bytes = 0;
        Ok(())
    }

    /// Produce the final commit capability only after the same checked path
    /// that parses the encrypted tar and validates all multipart boundaries.
    fn push_openpgp_decoded_checked(
        &mut self,
        number: u64,
        decoded: &[u8],
        termination: Termination<'_>,
        encrypted: bool,
        authenticated: AuthenticatedPermit,
    ) -> Result<TarValidatedPermit, AssemblyError> {
        self.push_decoded_checked(number, decoded, termination, encrypted)?;
        Ok(self.admission.tar_validated(authenticated))
    }
    fn charge_metadata(&mut self, amount: usize) -> Result<(), AssemblyError> {
        self.metadata_bytes = self
            .metadata_bytes
            .checked_add(amount)
            .filter(|n| {
                n.checked_add(self.scratch_bytes)
                    .is_some_and(|total| total <= self.limits.max_metadata_bytes)
            })
            .ok_or(AssemblyError::Limit)?;
        Ok(())
    }
    fn append_payload(
        &mut self,
        entry: &mut LogicalEntry,
        bytes: &[u8],
    ) -> Result<(), AssemblyError> {
        self.payload_bytes = self
            .payload_bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.limits.max_payload_bytes)
            .ok_or(AssemblyError::Limit)?;
        entry
            .payload
            .try_reserve_exact(bytes.len())
            .map_err(|_| AssemblyError::Allocation)?;
        entry.payload.extend_from_slice(bytes);
        Ok(())
    }
    fn make_entry(
        &mut self,
        entry: &TarEntry<'_>,
        operation: LogicalOperation,
        path: BackupPath,
    ) -> Result<LogicalEntry, AssemblyError> {
        if self.entries.len() >= self.limits.max_entries {
            return Err(AssemblyError::Limit);
        }
        let path_bytes = path
            .components()
            .iter()
            .try_fold(0usize, |n, c| n.checked_add(c.len()))
            .ok_or(AssemblyError::Limit)?;
        // Charge four path copies: logical output, index, retained preceding
        // boundary, and an additional conservative overlap allowance. Capacity
        // checks for current first/last clones happen before classification.
        let paths = path_capacity_charge(path_bytes, path.components().len())?
            .checked_mul(4)
            .ok_or(AssemblyError::Limit)?;
        let amount = std::mem::size_of::<LogicalEntry>()
            .checked_mul(2)
            .and_then(|n| n.checked_add(paths))
            .and_then(|n| n.checked_add(index_capacity_charge()))
            .and_then(|n| n.checked_add(entry.linkname.len()))
            .and_then(|n| n.checked_add(entry.uname.len()))
            .and_then(|n| n.checked_add(entry.gname.len()))
            .ok_or(AssemblyError::Limit)?;
        self.charge_metadata(amount)?;
        Ok(LogicalEntry {
            operation,
            path,
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
            payload: Vec::new(),
        })
    }
    fn complete(&mut self, entry: LogicalEntry) -> Result<(), AssemblyError> {
        // Reserve one exact slot to avoid uncontrolled vector capacity growth.
        self.entries
            .try_reserve_exact(1)
            .map_err(|_| AssemblyError::Allocation)?;
        self.entries.push(entry);
        Ok(())
    }
    //= docs/parsing-requirements.md#parse-multi-001
    //= type=implementation
    //# Multipart assembly MUST concatenate payloads for one operation and path in consecutive part-number order starting at 1.
    //= docs/parsing-requirements.md#parse-multi-002
    //= type=implementation
    //# Multipart assembly MUST reject missing, duplicate, reversed, mixed-operation, or interleaved parts of the same logical entry.
    //= docs/parsing-requirements.md#parse-multi-005
    //= type=implementation
    //# Repeated ordinary operations for one path within a backup set MUST retain archive order for snapshot-state interpretation.
    //= docs/parsing-requirements.md#parse-multi-006
    //= type=implementation
    //# Assembly MUST reject an ordinary/multipart representation mixture or a second multipart logical entry for one path within the initial supported backup-set profile.
    fn accept_entry(
        &mut self,
        entry: &TarEntry<'_>,
        operation: Operation,
        path: BackupPath,
        part: Option<u64>,
    ) -> Result<(), AssemblyError> {
        let logical = match operation {
            Operation::Snapshot | Operation::MultipartSnapshot => LogicalOperation::Snapshot,
            Operation::Diff | Operation::MultipartDiff => LogicalOperation::Diff,
            Operation::Deleted => LogicalOperation::Deleted,
            _ => return Err(AssemblyError::Interleaved),
        };
        if let Some(number) = part {
            if entry.payload.len() > CHUNK_BYTES {
                return Err(AssemblyError::PartLength);
            }
            let mut pending = if let Some(pending) = self.pending.take() {
                if path != pending.entry.path || logical != pending.entry.operation {
                    return Err(AssemblyError::Interleaved);
                }
                if number != pending.next_part {
                    return Err(AssemblyError::PartOrder);
                }
                pending
            } else {
                if number != 1 {
                    return Err(AssemblyError::PartOrder);
                }
                if self.started_paths.contains_key(&path) {
                    return Err(AssemblyError::DuplicateEntry);
                }
                let owned = self.make_entry(entry, logical, path.clone())?;
                self.started_paths.insert(path, true);
                Pending {
                    entry: owned,
                    next_part: 1,
                }
            };
            self.append_payload(&mut pending.entry, entry.payload)?;
            if entry.payload.len() == CHUNK_BYTES {
                pending.next_part = number.checked_add(1).ok_or(AssemblyError::PartOrder)?;
                self.pending = Some(pending);
            } else {
                self.complete(pending.entry)?;
            }
        } else {
            if self.pending.is_some() {
                return Err(AssemblyError::Interleaved);
            }
            if self.started_paths.get(&path) == Some(&true) {
                return Err(AssemblyError::DuplicateEntry);
            }
            let mut owned = self.make_entry(entry, logical, path.clone())?;
            self.started_paths.insert(path, false);
            self.append_payload(&mut owned, entry.payload)?;
            self.complete(owned)?;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<AssembledArchive, AssemblyError> {
        if self.failed {
            return Err(AssemblyError::Poisoned);
        }
        if !self.admission.is_complete() || self.pending.is_some() {
            return Err(AssemblyError::Incomplete);
        }
        Ok(AssembledArchive {
            entries: self.entries,
        })
    }
}

#[cfg(test)]
mod openpgp_tests;
