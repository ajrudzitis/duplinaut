//! Native plain/gzip and bounded OpenPGP reconstruction. Discovery is
//! metadata-only; successful reads retain explicit envelope/authentication
//! observations but do not prove external signer validity or publish files.
//! Whole-reader controller refinement is pending.
use crate::chain::{ChainError, ChainLimits, ChainPlan, DescriptorError, SetDescriptor};
use crate::envelope::{decode_metadata, EnvelopeError, EnvelopeLimits};
use crate::manifest::{Manifest, ManifestError, ManifestLimits};
use crate::multipart::{AssemblyError, AssemblyLimits, MultipartAssembler};
use crate::names::{
    parse_name, Envelope, NameConfig, NameError, ObjectKind, ObjectName, Timestamp,
};
use crate::openpgp::{self, Credentials, CryptoError};
pub use crate::openpgp::{
    AuthorizedSigner as AuthorizedSignerEvidence,
    CertificateMetadata as OpenPgpCertificateMetadata,
    CertificateSignatureMetadata as OpenPgpSignatureMetadata, Credentials as CryptoCredentials,
    CryptoError as OpenPgpError, CryptoLimits as OpenPgpLimits, SecretExport,
};
use crate::path::BackupPath;
use crate::restore_plan::{RestorePlan, RestorePlanError, RestorePlanLimits};
use crate::source::{
    Cancellation, CheckedReader, ObjectSource, SourceError, SourceLimits, StoredObject,
};
use crate::state::{SnapshotState, StateError, StateLimits};
use crate::verified::catalog_accounting::{
    charge_catalog_object_metadata, initialize_catalog_metadata, CatalogObjectMetadata,
};
use crate::verified::reader_volume_scan::{
    advance_volume_scan_cursor, charge_volume_object, record_volume_candidate, reserve_volume_scan,
    start_volume_scan_cursor, volume_scan_cursor_complete, VolumeScan, VolumeScanError,
};
pub use pgp::types::Password as OpenPgpPassword;
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
#[derive(Clone, Copy, Debug)]
pub struct EvidenceLimits {
    pub max_objects: usize,
    pub max_metadata_bytes: usize,
    pub max_certificate_versions: usize,
    pub max_comparisons: usize,
}
impl Default for EvidenceLimits {
    fn default() -> Self {
        Self {
            max_objects: 1_000_000,
            max_metadata_bytes: 32 * 1024 * 1024,
            max_certificate_versions: 64,
            max_comparisons: 16_000_000,
        }
    }
}
/// Borrowed, immutable policy for one full-chain read. Credentials are never
/// copied into catalog/snapshot evidence or formatted into diagnostics.
pub struct CryptoReadContext<'a> {
    pub credentials: CryptoCredentials<'a>,
    pub crypto_limits: OpenPgpLimits,
    pub evidence_limits: EvidenceLimits,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationKind {
    PlainOrGzip,
    OpenPgpIntegrity,
    OpenPgpAuthorized,
}
/// Redacted policy record for one completed read. Fingerprint authorization is
/// offline: this records configured inputs and bounds, not present validity,
/// revocation policy, user identity, or Web-of-Trust acceptance.
#[derive(Clone, Debug)]
pub struct AuthenticationPolicySummary {
    pub authorization_configured: bool,
    pub require_signature: bool,
    pub permitted_primary_fingerprints: Vec<[u8; 20]>,
    pub crypto_limits: OpenPgpLimits,
    pub evidence_limits: EvidenceLimits,
}
#[derive(Clone, Debug)]
pub struct ObjectAuthentication {
    pub key: String,
    pub identity: crate::source::ObjectIdentity,
    pub set_start: Timestamp,
    pub set_end: Timestamp,
    pub manifest: bool,
    pub ordinal: u64,
    pub envelope: Envelope,
    pub kind: AuthenticationKind,
    pub authorized_signer: Option<AuthorizedSignerEvidence>,
    pub certificate_versions: Vec<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct AuthenticationEvidence {
    policy: Option<AuthenticationPolicySummary>,
    objects: Vec<ObjectAuthentication>,
    certificates: Vec<OpenPgpCertificateMetadata>,
}
impl AuthenticationEvidence {
    pub fn policy(&self) -> Option<&AuthenticationPolicySummary> {
        self.policy.as_ref()
    }
    pub fn objects(&self) -> &[ObjectAuthentication] {
        &self.objects
    }
    pub fn certificates(&self) -> &[OpenPgpCertificateMetadata] {
        &self.certificates
    }
}
fn policy_summary(
    context: Option<&CryptoReadContext<'_>>,
) -> Result<(Option<AuthenticationPolicySummary>, usize), ReaderError> {
    let Some(context) = context else {
        return Ok((None, 0));
    };
    if context.credentials.permitted_signers.len() > context.crypto_limits.certificates {
        return Err(ReaderError::Limit);
    }
    let minimum_bytes = std::mem::size_of::<AuthenticationPolicySummary>()
        .checked_add(
            context
                .credentials
                .permitted_signers
                .len()
                .checked_mul(std::mem::size_of::<[u8; 20]>())
                .ok_or(ReaderError::Limit)?,
        )
        .filter(|n| *n <= context.evidence_limits.max_metadata_bytes)
        .ok_or(ReaderError::Limit)?;
    let mut permitted = Vec::new();
    permitted
        .try_reserve_exact(context.credentials.permitted_signers.len())
        .map_err(|_| ReaderError::Allocation)?;
    permitted.extend_from_slice(context.credentials.permitted_signers);
    let bytes = std::mem::size_of::<AuthenticationPolicySummary>()
        .checked_add(
            permitted
                .capacity()
                .checked_mul(std::mem::size_of::<[u8; 20]>())
                .ok_or(ReaderError::Limit)?,
        )
        .filter(|n| *n >= minimum_bytes && *n <= context.evidence_limits.max_metadata_bytes)
        .ok_or(ReaderError::Limit)?;
    Ok((
        Some(AuthenticationPolicySummary {
            authorization_configured: context.credentials.require_signature
                || !context.credentials.permitted_signers.is_empty(),
            require_signature: context.credentials.require_signature,
            permitted_primary_fingerprints: permitted,
            crypto_limits: context.crypto_limits,
            evidence_limits: context.evidence_limits,
        }),
        bytes,
    ))
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
    Crypto(CryptoError),
}
pub struct Catalog<'a> {
    source: &'a dyn ObjectSource,
    objects: Vec<CatalogObject>,
    manifest_objects: Vec<usize>,
    descriptors: Vec<SetDescriptor>,
    limits: ReaderLimits,
}
fn credentials<'a>(context: &CryptoReadContext<'a>) -> Credentials<'a> {
    Credentials {
        message_passwords: context.credentials.message_passwords,
        secret_exports: context.credentials.secret_exports,
        signer_exports: context.credentials.signer_exports,
        permitted_signers: context.credentials.permitted_signers,
        require_signature: context.credentials.require_signature,
    }
}
fn source_metadata_charge(object: &StoredObject) -> Option<usize> {
    object
        .key
        .capacity()
        .checked_add(object.identity.etag.as_ref().map_or(0, String::capacity))?
        .checked_add(
            object
                .identity
                .version_id
                .as_ref()
                .map_or(0, String::capacity),
        )?
        .checked_add(std::mem::size_of::<ObjectAuthentication>())
}
#[allow(clippy::too_many_arguments)] // Evidence record fields map directly to the source declaration.
fn record_plain(
    evidence: &mut AuthenticationEvidence,
    used: &mut usize,
    object: &StoredObject,
    set_start: Timestamp,
    set_end: Timestamp,
    manifest: bool,
    ordinal: u64,
    envelope: Envelope,
    limits: Option<EvidenceLimits>,
) -> Result<(), ReaderError> {
    let Some(limits) = limits else { return Ok(()) };
    if evidence.objects.len() >= limits.max_objects {
        return Err(ReaderError::Limit);
    }
    let charge = source_metadata_charge(object).ok_or(ReaderError::Limit)?;
    *used = used
        .checked_add(charge)
        .filter(|n| *n <= limits.max_metadata_bytes)
        .ok_or(ReaderError::Limit)?;
    evidence
        .objects
        .try_reserve_exact(1)
        .map_err(|_| ReaderError::Allocation)?;
    evidence.objects.push(ObjectAuthentication {
        key: object.key.clone(),
        identity: object.identity.clone(),
        set_start,
        set_end,
        manifest,
        ordinal,
        envelope,
        kind: AuthenticationKind::PlainOrGzip,
        authorized_signer: None,
        certificate_versions: Vec::new(),
    });
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Evidence record fields map directly to the source declaration.
fn record_evidence(
    evidence: &mut AuthenticationEvidence,
    used: &mut usize,
    comparisons: &mut usize,
    object: &StoredObject,
    set_start: Timestamp,
    set_end: Timestamp,
    manifest: bool,
    ordinal: u64,
    envelope: Envelope,
    decoded: &openpgp::DecodedObject,
    limits: EvidenceLimits,
) -> Result<(), ReaderError> {
    if evidence.objects.len() >= limits.max_objects {
        return Err(ReaderError::Limit);
    }
    let mut charge = source_metadata_charge(object).ok_or(ReaderError::Limit)?;
    charge = charge
        .checked_add(
            decoded
                .certificates
                .len()
                .checked_mul(std::mem::size_of::<usize>())
                .ok_or(ReaderError::Limit)?,
        )
        .ok_or(ReaderError::Limit)?;
    *used = used
        .checked_add(charge)
        .filter(|n| *n <= limits.max_metadata_bytes)
        .ok_or(ReaderError::Limit)?;
    let mut versions = Vec::new();
    versions
        .try_reserve_exact(decoded.certificates.len())
        .map_err(|_| ReaderError::Allocation)?;
    for certificate in &decoded.certificates {
        *comparisons = comparisons
            .checked_add(evidence.certificates.len())
            .filter(|n| *n <= limits.max_comparisons)
            .ok_or(ReaderError::Limit)?;
        if let Some(index) = evidence
            .certificates
            .iter()
            .position(|known| known == certificate)
        {
            versions.push(index);
        } else {
            if evidence.certificates.len() >= limits.max_certificate_versions {
                return Err(ReaderError::Limit);
            }
            let cert_charge = std::mem::size_of::<OpenPgpCertificateMetadata>()
                .checked_add(
                    certificate
                        .signatures
                        .capacity()
                        .checked_mul(std::mem::size_of::<
                            crate::openpgp::CertificateSignatureMetadata,
                        >())
                        .ok_or(ReaderError::Limit)?,
                )
                .ok_or(ReaderError::Limit)?;
            *used = used
                .checked_add(cert_charge)
                .filter(|n| *n <= limits.max_metadata_bytes)
                .ok_or(ReaderError::Limit)?;
            evidence
                .certificates
                .try_reserve_exact(1)
                .map_err(|_| ReaderError::Allocation)?;
            let index = evidence.certificates.len();
            evidence.certificates.push(certificate.clone());
            versions.push(index);
        }
    }
    evidence
        .objects
        .try_reserve_exact(1)
        .map_err(|_| ReaderError::Allocation)?;
    evidence.objects.push(ObjectAuthentication {
        key: object.key.clone(),
        identity: object.identity.clone(),
        set_start,
        set_end,
        manifest,
        ordinal,
        envelope,
        kind: if decoded.authorized_signer.is_some() {
            AuthenticationKind::OpenPgpAuthorized
        } else {
            AuthenticationKind::OpenPgpIntegrity
        },
        authorized_signer: decoded.authorized_signer.clone(),
        certificate_versions: versions,
    });
    Ok(())
}
#[cfg(test)]
mod crypto_evidence_tests;
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
        // Charge listing overlap, requested catalog/descriptor/index slots and
        // temporary sorted indices before allocating. String capacities remain
        // owned by their original StoredObjects; no key/identity clone is made.
        let mut charged = initialize_catalog_metadata(
            listed.capacity(),
            std::mem::size_of::<StoredObject>(),
            std::mem::size_of::<Self>(),
            limits.catalog_metadata_bytes,
        )
        .ok_or(ReaderError::Limit)?;
        let index_metadata_bytes = std::mem::size_of::<usize>()
            .checked_mul(4)
            .ok_or(ReaderError::Limit)?;
        for object in &listed {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            if !object.key.starts_with(prefix) {
                return Err(ReaderError::CollectionAmbiguous);
            }
            let etag_capacity = object.identity.etag.as_ref().map_or(0, String::capacity);
            let version_capacity = object
                .identity
                .version_id
                .as_ref()
                .map_or(0, String::capacity);
            charged = charge_catalog_object_metadata(
                charged,
                CatalogObjectMetadata {
                    key_capacity: object.key.capacity(),
                    etag_capacity,
                    version_capacity,
                    catalog_object_bytes: std::mem::size_of::<CatalogObject>(),
                    descriptor_bytes: std::mem::size_of::<SetDescriptor>(),
                    index_bytes: index_metadata_bytes,
                },
                limits.catalog_metadata_bytes,
            )
            .ok_or(ReaderError::Limit)?;
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
                    } else if name.envelope == Envelope::Pkcs7 {
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
        maximum: usize,
    ) -> Result<Vec<u8>, ReaderError> {
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        let size = usize::try_from(object.size).map_err(|_| ReaderError::Limit)?;
        if size > self.limits.stored_buffer_bytes || size > maximum {
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
    //= docs/parsing-requirements.md#parse-env-018
    //= type=implementation
    //# OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.
    //= docs/parsing-requirements.md#parse-env-019
    //= type=implementation
    //# A snapshot read with signer authorization configured MUST require that authorization for every required manifest and data volume throughout the selected full/incremental chain, rejecting unsigned or plain/gzip required objects without policy downgrade.
    //= docs/parsing-requirements.md#parse-env-021
    //= type=implementation
    //# The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.
    //= docs/parsing-requirements.md#parse-env-022
    //= type=implementation
    //# Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.
    //= docs/parsing-requirements.md#parse-env-023
    //= type=implementation
    //# Snapshot authentication evidence MUST distinguish integrity-only objects from configured signer authorization and preserve available complete signer fingerprints and certificate creation/expiry/revocation observations without implying current validity or Web-of-Trust acceptance.
    //= docs/parsing-requirements.md#parse-env-024
    //= type=implementation
    //# The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.
    /// Reconstruct the explicit full-anchor/exact-date chain. Every normalized
    /// volume is read; raw interval hints never skip required integrity evidence.
    pub fn read_snapshot(
        &self,
        full_anchor: usize,
        target: Timestamp,
        cancel: &Cancellation,
    ) -> Result<ValidatedSnapshot, ReaderError> {
        self.read_snapshot_inner(full_anchor, target, cancel, None)
    }
    /// Reconstruct a full selected chain using borrowed OpenPGP credentials and
    /// return bounded per-object authentication observations with the snapshot.
    pub fn read_snapshot_with_crypto(
        &self,
        full_anchor: usize,
        target: Timestamp,
        cancel: &Cancellation,
        context: &CryptoReadContext<'_>,
    ) -> Result<ValidatedSnapshot, ReaderError> {
        self.read_snapshot_inner(full_anchor, target, cancel, Some(context))
    }
    fn read_snapshot_inner(
        &self,
        full_anchor: usize,
        target: Timestamp,
        cancel: &Cancellation,
        context: Option<&CryptoReadContext<'_>>,
    ) -> Result<ValidatedSnapshot, ReaderError> {
        if cancel.is_cancelled() {
            return Err(ReaderError::Source(SourceError::Cancelled));
        }
        let plan = ChainPlan::new(&self.descriptors, full_anchor, target, self.limits.chain)
            .map_err(ReaderError::Chain)?;
        let mut state: Option<SnapshotState> = None;
        let mut checks = 0usize;
        let (policy, policy_bytes) = policy_summary(context)?;
        let mut evidence = AuthenticationEvidence {
            policy,
            ..AuthenticationEvidence::default()
        };
        let mut evidence_bytes = policy_bytes;
        let mut comparisons = 0usize;
        let signer_required = context.is_some_and(|context| {
            context.credentials.require_signature
                || !context.credentials.permitted_signers.is_empty()
        });
        for &index in plan.indices() {
            if cancel.is_cancelled() {
                return Err(ReaderError::Source(SourceError::Cancelled));
            }
            let object = &self.objects[self.manifest_objects[index]];
            let name = object.name.unwrap();
            if name.envelope == Envelope::OpenPgp && context.is_none() {
                return Err(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope));
            }
            if signer_required && !matches!(name.envelope, Envelope::OpenPgp) {
                return Err(ReaderError::Crypto(CryptoError::Unsigned));
            }
            let source_max =
                usize::try_from(self.limits.source.max_stored_bytes).unwrap_or(usize::MAX);
            let stored_limit = if name.envelope == Envelope::OpenPgp {
                context
                    .map_or(0, |context| context.crypto_limits.stored_bytes)
                    .min(self.limits.stored_buffer_bytes)
                    .min(self.limits.envelope.stored_bytes)
                    .min(source_max)
            } else {
                self.limits.stored_buffer_bytes.min(source_max)
            };
            let stored = self.read_stored(&object.object, cancel, stored_limit)?;
            let decoded = if name.envelope == Envelope::OpenPgp {
                let context =
                    context.ok_or(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope))?;
                let mut limits = context.crypto_limits;
                limits.stored_bytes = stored_limit;
                limits.decoded_bytes = limits
                    .decoded_bytes
                    .min(self.limits.decoded_manifest_bytes)
                    .min(self.limits.envelope.decoded_bytes);
                if cancel.is_cancelled() {
                    return Err(ReaderError::Source(SourceError::Cancelled));
                }
                let decoded = openpgp::decode(&stored, credentials(context), limits)
                    .map_err(ReaderError::Crypto)?;
                if cancel.is_cancelled() {
                    return Err(ReaderError::Source(SourceError::Cancelled));
                }
                record_evidence(
                    &mut evidence,
                    &mut evidence_bytes,
                    &mut comparisons,
                    &object.object,
                    name.start,
                    name.end,
                    true,
                    0,
                    name.envelope,
                    &decoded,
                    context.evidence_limits,
                )?;
                decoded.bytes
            } else {
                if !matches!(name.envelope, Envelope::Plain | Envelope::Gzip) {
                    return Err(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope));
                }
                let mut limits = self.limits.envelope;
                limits.decoded_bytes = limits.decoded_bytes.min(self.limits.decoded_manifest_bytes);
                let bytes = decode_metadata(&stored, name.envelope, limits)
                    .map_err(ReaderError::Envelope)?;
                record_plain(
                    &mut evidence,
                    &mut evidence_bytes,
                    &object.object,
                    name.start,
                    name.end,
                    true,
                    0,
                    name.envelope,
                    context.map(|x| x.evidence_limits),
                )?;
                bytes
            };
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
            let volumes = manifest.volumes();
            let reservation = reserve_volume_scan(
                checks,
                volumes.len(),
                self.objects.len(),
                self.limits.max_object_checks,
            )
            .ok_or(ReaderError::Limit)?;
            let mut cursor =
                start_volume_scan_cursor(reservation, volumes.len(), self.objects.len())
                    .ok_or(ReaderError::Limit)?;
            for (volume_index, record) in volumes.iter().enumerate() {
                if cancel.is_cancelled() {
                    return Err(ReaderError::Source(SourceError::Cancelled));
                }
                let mut scan = VolumeScan {
                    checked_objects: checks,
                    candidate_index: None,
                };
                for (object_index, object) in self.objects.iter().enumerate() {
                    if cancel.is_cancelled() {
                        return Err(ReaderError::Source(SourceError::Cancelled));
                    }
                    cursor = advance_volume_scan_cursor(cursor, volume_index, object_index)
                        .ok_or(ReaderError::Limit)?;
                    let charged = charge_volume_object(
                        scan,
                        reservation.final_checks,
                        object_index,
                        self.objects.len(),
                    )
                    .map_err(|error| match error {
                        VolumeScanError::Limit => ReaderError::Limit,
                        VolumeScanError::Duplicate => ReaderError::DuplicateArtifact,
                    })?;
                    let matches = object.name.is_some_and(|volume| {
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
                    });
                    scan =
                        record_volume_candidate(charged, matches).map_err(|error| match error {
                            VolumeScanError::Limit => ReaderError::Limit,
                            VolumeScanError::Duplicate => ReaderError::DuplicateArtifact,
                        })?;
                }
                checks = scan.checked_objects;
                let object = match scan.candidate_index {
                    Some(index) => self.objects.get(index).ok_or(ReaderError::Limit)?,
                    None => {
                        return Err(ReaderError::MissingVolume {
                            number: record.number,
                        })
                    }
                };
                let envelope = object.name.unwrap().envelope;
                if signer_required && envelope != Envelope::OpenPgp {
                    return Err(ReaderError::Crypto(CryptoError::Unsigned));
                }
                let stored_limit = if envelope == Envelope::OpenPgp {
                    context
                        .ok_or(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope))?
                        .crypto_limits
                        .stored_bytes
                        .min(self.limits.stored_buffer_bytes)
                        .min(self.limits.assembly.envelope.stored_bytes)
                        .min(
                            usize::try_from(self.limits.source.max_stored_bytes)
                                .unwrap_or(usize::MAX),
                        )
                } else {
                    self.limits.stored_buffer_bytes.min(
                        usize::try_from(self.limits.source.max_stored_bytes).unwrap_or(usize::MAX),
                    )
                };
                let stored = self.read_stored(&object.object, cancel, stored_limit)?;
                if envelope == Envelope::OpenPgp {
                    let context =
                        context.ok_or(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope))?;
                    let mut limits = context.crypto_limits;
                    limits.stored_bytes = stored_limit;
                    limits.decoded_bytes = limits
                        .decoded_bytes
                        .min(self.limits.assembly.envelope.decoded_bytes)
                        .min(self.limits.assembly.tar.max_bytes);
                    if cancel.is_cancelled() {
                        return Err(ReaderError::Source(SourceError::Cancelled));
                    }
                    let decoded = assembler
                        .push_openpgp_volume(
                            record.number,
                            &stored,
                            credentials(context),
                            limits,
                            cancel,
                        )
                        .map_err(|error| {
                            if error == AssemblyError::Cancelled {
                                ReaderError::Source(SourceError::Cancelled)
                            } else {
                                ReaderError::Assembly(error)
                            }
                        })?;
                    if cancel.is_cancelled() {
                        return Err(ReaderError::Source(SourceError::Cancelled));
                    }
                    record_evidence(
                        &mut evidence,
                        &mut evidence_bytes,
                        &mut comparisons,
                        &object.object,
                        name.start,
                        name.end,
                        false,
                        record.number,
                        envelope,
                        &decoded,
                        context.evidence_limits,
                    )?;
                } else {
                    if !matches!(envelope, Envelope::Plain | Envelope::Gzip) {
                        return Err(ReaderError::Envelope(EnvelopeError::UnsupportedEnvelope));
                    }
                    assembler
                        .push_stored_volume(record.number, &stored, envelope)
                        .map_err(ReaderError::Assembly)?;
                    record_plain(
                        &mut evidence,
                        &mut evidence_bytes,
                        &object.object,
                        name.start,
                        name.end,
                        false,
                        record.number,
                        envelope,
                        context.map(|x| x.evidence_limits),
                    )?;
                }
            }
            if !volume_scan_cursor_complete(cursor) || checks != reservation.final_checks {
                return Err(ReaderError::Limit);
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
            evidence,
        })
    }
}
/// Fully reconstructed under this reader's supported plain/gzip checks. This is
/// not a signer/authenticity certificate or a published filesystem restoration.
pub struct ValidatedSnapshot {
    target: Timestamp,
    state: SnapshotState,
    evidence: AuthenticationEvidence,
}
impl ValidatedSnapshot {
    pub fn target(&self) -> Timestamp {
        self.target
    }
    pub fn state(&self) -> &SnapshotState {
        &self.state
    }
    pub fn authentication_evidence(&self) -> &AuthenticationEvidence {
        &self.evidence
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
