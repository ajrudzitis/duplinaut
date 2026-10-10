//! Bounded manifest framing and contextual interpretation.
//!
//! Vest supplies the shared byte-token grammar. Line framing, counted sections,
//! singleton fields and restart normalization are stateful semantic operations,
//! kept here as explicit Verus-friendly loops for independent proof review.
// Keep length predicates, byte bounds and explicit error branches visible to
// Verus's arithmetic and control-flow reasoning. These narrow style exceptions
// avoid replacing reviewed kernels with trait-dispatched ranges or Try lowering.
#![allow(clippy::len_zero, clippy::manual_range_contains, clippy::question_mark)]
use crate::manifest_token::{decode_token, TokenError};
use crate::path::BackupPath;
use vstd::prelude::*;

verus! {
#[derive(Debug, Clone, Copy)]
pub struct ManifestLimits {
    pub metadata_bytes: usize,
    pub line_bytes: usize,
    pub entries: usize,
    pub path_bytes: usize,
    pub path_components: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError { Malformed, UnsupportedIntegrity, Limit }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestAlgorithm { Sha1, Md5, Unsupported }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredDigest {
    pub algorithm: DigestAlgorithm,
    pub name: Vec<u8>,
    /// Hexadecimal bytes, normalized to lowercase. Digests describe stored bytes.
    pub hex: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary { pub path: BackupPath, pub block: Option<u64> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeRecord {
    pub number: u64,
    pub start: Boundary,
    pub end: Boundary,
    pub hashes: Vec<DeclaredDigest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction { New, Changed, Deleted }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange { pub action: FileAction, pub path: BackupPath }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub hostname: Option<Vec<u8>>,
    pub localdir: Option<Vec<u8>>,
    /// Original ordered records are retained, including overwritten restart data.
    pub records: Vec<VolumeRecord>,
    /// None means absent, rather than an empty change summary.
    pub filelist: Option<Vec<FileChange>>,
}

struct PendingVolume {
    number: u64, start: Option<Boundary>, end: Option<Boundary>, hashes: Vec<DeclaredDigest>,
}

fn space(byte: u8) -> bool { byte == b' ' || byte == b'\t' }
fn lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' { byte + 32 } else { byte }
}
fn equal_word(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() { return false; }
    let mut i = 0;
    while i < left.len() {
        if lower(left[i]) != lower(right[i]) { return false; }
        i += 1;
    }
    true
}
fn owned(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() { result.push(bytes[i]); i += 1; }
    result
}
fn trim(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && space(bytes[start]) { start += 1; }
    while start < end && space(bytes[end - 1]) { end -= 1; }
    vstd::slice::slice_subrange(bytes, start, end)
}
fn word(bytes: &[u8]) -> (&[u8], &[u8]) {
    let bytes = trim(bytes);
    let mut end = 0;
    while end < bytes.len() && !space(bytes[end]) { end += 1; }
    (vstd::slice::slice_subrange(bytes, 0, end), trim(vstd::slice::slice_subrange(bytes, end, bytes.len())))
}
fn decimal(bytes: &[u8], positive: bool) -> Result<u64, ManifestError> {
    if bytes.len() == 0 { return Err(ManifestError::Malformed); }
    let mut number: u64 = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] < b'0' || bytes[i] > b'9' { return Err(ManifestError::Malformed); }
        let digit = (bytes[i] - b'0') as u64;
        if number > (u64::MAX - digit) / 10 { return Err(ManifestError::Malformed); }
        number = number * 10 + digit;
        i += 1;
    }
    if positive && number == 0 { return Err(ManifestError::Malformed); }
    Ok(number)
}
fn token<'a>(bytes: &'a [u8], limits: &ManifestLimits) -> Result<(Vec<u8>, &'a [u8]), ManifestError> {
    let (consumed, decoded) = match decode_token(bytes, limits.line_bytes) {
        Ok(value) => value,
        Err(TokenError::Limit) => return Err(ManifestError::Limit),
        Err(_) => return Err(ManifestError::Malformed),
    };
    if consumed < bytes.len() && !space(bytes[consumed]) { return Err(ManifestError::Malformed); }
    Ok((decoded, trim(vstd::slice::slice_subrange(bytes, consumed, bytes.len()))))
}
fn path(bytes: &[u8], limits: &ManifestLimits) -> Result<BackupPath, ManifestError> {
    match BackupPath::parse(bytes, false, limits.path_bytes, limits.path_components) {
        Ok(value) => Ok(value),
        Err(crate::path::PathError::ByteLimit) | Err(crate::path::PathError::ComponentLimit) => Err(ManifestError::Limit),
        Err(_) => Err(ManifestError::Malformed),
    }
}
fn boundary(bytes: &[u8], limits: &ManifestLimits) -> Result<Boundary, ManifestError> {
    let (decoded, rest) = match token(bytes, limits) { Ok(value) => value, Err(error) => return Err(error) };
    let path = match path(&decoded, limits) { Ok(value) => value, Err(error) => return Err(error) };
    let block = if rest.len() == 0 { None } else {
        match decimal(rest, true) { Ok(value) => Some(value), Err(error) => return Err(error) }
    };
    Ok(Boundary { path, block })
}
fn digest(bytes: &[u8]) -> Result<DeclaredDigest, ManifestError> {
    let (name, hex) = word(bytes);
    if name.len() == 0 || hex.len() == 0 { return Err(ManifestError::Malformed); }
    let algorithm = if equal_word(name, b"SHA1") { DigestAlgorithm::Sha1 }
        else if equal_word(name, b"MD5") { DigestAlgorithm::Md5 } else { DigestAlgorithm::Unsupported };
    if (algorithm == DigestAlgorithm::Sha1 && hex.len() != 40)
        || (algorithm == DigestAlgorithm::Md5 && hex.len() != 32) { return Err(ManifestError::Malformed); }
    let mut normalized_name = Vec::new();
    let mut i = 0;
    while i < name.len() { normalized_name.push(lower(name[i])); i += 1; }
    let mut normalized_hex = Vec::new();
    i = 0;
    while i < hex.len() {
        let byte = lower(hex[i]);
        if !((byte >= b'0' && byte <= b'9') || (byte >= b'a' && byte <= b'f')) {
            return Err(ManifestError::Malformed);
        }
        normalized_hex.push(byte); i += 1;
    }
    Ok(DeclaredDigest { algorithm, name: normalized_name, hex: normalized_hex })
}
fn finish(pending: PendingVolume) -> Result<VolumeRecord, ManifestError> {
    let start = match pending.start { Some(value) => value, None => return Err(ManifestError::Malformed) };
    let end = match pending.end { Some(value) => value, None => return Err(ManifestError::Malformed) };
    if start.path.compare(&end.path) > 0 { return Err(ManifestError::Malformed); }
    if pending.hashes.len() == 0 { return Err(ManifestError::Malformed); }
    let mut supported = false;
    let mut i = 0;
    while i < pending.hashes.len() {
        if pending.hashes[i].algorithm != DigestAlgorithm::Unsupported { supported = true; }
        i += 1;
    }
    if !supported { return Err(ManifestError::UnsupportedIntegrity); }
    Ok(VolumeRecord { number: pending.number, start, end, hashes: pending.hashes })
}

