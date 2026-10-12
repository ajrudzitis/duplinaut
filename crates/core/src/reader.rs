//! Native plain/gzip orchestration. Discovery is metadata-only; successful
//! reconstruction checks every selected set/volume but proves no authenticity
//! or filesystem publication. Whole-reader controller refinement is pending.
use crate::chain::{ChainError, ChainLimits, ChainPlan, DescriptorError, SetDescriptor};
use crate::envelope::{decode_metadata, EnvelopeError, EnvelopeLimits};
use crate::manifest::{Manifest, ManifestError, ManifestLimits};
use crate::multipart::{AssemblyError, AssemblyLimits, MultipartAssembler};
use crate::names::{
    parse_name, Envelope, NameConfig, NameError, ObjectKind, ObjectName, Timestamp,
};
use crate::path::BackupPath;
use crate::restore_plan::{RestorePlan, RestorePlanError, RestorePlanLimits};
use crate::source::{
    Cancellation, CheckedReader, ObjectSource, SourceError, SourceLimits, StoredObject,
};
use crate::state::{SnapshotState, StateError, StateLimits};
use std::io::Read;

#[derive(Clone, Copy, Debug)]
pub struct ReaderLimits {
    pub source: SourceLimits,
    pub catalog_metadata_bytes: usize,
    pub stored_buffer_bytes: usize,
    pub decoded_manifest_bytes: usize,
    pub normalization_metadata_bytes: usize,
    pub max_object_checks: usize,
    /// Manifest metadata envelope bounds. Volume bounds use `assembly.envelope`.
    pub envelope: EnvelopeLimits,
    pub manifest: ManifestLimits,
    pub assembly: AssemblyLimits,
    pub state: StateLimits,
    pub chain: ChainLimits,
}
impl Default for ReaderLimits {
    fn default() -> Self {
        Self {
            source: SourceLimits::default(),
            catalog_metadata_bytes: 256 * 1024 * 1024,
            stored_buffer_bytes: 256 * 1024 * 1024,
            decoded_manifest_bytes: 32 * 1024 * 1024,
            normalization_metadata_bytes: 32 * 1024 * 1024,
            max_object_checks: 16_000_000,
            envelope: EnvelopeLimits::default(),
            manifest: ManifestLimits::default(),
            assembly: AssemblyLimits::default(),
            state: StateLimits::default(),
            chain: ChainLimits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryStatus {
    Completed,
    Unknown,
    Malformed(NameError),
    Partial,
    UnsupportedEnvelope,
}
pub struct CatalogObject {
    object: StoredObject,
    name: Option<ObjectName>,
    status: DiscoveryStatus,
}
impl CatalogObject {
    pub fn object(&self) -> &StoredObject {
        &self.object
    }
    pub fn name(&self) -> Option<ObjectName> {
        self.name
    }
    pub fn status(&self) -> DiscoveryStatus {
        self.status
    }
}
#[derive(Debug)]
pub enum ReaderError {
    Source(SourceError),
    Limit,
    Allocation,
    CollectionAmbiguous,
    DuplicateObject,
    DuplicateArtifact,
    Filename(DescriptorError),
    Configuration(NameError),
    Chain(ChainError),
    Envelope(EnvelopeError),
    Manifest(ManifestError),
    MissingVolume { number: u64 },
    Assembly(AssemblyError),
    State(StateError),
}
pub struct Catalog<'a> {
    source: &'a dyn ObjectSource,
    objects: Vec<CatalogObject>,
    manifest_objects: Vec<usize>,
    descriptors: Vec<SetDescriptor>,
    limits: ReaderLimits,
}
fn parent(key: &str) -> &str {
    key.rsplit_once('/').map_or("", |(parent, _)| parent)
}
fn semantic(name: ObjectName) -> Option<(u8, i128, i128, Option<u64>)> {
    if !name.completed() {
        return None;
    }
    let kind = match name.kind {
        ObjectKind::FullManifest => 0,
        ObjectKind::IncrementalManifest => 1,
        ObjectKind::FullVolume => 2,
        ObjectKind::IncrementalVolume => 3,
        _ => return None,
    };
    Some((kind, name.start.0, name.end.0, name.volume))
}
impl<'a> Catalog<'a> {
    //= docs/parsing-requirements.md#parse-source-001
    //= type=implementation
    //# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
    /// Discover one flat key-parent backup collection without reading objects.
    /// Completed names are metadata evidence, never validated restorability.
    pub fn discover(
        source: &'a dyn ObjectSource,
        prefix: &str,
        names: NameConfig<'_>,
        limits: ReaderLimits,
        cancel: &Cancellation,
    ) -> Result<Self, ReaderError> {
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        if let Err(NameError::Configuration) = parse_name(b"", names) {
            return Err(ReaderError::Configuration(NameError::Configuration));
        }
        let listed = source
            .list(prefix, limits.source, cancel)
            .map_err(ReaderError::Source)?;
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        if listed.len() > limits.source.max_objects {
            return Err(ReaderError::Limit);
        }
        // Charge listing overlap, exact-capacity catalog/descriptors/indexes and
        // temporary sorted indices before allocating. String capacities remain
        // owned by their original StoredObjects; no key/identity clone is made.
        let mut charged = listed
            .capacity()
            .checked_mul(std::mem::size_of::<StoredObject>())
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
            .ok_or(ReaderError::Limit)?;
        for object in &listed {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            if !object.key.starts_with(prefix) {
                return Err(ReaderError::CollectionAmbiguous);
            }
            let identities = object
                .identity
                .etag
                .as_ref()
                .map_or(0, String::capacity)
                .checked_add(
                    object
                        .identity
                        .version_id
                        .as_ref()
                        .map_or(0, String::capacity),
                )
                .ok_or(ReaderError::Limit)?;
            charged = charged
                .checked_add(object.key.capacity())
                .and_then(|n| n.checked_add(identities))
                .and_then(|n| {
                    n.checked_add(
                        std::mem::size_of::<CatalogObject>()
                            + std::mem::size_of::<SetDescriptor>()
                            + 4 * std::mem::size_of::<usize>(),
                    )
                })
                .filter(|n| *n <= limits.catalog_metadata_bytes)
                .ok_or(ReaderError::Limit)?;
        }
        if charged > limits.catalog_metadata_bytes {
            return Err(ReaderError::Limit);
        }
        let mut order: Vec<usize> = (0..listed.len()).collect();
        order.sort_unstable_by(|a, b| listed[*a].key.cmp(&listed[*b].key));
        if order
            .windows(2)
            .any(|pair| listed[pair[0]].key == listed[pair[1]].key)
        {
            return Err(ReaderError::DuplicateObject);
        }
        let mut objects = Vec::with_capacity(listed.len());
        let mut manifest_objects = Vec::with_capacity(listed.len());
        let mut descriptors = Vec::with_capacity(listed.len());
        for object in listed {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            let basename = object.key.rsplit('/').next().unwrap_or("");
            let (name, status) = match parse_name(basename.as_bytes(), names) {
                Ok(None) => (None, DiscoveryStatus::Unknown),
                Err(error) => (None, DiscoveryStatus::Malformed(error)),
                Ok(Some(name)) => {
                    let status = if name.partial {
                        DiscoveryStatus::Partial
                    } else if matches!(name.envelope, Envelope::OpenPgp | Envelope::Pkcs7) {
                        DiscoveryStatus::UnsupportedEnvelope
                    } else {
                        DiscoveryStatus::Completed
                    };
                    (Some(name), status)
                }
            };
            if let Some(name) = name {
                if name.completed()
                    && matches!(
                        name.kind,
                        ObjectKind::FullManifest | ObjectKind::IncrementalManifest
                    )
                {
                    descriptors.push(
                        SetDescriptor::from_manifest_name(name).map_err(ReaderError::Filename)?,
                    );
                    manifest_objects.push(objects.len());
                }
            }
            objects.push(CatalogObject {
                object,
                name,
                status,
            });
        }
        let mut collection = None;
        for object in &objects {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            if object.name.is_some_and(ObjectName::completed) {
                let current = parent(&object.object.key);
                if collection.is_some_and(|previous| previous != current) {
                    return Err(ReaderError::CollectionAmbiguous);
                }
                collection = Some(current);
            }
        }
        order.retain(|index| objects[*index].name.and_then(semantic).is_some());
        order.sort_unstable_by_key(|index| objects[*index].name.and_then(semantic));
        if order.windows(2).any(|pair| {
            objects[pair[0]].name.and_then(semantic) == objects[pair[1]].name.and_then(semantic)
        }) {
            return Err(ReaderError::DuplicateArtifact);
        }
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        Ok(Self {
            source,
            objects,
            manifest_objects,
            descriptors,
            limits,
        })
    }
    pub fn objects(&self) -> &[CatalogObject] {
        &self.objects
    }
    /// Indices identify the immutable descriptor sequence used by read_snapshot.
    pub fn manifests(&self) -> impl Iterator<Item = (usize, &StoredObject, &SetDescriptor)> {
        self.manifest_objects
            .iter()
            .enumerate()
            .map(|(index, object)| {
                (
                    index,
                    &self.objects[*object].object,
                    &self.descriptors[index],
                )
            })
    }
    fn read_stored(
        &self,
        object: &StoredObject,
        cancel: &Cancellation,
    ) -> Result<Vec<u8>, ReaderError> {
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        let size = usize::try_from(object.size).map_err(|_| ReaderError::Limit)?;
        if size > self.limits.stored_buffer_bytes {
            return Err(ReaderError::Limit);
        }
        if object.size > self.limits.source.max_stored_bytes {
            return Err(ReaderError::Source(SourceError::Limit));
        }
        let stream = self
            .source
            .open(object, self.limits.source, cancel)
            .map_err(ReaderError::Source)?;
        let mut stream = CheckedReader::new(
            stream,
            object.size,
            self.limits.source.max_stored_bytes,
            cancel.clone(),
        )
        .map_err(ReaderError::Source)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| ReaderError::Allocation)?;
        let mut buffer = [0u8; 65536];
        loop {
            let n = match stream.read(&mut buffer) {
                Ok(n) => n,
                Err(error)
                    if error.kind() == std::io::ErrorKind::Interrupted
                        && !cancel.is_cancelled() =>
                {
                    continue
                }
                Err(_) if cancel.is_cancelled() => {
                    return Err(ReaderError::Source(SourceError::Cancelled))
                }
                Err(error) => return Err(ReaderError::Source(SourceError::Io(error))),
            };
            if n == 0 {
                break;
            }
            if n > size - bytes.len() {
                return Err(ReaderError::Limit);
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        Ok(bytes)
    }
    //= docs/parsing-requirements.md#parse-chain-001
    //= type=implementation
    //# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
    //= docs/parsing-requirements.md#parse-chain-002
    //= type=implementation
    //# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
    //= docs/parsing-requirements.md#parse-multi-003
    //= type=implementation
    //# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
    /// Reconstruct the explicit full-anchor/exact-date chain. Every normalized
    /// volume is read; raw interval hints never skip required integrity evidence.
    pub fn read_snapshot(
        &self,
        full_anchor: usize,
        target: Timestamp,
        cancel: &Cancellation,
    ) -> Result<ValidatedSnapshot, ReaderError> {
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        let plan = ChainPlan::new(&self.descriptors, full_anchor, target, self.limits.chain)
            .map_err(ReaderError::Chain)?;
        let mut state: Option<SnapshotState> = None;
        let mut checks = 0usize;
        for &index in plan.indices() {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            let object = &self.objects[self.manifest_objects[index]];
            let name = object.name.unwrap();
            if !matches!(name.envelope, Envelope::Plain | Envelope::Gzip) {
                return Err(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope));
            }
            let stored = self.read_stored(&object.object, cancel)?;
            let mut envelope_limits = self.limits.envelope;
            envelope_limits.decoded_bytes = envelope_limits
                .decoded_bytes
                .min(self.limits.decoded_manifest_bytes);
            let decoded = decode_metadata(&stored, name.envelope, envelope_limits)
                .map_err(ReaderError::Envelope)?;
            let manifest =
                Manifest::parse(&decoded, self.limits.manifest).map_err(ReaderError::Manifest)?;
            drop(decoded);
            drop(stored);
            let mut assembler = MultipartAssembler::new(&manifest, self.limits.assembly)
                .map_err(ReaderError::Assembly)?;
            // The iteration view is additional to the assembler's retained
            // normalized view, and has its own preallocation metadata bound.
            manifest
                .records
                .len()
                .max(4)
                .checked_mul(2 * std::mem::size_of::<&crate::manifest::VolumeRecord>())
                .filter(|n| *n <= self.limits.normalization_metadata_bytes)
                .ok_or(ReaderError::Limit)?;
            for record in manifest.volumes() {
                if cancel.is_cancelled() {
                    return Err(ReaderError::Source(SourceError::Cancelled));
                }
                let mut candidate = None;
                for object in &self.objects {
                    if cancel.is_cancelled() {
                        return Err(ReaderError::Source(SourceError::Cancelled));
                    }
                    checks = checks
                        .checked_add(1)
                        .filter(|n| *n <= self.limits.max_object_checks)
                        .ok_or(ReaderError::Limit)?;
                    if object.name.is_some_and(|volume| {
                        volume.completed()
                            && volume.start == name.start
                            && volume.end == name.end
                            && volume.volume == Some(record.number)
                            && matches!(
                                (name.kind, volume.kind),
                                (ObjectKind::FullManifest, ObjectKind::FullVolume)
                                    | (
                                        ObjectKind::IncrementalManifest,
                                        ObjectKind::IncrementalVolume
                                    )
                            )
                    }) {
                        if candidate.is_some() {
                            return Err(ReaderError::DuplicateArtifact);
                        }
                        candidate = Some(object);
                    }
                }
                let object = candidate.ok_or(ReaderError::MissingVolume {
                    number: record.number,
                })?;
                let envelope = object.name.unwrap().envelope;
                if !matches!(envelope, Envelope::Plain | Envelope::Gzip) {
                    return Err(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope));
                }
                let stored = self.read_stored(&object.object, cancel)?;
                assembler
                    .push_stored_volume(record.number, &stored, envelope)
                    .map_err(ReaderError::Assembly)?;
            }
            let archive = assembler.finish().map_err(ReaderError::Assembly)?;
            state = Some(
                match state {
                    None => SnapshotState::from_full(&archive, self.limits.state),
                    Some(previous) => previous.apply_incremental(&archive, self.limits.state),
                }
                .map_err(ReaderError::State)?,
            );
        }
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        Ok(ValidatedSnapshot {
            target,
            state: state.ok_or(ReaderError::Chain(ChainError::Missing))?,
        })
    }
}
/// Fully reconstructed under this reader's supported plain/gzip checks. This is
/// not a signer/authenticity certificate or a published filesystem restoration.
pub struct ValidatedSnapshot {
    target: Timestamp,
    state: SnapshotState,
}
impl ValidatedSnapshot {
    pub fn target(&self) -> Timestamp {
        self.target
    }
    pub fn state(&self) -> &SnapshotState {
        &self.state
    }
    pub fn restore_all(
        &self,
        limits: RestorePlanLimits,
    ) -> Result<RestorePlan<'_>, RestorePlanError<'_>> {
        RestorePlan::all(&self.state, limits)
    }
    pub fn restore_roots(
        &self,
        roots: &[BackupPath],
        limits: RestorePlanLimits,
    ) -> Result<RestorePlan<'_>, RestorePlanError<'_>> {
        RestorePlan::from_roots(&self.state, roots, limits)
    }
}