impl Manifest {
    //= docs/parsing-requirements.md#parse-man-001
    //# The manifest parser MUST recover ordered volume records containing volume number, starting path, ending path, optional block boundaries, and declared hashes.
    //= docs/parsing-requirements.md#parse-man-006
    //# The reader MUST retain the distinction between absent block boundaries and explicit numeric block boundaries.
    //= docs/parsing-requirements.md#parse-man-007
    //# The manifest parser MUST accept an optional counted Filelist section before or after volume records without interpreting its lines as volume fields.
    //= docs/parsing-requirements.md#parse-env-006
    //# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
    pub fn parse(bytes: &[u8], limits: ManifestLimits) -> Result<Self, ManifestError> {
        if bytes.len() > limits.metadata_bytes { return Err(ManifestError::Limit); }
        let mut manifest = Self { hostname: None, localdir: None, records: Vec::new(), filelist: None };
        let mut pending: Option<PendingVolume> = None;
        let mut remaining: u64 = 0;
        let mut filelist_closed = false;
        let mut entry_count = 0;
        let mut cursor = 0;
        let mut metadata_reserved = 0;
        while cursor < bytes.len() {
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                if cursor - start >= limits.line_bytes { return Err(ManifestError::Limit); }
                cursor += 1;
            }
            let mut end = cursor;
            if end > start && bytes[end - 1] == b'\r' { end -= 1; }
            let raw = vstd::slice::slice_subrange(bytes, start, end);
            if cursor < bytes.len() { cursor += 1; }
            let line = trim(raw);
            if line.len() == 0 { continue; }
            // Reserve before decoding: bytes in fields plus conservative vector,
            // record and worst-case byte-component allocation overhead. This
            // intentionally overestimates retained metadata rather than treating
            // serialized bytes as the size of an allocated tree of vectors.
            if metadata_reserved > limits.metadata_bytes
                || limits.metadata_bytes - metadata_reserved < 512 { return Err(ManifestError::Limit); }
            metadata_reserved += 512;
            if line.len() > (limits.metadata_bytes - metadata_reserved) / 80 { return Err(ManifestError::Limit); }
            metadata_reserved += line.len() * 80;
            let indented = raw.len() > 0 && space(raw[0]);
            let (field, value) = word(line);
            if remaining > 0 {
                if !indented { return Err(ManifestError::Malformed); }
                let action = if equal_word(field, b"new") { FileAction::New }
                    else if equal_word(field, b"changed") { FileAction::Changed }
                    else if equal_word(field, b"deleted") { FileAction::Deleted }
                    else { return Err(ManifestError::Malformed); };
                let (decoded, rest) = match token(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
                if rest.len() != 0 { return Err(ManifestError::Malformed); }
                let path = match path(&decoded, &limits) { Ok(value) => value, Err(error) => return Err(error) };
                match &mut manifest.filelist {
                    Some(list) => list.push(FileChange { action, path }),
                    None => return Err(ManifestError::Malformed),
                }
                remaining -= 1;
                continue;
            }
            if !indented {
                if equal_word(field, b"Volume") || equal_word(field, b"Filelist") {
                    if let Some(record) = pending {
                        let record = match finish(record) { Ok(value) => value, Err(error) => return Err(error) };
                        manifest.records.push(record);
                    }
                    pending = None;
                    if equal_word(field, b"Volume") {
                        if filelist_closed { return Err(ManifestError::Malformed); }
                        if entry_count == limits.entries { return Err(ManifestError::Limit); }
                        entry_count += 1;
                        if value.len() < 2 || value[value.len() - 1] != b':' { return Err(ManifestError::Malformed); }
                        let number = match decimal(vstd::slice::slice_subrange(value, 0, value.len() - 1), true) {
                            Ok(value) => value, Err(error) => return Err(error),
                        };
                        pending = Some(PendingVolume { number, start: None, end: None, hashes: Vec::new() });
                    } else {
                        if manifest.filelist.is_some() { return Err(ManifestError::Malformed); }
                        remaining = match decimal(value, false) { Ok(value) => value, Err(error) => return Err(error) };
                        if remaining > (limits.entries - entry_count) as u64 { return Err(ManifestError::Limit); }
                        entry_count += remaining as usize;
                        filelist_closed = manifest.records.len() > 0;
                        manifest.filelist = Some(Vec::new());
                    }
                } else if equal_word(field, b"Hostname") {
                    if pending.is_some() || manifest.hostname.is_some() || value.len() == 0 { return Err(ManifestError::Malformed); }
                    manifest.hostname = Some(owned(value));
                } else if equal_word(field, b"Localdir") {
                    if pending.is_some() || manifest.localdir.is_some() { return Err(ManifestError::Malformed); }
                    let (decoded, rest) = match token(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
                    if rest.len() != 0 { return Err(ManifestError::Malformed); }
                    manifest.localdir = Some(decoded);
                } else { return Err(ManifestError::Malformed); }
            } else {
                let record = match &mut pending { Some(value) => value, None => return Err(ManifestError::Malformed) };
                if equal_word(field, b"StartingPath") || equal_word(field, b"EndingPath") {
                    let boundary = match boundary(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
                    if equal_word(field, b"StartingPath") {
                        if record.start.is_some() { return Err(ManifestError::Malformed); }
                        record.start = Some(boundary);
                    } else {
                        if record.end.is_some() { return Err(ManifestError::Malformed); }
                        record.end = Some(boundary);
                    }
                } else if equal_word(field, b"Hash") {
                    let hash = match digest(value) { Ok(value) => value, Err(error) => return Err(error) };
                    let mut i = 0;
                    let mut duplicate = false;
                    while i < record.hashes.len() {
                        if equal_word(&record.hashes[i].name, &hash.name) {
                            if record.hashes[i].hex != hash.hex { return Err(ManifestError::Malformed); }
                            duplicate = true;
                        }
                        i += 1;
                    }
                    if !duplicate {
                        if entry_count == limits.entries { return Err(ManifestError::Limit); }
                        entry_count += 1;
                        record.hashes.push(hash);
                    }
                } else { return Err(ManifestError::Malformed); }
            }
        }
        if remaining != 0 { return Err(ManifestError::Malformed); }
        if let Some(record) = pending {
            let record = match finish(record) { Ok(value) => value, Err(error) => return Err(error) };
            manifest.records.push(record);
        }
        if manifest.records.len() == 0 { return Err(ManifestError::Malformed); }
        Ok(manifest)
    }

    /// Ordered by volume number, last declaration wins, capped by final number.
    //= docs/parsing-requirements.md#parse-man-004
    //# Manifest interpretation MUST replace earlier records of a repeated volume number with its last record and discard volume records above the last recorded volume number.
    pub fn volumes(&self) -> Vec<&VolumeRecord> {
        let mut result: Vec<&VolumeRecord> = Vec::new();
        if self.records.len() == 0 { return result; }
        let last = self.records[self.records.len() - 1].number;
        let mut i = 0;
        while i < self.records.len() {
            let record = &self.records[i];
            if record.number <= last {
                let mut j = 0;
                while j < result.len() && result[j].number < record.number { j += 1; }
                if j < result.len() && result[j].number == record.number { result[j] = record; }
                else { result.insert(j, record); }
            }
            i += 1;
        }
        result
    }
}

fn descendant(path: &BackupPath, root: &BackupPath) -> bool {
    if path.components().len() < root.components().len() { return false; }
    let mut i = 0;
    while i < root.components().len() {
        if path.components()[i] != root.components()[i] { return false; }
        i += 1;
    }
    true
}

impl VolumeRecord {
    //= docs/parsing-requirements.md#parse-man-005
    //# Volume selection MUST include every volume whose declared inclusive path interval can contain a selected entry or descendant, including shared multipart boundary paths.
    pub fn may_contain(&self, selected: &BackupPath) -> bool {
        // Public records can be caller-constructed. Invalid ranges select all.
        if self.start.path.compare(&self.end.path) > 0 { return true; }
        (self.start.path.compare(selected) <= 0 && self.end.path.compare(selected) >= 0)
            || descendant(&self.start.path, selected) || descendant(&self.end.path, selected)
    }
}
}

impl Default for ManifestLimits {
    fn default() -> Self {
        Self {
            metadata_bytes: 256 * 1024 * 1024,
            line_bytes: 4 * 1024 * 1024 + 128,
            entries: 10_000_000,
            path_bytes: 1024 * 1024,
            path_components: 1024,
        }
    }
}
impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "manifest: {self:?}")
    }
}
impl std::error::Error for ManifestError {}
