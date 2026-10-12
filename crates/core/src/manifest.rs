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

pub open spec fn spacing(byte: u8) -> bool { byte == 32 || byte == 9 }
pub open spec fn lowercase(byte: u8) -> u8 {
    if 65 <= byte <= 90 { (byte as int + 32) as u8 } else { byte }
}
pub open spec fn start_nonspace(bytes: Seq<u8>, index: int) -> int
    decreases bytes.len() - index,
{
    if 0 <= index < bytes.len() && spacing(bytes[index]) {
        start_nonspace(bytes, index + 1)
    } else { index }
}
pub open spec fn end_nonspace(bytes: Seq<u8>, start: int, end: int) -> int
    decreases end - start,
{
    if 0 <= start < end <= bytes.len() && spacing(bytes[end - 1]) {
        end_nonspace(bytes, start, end - 1)
    } else { end }
}
pub open spec fn trimmed(bytes: Seq<u8>) -> Seq<u8> {
    let start = start_nonspace(bytes, 0);
    bytes.subrange(start, end_nonspace(bytes, start, bytes.len() as int))
}
pub open spec fn word_end(bytes: Seq<u8>, index: int) -> int
    decreases bytes.len() - index,
{
    if 0 <= index < bytes.len() && !spacing(bytes[index]) { word_end(bytes, index + 1) }
    else { index }
}
pub open spec fn word_model(bytes: Seq<u8>) -> (Seq<u8>, Seq<u8>) {
    let line = trimmed(bytes);
    let end = word_end(line, 0);
    (line.take(end), trimmed(line.skip(end)))
}
/// Decimal grammar uses mathematical integers; overflow is an explicit
/// representability check, independently of the executable division guard.
pub open spec fn decimal_scan(bytes: Seq<u8>, index: int, value: int, positive: bool) -> Result<u64, ManifestError>
    decreases bytes.len() - index,
{
    if !(0 <= index <= bytes.len() && 0 <= value <= u64::MAX) { Err(ManifestError::Malformed) }
    else if index == bytes.len() {
        if positive && value == 0 { Err(ManifestError::Malformed) } else { Ok(value as u64) }
    } else if bytes[index] < 48 || bytes[index] > 57 { Err(ManifestError::Malformed) }
    else {
        let next = value * 10 + bytes[index] as int - 48;
        if next > u64::MAX { Err(ManifestError::Malformed) }
        else { decimal_scan(bytes, index + 1, next, positive) }
    }
}
pub open spec fn decimal_model(bytes: Seq<u8>, positive: bool) -> Result<u64, ManifestError> {
    if bytes.len() == 0 { Err(ManifestError::Malformed) } else { decimal_scan(bytes, 0, 0, positive) }
}
pub open spec fn manifest_token_model(bytes: Seq<u8>, max_encoded: int)
    -> Result<(Seq<u8>, Seq<u8>), ManifestError> {
    if bytes.len() > max_encoded { Err(ManifestError::Limit) }
    else {
        match crate::verified::token_refinement::wire_token(bytes) {
            None => Err(ManifestError::Malformed),
            Some((n, decoded)) => {
                if n < bytes.len() && !spacing(bytes[n]) { Err(ManifestError::Malformed) }
                else { Ok((decoded, trimmed(bytes.skip(n)))) }
            },
        }
    }
}

fn space(byte: u8) -> (result: bool)
    ensures result == spacing(byte),
{ byte == b' ' || byte == b'\t' }
fn lower(byte: u8) -> (result: u8)
    ensures result == lowercase(byte),
{
    if byte >= b'A' && byte <= b'Z' { byte + 32 } else { byte }
}
fn equal_word(left: &[u8], right: &[u8]) -> (result: bool)
    ensures result == (left.len() == right.len()
        && forall|i: int| 0 <= i < left.len() ==> lowercase(left@[i]) == lowercase(right@[i])),
{
    if left.len() != right.len() { return false; }
    let mut i = 0;
    while i < left.len()
        invariant
            left.len() == right.len(),
            i <= left.len(),
            forall|j: int| 0 <= j < i ==> lowercase(left@[j]) == lowercase(right@[j]),
        decreases left.len() - i,
    {
        if lower(left[i]) != lower(right[i]) { return false; }
        i += 1;
    }
    true
}
fn owned(bytes: &[u8]) -> (result: Vec<u8>)
    ensures result@ == bytes@,
{
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(), result@ == bytes@.take(i as int),
        decreases bytes.len() - i,
    {
        result.push(bytes[i]);
        assert(result@ =~= bytes@.take(i as int + 1));
        i += 1;
    }
    result
}
fn trim(bytes: &[u8]) -> (result: &[u8])
    ensures result@ == trimmed(bytes@),
{
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && space(bytes[start])
        invariant start <= end == bytes.len(),
            start_nonspace(bytes@, 0) == start_nonspace(bytes@, start as int),
        decreases end - start,
    {
        reveal_with_fuel(start_nonspace, 1);
        start += 1;
    }
    proof { reveal_with_fuel(start_nonspace, 1); }
    while start < end && space(bytes[end - 1])
        invariant start <= end <= bytes.len(),
            start == start_nonspace(bytes@, 0),
            end_nonspace(bytes@, start as int, bytes.len() as int) == end_nonspace(bytes@, start as int, end as int),
        decreases end - start,
    {
        reveal_with_fuel(end_nonspace, 1);
        end -= 1;
    }
    proof { reveal_with_fuel(end_nonspace, 1); }
    vstd::slice::slice_subrange(bytes, start, end)
}
fn word(bytes: &[u8]) -> (result: (&[u8], &[u8]))
    ensures (result.0@, result.1@) == word_model(bytes@),
{
    let ghost original = bytes@;
    let bytes = trim(bytes);
    let mut end = 0;
    while end < bytes.len() && !space(bytes[end])
        invariant end <= bytes.len(), bytes@ == trimmed(original),
            word_end(bytes@, 0) == word_end(bytes@, end as int),
        decreases bytes.len() - end,
    {
        reveal_with_fuel(word_end, 1);
        end += 1;
    }
    proof { reveal_with_fuel(word_end, 1); }
    (vstd::slice::slice_subrange(bytes, 0, end), trim(vstd::slice::slice_subrange(bytes, end, bytes.len())))
}
fn decimal(bytes: &[u8], positive: bool) -> (result: Result<u64, ManifestError>)
    ensures result == decimal_model(bytes@, positive),
        match result { Ok(number) => !positive || number > 0, Err(_) => true },
{
    if bytes.len() == 0 { return Err(ManifestError::Malformed); }
    let mut number: u64 = 0;
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(),
            decimal_model(bytes@, positive) == decimal_scan(bytes@, i as int, number as int, positive),
        decreases bytes.len() - i,
    {
        proof { reveal_with_fuel(decimal_scan, 1); }
        if bytes[i] < b'0' || bytes[i] > b'9' { return Err(ManifestError::Malformed); }
        let digit = (bytes[i] - b'0') as u64;
        if number > (u64::MAX - digit) / 10 { return Err(ManifestError::Malformed); }
        number = number * 10 + digit;
        i += 1;
    }
    proof { reveal_with_fuel(decimal_scan, 1); }
    if positive && number == 0 { return Err(ManifestError::Malformed); }
    Ok(number)
}
fn token<'a>(bytes: &'a [u8], limits: &ManifestLimits) -> (result: Result<(Vec<u8>, &'a [u8]), ManifestError>)
    ensures match result {
        Ok((decoded, rest)) => manifest_token_model(bytes@, limits.line_bytes as int) == Ok((decoded@, rest@)),
        Err(error) => manifest_token_model(bytes@, limits.line_bytes as int) == Err(error),
    },
{
    let (consumed, decoded) = match decode_token(bytes, limits.line_bytes) {
        Ok(value) => value,
        Err(TokenError::Limit) => return Err(ManifestError::Limit),
        Err(_) => return Err(ManifestError::Malformed),
    };
    if consumed < bytes.len() && !space(bytes[consumed]) { return Err(ManifestError::Malformed); }
    Ok((decoded, trim(vstd::slice::slice_subrange(bytes, consumed, bytes.len()))))
}
pub open spec fn manifest_path_model(bytes: Seq<u8>, limits: ManifestLimits)
    -> Result<Seq<Seq<u8>>, ManifestError> {
    match crate::path::path_model(bytes, false, limits.path_bytes as int, limits.path_components as int) {
        Ok(value) => Ok(value),
        Err(crate::path::PathError::ByteLimit) | Err(crate::path::PathError::ComponentLimit)
            => Err(ManifestError::Limit),
        Err(_) => Err(ManifestError::Malformed),
    }
}
fn path(bytes: &[u8], limits: &ManifestLimits) -> (result: Result<BackupPath, ManifestError>)
    ensures match result {
        Ok(value) => manifest_path_model(bytes@, *limits) == Ok(value@),
        Err(error) => manifest_path_model(bytes@, *limits) == Err(error),
    },
{
    match BackupPath::parse(bytes, false, limits.path_bytes, limits.path_components) {
        Ok(value) => Ok(value),
        Err(crate::path::PathError::ByteLimit) | Err(crate::path::PathError::ComponentLimit) => Err(ManifestError::Limit),
        Err(_) => Err(ManifestError::Malformed),
    }
}
pub open spec fn boundary_model(bytes: Seq<u8>, limits: ManifestLimits)
    -> Result<(Seq<Seq<u8>>, Option<u64>), ManifestError> {
    match manifest_token_model(bytes, limits.line_bytes as int) {
        Err(error) => Err(error),
        Ok((decoded, rest)) => match manifest_path_model(decoded, limits) {
            Err(error) => Err(error),
            Ok(path) => {
                if rest.len() == 0 { Ok((path, None)) }
                else {
                    match decimal_model(rest, true) {
                        Ok(block) => Ok((path, Some(block))),
                        Err(error) => Err(error),
                    }
                }
            },
        },
    }
}
fn boundary(bytes: &[u8], limits: &ManifestLimits) -> (result: Result<Boundary, ManifestError>)
    ensures match result {
        Ok(value) => boundary_model(bytes@, *limits) == Ok((value.path@, value.block))
            && (match value.block { None => true, Some(number) => number > 0 }),
        Err(error) => boundary_model(bytes@, *limits) == Err(error),
    },
{
    let (decoded, rest) = match token(bytes, limits) { Ok(value) => value, Err(error) => return Err(error) };
    let path = match path(&decoded, limits) { Ok(value) => value, Err(error) => return Err(error) };
    let block = if rest.len() == 0 { None } else {
        match decimal(rest, true) { Ok(value) => Some(value), Err(error) => return Err(error) }
    };
    Ok(Boundary { path, block })
}
pub open spec fn normalized(bytes: Seq<u8>) -> Seq<u8> {
    bytes.map_values(|byte: u8| lowercase(byte))
}
pub open spec fn ascii_equal(left: Seq<u8>, right: Seq<u8>) -> bool {
    left.len() == right.len() && forall|i: int| 0 <= i < left.len()
        ==> lowercase(left[i]) == lowercase(right[i])
}
pub open spec fn digest_algorithm(name: Seq<u8>) -> DigestAlgorithm {
    if ascii_equal(name, seq![83u8, 72u8, 65u8, 49u8]) { DigestAlgorithm::Sha1 }
    else if ascii_equal(name, seq![77u8, 68u8, 53u8]) { DigestAlgorithm::Md5 }
    else { DigestAlgorithm::Unsupported }
}
pub open spec fn hexadecimal(byte: u8) -> bool {
    48 <= lowercase(byte) <= 57 || 97 <= lowercase(byte) <= 102
}
pub open spec fn digest_valid(bytes: Seq<u8>) -> bool {
    let (name, hex) = word_model(bytes);
    name.len() > 0 && hex.len() > 0
        && (digest_algorithm(name) != DigestAlgorithm::Sha1 || hex.len() == 40)
        && (digest_algorithm(name) != DigestAlgorithm::Md5 || hex.len() == 32)
        && forall|i: int| 0 <= i < hex.len() ==> hexadecimal(hex[i])
}
// Explicit arrays expose each grammar byte to the checked equality contract.
#[allow(clippy::byte_char_slices)]
fn digest(bytes: &[u8]) -> (result: Result<DeclaredDigest, ManifestError>)
    ensures match result {
        Ok(value) => digest_valid(bytes@)
            && value.algorithm == digest_algorithm(word_model(bytes@).0)
            && value.name@ == normalized(word_model(bytes@).0)
            && value.hex@ == normalized(word_model(bytes@).1),
        Err(error) => error == ManifestError::Malformed && !digest_valid(bytes@),
    },
{
    let (name, hex) = word(bytes);
    if name.len() == 0 || hex.len() == 0 { return Err(ManifestError::Malformed); }
    let algorithm = if equal_word(name, &[b'S', b'H', b'A', b'1']) { DigestAlgorithm::Sha1 }
        else if equal_word(name, &[b'M', b'D', b'5']) { DigestAlgorithm::Md5 } else { DigestAlgorithm::Unsupported };
    assert(algorithm == digest_algorithm(name@));
    let wrong_length = match algorithm {
        DigestAlgorithm::Sha1 => hex.len() != 40,
        DigestAlgorithm::Md5 => hex.len() != 32,
        DigestAlgorithm::Unsupported => false,
    };
    if wrong_length { return Err(ManifestError::Malformed); }
    let mut normalized_name = Vec::new();
    let mut i = 0;
    while i < name.len()
        invariant i <= name.len(), normalized_name@ == normalized(name@.take(i as int)),
            (name@, hex@) == word_model(bytes@),
            name.len() > 0, hex.len() > 0,
            algorithm == digest_algorithm(name@),
            algorithm == DigestAlgorithm::Sha1 ==> hex.len() == 40,
            algorithm == DigestAlgorithm::Md5 ==> hex.len() == 32,
        decreases name.len() - i,
    {
        normalized_name.push(lower(name[i]));
        assert(normalized_name@ =~= normalized(name@.take(i as int + 1)));
        i += 1;
    }
    let mut normalized_hex = Vec::new();
    i = 0;
    while i < hex.len()
        invariant i <= hex.len(),
            (name@, hex@) == word_model(bytes@),
            name.len() > 0, hex.len() > 0,
            algorithm == digest_algorithm(name@),
            algorithm == DigestAlgorithm::Sha1 ==> hex.len() == 40,
            algorithm == DigestAlgorithm::Md5 ==> hex.len() == 32,
            normalized_name@ == normalized(name@),
            normalized_hex@ == normalized(hex@.take(i as int)),
            forall|j: int| 0 <= j < i ==> hexadecimal(hex@[j]),
        decreases hex.len() - i,
    {
        let byte = lower(hex[i]);
        if !((byte >= b'0' && byte <= b'9') || (byte >= b'a' && byte <= b'f')) {
            return Err(ManifestError::Malformed);
        }
        normalized_hex.push(byte);
        assert(normalized_hex@ =~= normalized(hex@.take(i as int + 1)));
        i += 1;
    }
    Ok(DeclaredDigest { algorithm, name: normalized_name, hex: normalized_hex })
}
pub open spec fn digest_names_equal(left: DeclaredDigest, right: DeclaredDigest) -> bool {
    ascii_equal(left.name@, right.name@)
}
pub open spec fn digest_conflict(hashes: Seq<DeclaredDigest>, hash: DeclaredDigest) -> bool {
    exists|i: int| 0 <= i < hashes.len() && digest_names_equal(hashes[i], hash)
        && hashes[i].hex@ != hash.hex@
}
pub open spec fn digest_present(hashes: Seq<DeclaredDigest>, hash: DeclaredDigest) -> bool {
    exists|i: int| 0 <= i < hashes.len() && digest_names_equal(hashes[i], hash)
}
fn digest_duplicate(hashes: &[DeclaredDigest], hash: &DeclaredDigest)
    -> (result: Result<bool, ManifestError>)
    ensures match result {
        Err(error) => error == ManifestError::Malformed && digest_conflict(hashes@, *hash),
        Ok(duplicate) => !digest_conflict(hashes@, *hash) && duplicate == digest_present(hashes@, *hash),
    },
{
    let mut i = 0;
    let mut duplicate = false;
    while i < hashes.len()
        invariant i <= hashes.len(),
            duplicate == (exists|j: int| 0 <= j < i && digest_names_equal(hashes@[j], *hash)),
            forall|j: int| 0 <= j < i && digest_names_equal(hashes@[j], *hash)
                ==> hashes@[j].hex@ == hash.hex@,
        decreases hashes.len() - i,
    {
        if equal_word(&hashes[i].name, &hash.name) {
            if hashes[i].hex != hash.hex {
                assert(hashes@[i as int].hex@ != hash.hex@);
                return Err(ManifestError::Malformed);
            }
            duplicate = true;
        }
        i += 1;
    }
    Ok(duplicate)
}
spec fn finish_model(pending: PendingVolume) -> Result<VolumeRecord, ManifestError> {
    match (pending.start, pending.end) {
        (Some(start), Some(end)) => {
            if crate::path::components_less(end.path@, start.path@) || pending.hashes@.len() == 0 {
                Err(ManifestError::Malformed)
            } else if !(exists|i: int| 0 <= i < pending.hashes@.len()
                && pending.hashes@[i].algorithm != DigestAlgorithm::Unsupported) {
                Err(ManifestError::UnsupportedIntegrity)
            } else { Ok(VolumeRecord { number: pending.number, start, end, hashes: pending.hashes }) }
        },
        _ => Err(ManifestError::Malformed),
    }
}
fn finish(pending: PendingVolume) -> (result: Result<VolumeRecord, ManifestError>)
    ensures result == finish_model(pending),
{
    let start = match pending.start { Some(value) => value, None => return Err(ManifestError::Malformed) };
    let end = match pending.end { Some(value) => value, None => return Err(ManifestError::Malformed) };
    if !path_before_or_equal(&start.path, &end.path) { return Err(ManifestError::Malformed); }
    if pending.hashes.len() == 0 { return Err(ManifestError::Malformed); }
    let mut supported = false;
    let mut i = 0;
    while i < pending.hashes.len()
        invariant i <= pending.hashes.len(),
            pending.start == Some(start), pending.end == Some(end),
            !crate::path::components_less(end.path@, start.path@), pending.hashes.len() > 0,
            supported == (exists|j: int| 0 <= j < i
                && pending.hashes@[j].algorithm != DigestAlgorithm::Unsupported),
        decreases pending.hashes.len() - i,
    {
        if !matches!(pending.hashes[i].algorithm, DigestAlgorithm::Unsupported) { supported = true; }
        i += 1;
    }
    if !supported { return Err(ManifestError::UnsupportedIntegrity); }
    Ok(VolumeRecord { number: pending.number, start, end, hashes: pending.hashes })
}

pub open spec fn record_values(records: Seq<&VolumeRecord>) -> Seq<VolumeRecord> {
    records.map_values(|record: &VolumeRecord| *record)
}
pub closed spec fn insertion_index(records: Seq<VolumeRecord>, number: u64, index: int) -> int
    decreases records.len() - index,
{
    if 0 <= index < records.len() && records[index].number < number {
        insertion_index(records, number, index + 1)
    } else { index }
}
pub open spec fn insert_record(records: Seq<VolumeRecord>, record: VolumeRecord) -> Seq<VolumeRecord> {
    let index = insertion_index(records, record.number, 0);
    if index < records.len() && records[index].number == record.number {
        records.update(index, record)
    } else { records.insert(index, record) }
}
pub open spec fn normalized_prefix(records: Seq<VolumeRecord>, end: int, cap: u64) -> Seq<VolumeRecord>
    decreases end,
{
    if end <= 0 || end > records.len() { Seq::empty() }
    else {
        let previous = normalized_prefix(records, end - 1, cap);
        if records[end - 1].number <= cap { insert_record(previous, records[end - 1]) }
        else { previous }
    }
}
pub open spec fn normalized_records(records: Seq<VolumeRecord>) -> Seq<VolumeRecord> {
    if records.len() == 0 { Seq::empty() }
    else { normalized_prefix(records, records.len() as int, records.last().number) }
}
pub open spec fn ordered_records(records: Seq<VolumeRecord>) -> bool {
    forall|i: int, j: int| 0 <= i < j < records.len() ==> records[i].number < records[j].number
}
proof fn lemma_insertion_index(records: Seq<VolumeRecord>, number: u64, start: int)
    requires 0 <= start <= records.len(),
    ensures
        start <= insertion_index(records, number, start) <= records.len(),
        forall|i: int| start <= i < insertion_index(records, number, start)
            ==> records[i].number < number,
        insertion_index(records, number, start) < records.len()
            ==> records[insertion_index(records, number, start)].number >= number,
    decreases records.len() - start,
{
    reveal_with_fuel(insertion_index, 1);
    if start < records.len() && records[start].number < number {
        lemma_insertion_index(records, number, start + 1);
    }
}
proof fn lemma_insert_ordered(records: Seq<VolumeRecord>, record: VolumeRecord)
    requires ordered_records(records),
    ensures ordered_records(insert_record(records, record)),
{
    lemma_insertion_index(records, record.number, 0);
    let index = insertion_index(records, record.number, 0);
    let output = insert_record(records, record);
    assert forall|i: int, j: int| 0 <= i < j < output.len()
        implies output[i].number < output[j].number by {
        if index < records.len() && records[index].number == record.number {
            assert(output[i].number == records[i].number);
            assert(output[j].number == records[j].number);
        } else {
            if i < index && j < index {
                assert(output[i] == records[i]);
                assert(output[j] == records[j]);
            } else if i < index && j == index {
                assert(output[i] == records[i]);
            } else if i < index && j > index {
                assert(output[i] == records[i]);
                assert(output[j] == records[j - 1]);
            } else if i == index {
                assert(output[j] == records[j - 1]);
                if j - 1 > index { assert(records[index].number < records[j - 1].number); }
            } else {
                assert(output[i] == records[i - 1]);
                assert(output[j] == records[j - 1]);
            }
        }
    }
}
proof fn lemma_normalized_ordered(records: Seq<VolumeRecord>, end: int, cap: u64)
    requires 0 <= end <= records.len(),
    ensures ordered_records(normalized_prefix(records, end, cap)),
    decreases end,
{
    reveal_with_fuel(normalized_prefix, 1);
    if end > 0 {
        lemma_normalized_ordered(records, end - 1, cap);
        if records[end - 1].number <= cap {
            lemma_insert_ordered(normalized_prefix(records, end - 1, cap), records[end - 1]);
        }
    }
}
proof fn lemma_insert_contains(records: Seq<VolumeRecord>, record: VolumeRecord)
    requires ordered_records(records),
    ensures exists|i: int| 0 <= i < insert_record(records, record).len()
        && insert_record(records, record)[i] == record,
{
    lemma_insertion_index(records, record.number, 0);
    let index = insertion_index(records, record.number, 0);
    let output = insert_record(records, record);
    let replacement = index < records.len() && records[index].number == record.number;
    assert(0 <= index < output.len() && output[index] == record);
}
proof fn lemma_insert_origin(records: Seq<VolumeRecord>, record: VolumeRecord)
    requires ordered_records(records),
    ensures forall|i: int| 0 <= i < insert_record(records, record).len() ==>
        (if insert_record(records, record)[i].number == record.number {
            insert_record(records, record)[i] == record
        } else { exists|j: int| 0 <= j < records.len()
            && records[j] == insert_record(records, record)[i] }),
{
    lemma_insertion_index(records, record.number, 0);
    let index = insertion_index(records, record.number, 0);
    let output = insert_record(records, record);
    let replacement = index < records.len() && records[index].number == record.number;
    assert(0 <= index < output.len() && output[index] == record);
    assert forall|i: int| 0 <= i < output.len() implies
        (if output[i].number == record.number { output[i] == record }
        else { exists|j: int| 0 <= j < records.len() && records[j] == output[i] }) by {
        if i != index {
            let old_index = if replacement || i < index { i } else { i - 1 };
            assert(0 <= old_index < records.len());
            assert(output[i] == records[old_index]);
            if old_index < index { assert(records[old_index].number < record.number); }
            else if replacement { assert(record.number < records[old_index].number); }
            else {
                if old_index == index { assert(records[index].number != record.number); }
                else { assert(records[index].number < records[old_index].number); }
            }
            assert(output[i].number != record.number);
        }
    }
}
proof fn lemma_insert_preserves(records: Seq<VolumeRecord>, record: VolumeRecord)
    requires ordered_records(records),
    ensures forall|j: int| 0 <= j < records.len() && records[j].number != record.number ==>
        exists|i: int| 0 <= i < insert_record(records, record).len()
            && insert_record(records, record)[i] == records[j],
{
    lemma_insertion_index(records, record.number, 0);
    let index = insertion_index(records, record.number, 0);
    let output = insert_record(records, record);
    let replacement = index < records.len() && records[index].number == record.number;
    assert(0 <= index < output.len() && output[index] == record);
    assert forall|j: int| 0 <= j < records.len() && records[j].number != record.number
        implies exists|i: int| 0 <= i < output.len() && output[i] == records[j] by {
        let new_index = if replacement || j < index { j } else { j + 1 };
        assert(0 <= new_index < output.len() && output[new_index] == records[j]);
    }
}
proof fn lemma_insert_members(records: Seq<VolumeRecord>, record: VolumeRecord)
    requires ordered_records(records),
    ensures
        exists|i: int| 0 <= i < insert_record(records, record).len()
            && insert_record(records, record)[i] == record,
        forall|i: int| 0 <= i < insert_record(records, record).len() ==>
            (if insert_record(records, record)[i].number == record.number {
                insert_record(records, record)[i] == record
            } else {
                exists|j: int| 0 <= j < records.len() && records[j] == insert_record(records, record)[i]
            }),
        forall|j: int| 0 <= j < records.len() && records[j].number != record.number ==>
            exists|i: int| 0 <= i < insert_record(records, record).len()
                && insert_record(records, record)[i] == records[j],
{
    lemma_insert_contains(records, record);
    lemma_insert_origin(records, record);
    lemma_insert_preserves(records, record);
}
pub closed spec fn latest_record(records: Seq<VolumeRecord>, end: int, record: VolumeRecord) -> bool {
    exists|j: int| 0 <= j < end && record == records[j]
        && forall|k: int| j < k < end ==> records[k].number != record.number
}
proof fn lemma_latest_new(records: Seq<VolumeRecord>, end: int)
    requires 0 < end <= records.len(),
    ensures latest_record(records, end, records[end - 1]),
{
    reveal(latest_record);
    assert(0 <= end - 1 < end);
}
proof fn lemma_latest_preserved(records: Seq<VolumeRecord>, end: int, record: VolumeRecord)
    requires 0 < end <= records.len(), latest_record(records, end - 1, record),
        record.number != records[end - 1].number,
    ensures latest_record(records, end, record),
{
    reveal(latest_record);
    let j = choose|j: int| 0 <= j < end - 1 && record == records[j]
        && forall|k: int| j < k < end - 1 ==> records[k].number != record.number;
    assert(forall|k: int| j < k < end ==> records[k].number != record.number);
}
pub open spec fn normalization_semantics(records: Seq<VolumeRecord>, end: int, cap: u64,
    output: Seq<VolumeRecord>) -> bool {
    &&& ordered_records(output)
    &&& forall|i: int| 0 <= i < output.len() ==> output[i].number <= cap
    &&& forall|i: int| 0 <= i < output.len() ==> #[trigger] latest_record(records, end, output[i])
    &&& forall|j: int| 0 <= j < end && records[j].number <= cap ==>
        exists|i: int| 0 <= i < output.len() && output[i].number == records[j].number
}
proof fn lemma_insert_capped(previous: Seq<VolumeRecord>, record: VolumeRecord, cap: u64)
    requires ordered_records(previous), record.number <= cap,
        forall|i: int| 0 <= i < previous.len() ==> previous[i].number <= cap,
    ensures forall|i: int| 0 <= i < insert_record(previous, record).len()
        ==> insert_record(previous, record)[i].number <= cap,
{
    lemma_insert_members(previous, record);
    let output = insert_record(previous, record);
    assert forall|i: int| 0 <= i < output.len() implies output[i].number <= cap by {
        if output[i].number != record.number {
            let j = choose|j: int| 0 <= j < previous.len() && previous[j] == output[i];
        }
    }
}
proof fn lemma_insert_latest(records: Seq<VolumeRecord>, end: int, previous: Seq<VolumeRecord>)
    requires 0 < end <= records.len(), ordered_records(previous),
        forall|i: int| 0 <= i < previous.len() ==> #[trigger] latest_record(records, end - 1, previous[i]),
    ensures forall|i: int| 0 <= i < insert_record(previous, records[end - 1]).len()
        ==> #[trigger] latest_record(records, end, insert_record(previous, records[end - 1])[i]),
{
    let record = records[end - 1];
    lemma_insert_members(previous, record);
    lemma_latest_new(records, end);
    let output = insert_record(previous, record);
    assert forall|i: int| 0 <= i < output.len() implies
        #[trigger] latest_record(records, end, output[i]) by {
        if output[i].number != record.number {
            let p = choose|p: int| 0 <= p < previous.len() && previous[p] == output[i];
            lemma_latest_preserved(records, end, previous[p]);
        }
    }
}
proof fn lemma_insert_covers(records: Seq<VolumeRecord>, end: int, cap: u64, previous: Seq<VolumeRecord>)
    requires 0 < end <= records.len(), ordered_records(previous),
        forall|j: int| 0 <= j < end - 1 && records[j].number <= cap ==>
            exists|i: int| 0 <= i < previous.len() && previous[i].number == records[j].number,
    ensures forall|j: int| 0 <= j < end && records[j].number <= cap ==>
        exists|i: int| 0 <= i < insert_record(previous, records[end - 1]).len()
            && #[trigger] insert_record(previous, records[end - 1])[i].number == records[j].number,
{
    let record = records[end - 1];
    lemma_insert_members(previous, record);
    let output = insert_record(previous, record);
    assert forall|j: int| 0 <= j < end && records[j].number <= cap implies
        exists|i: int| 0 <= i < output.len() && output[i].number == records[j].number by {
        if records[j].number != record.number {
            let p = choose|p: int| 0 <= p < previous.len() && previous[p].number == records[j].number;
        }
    }
}
proof fn lemma_skipped_latest(records: Seq<VolumeRecord>, end: int, cap: u64, previous: Seq<VolumeRecord>)
    requires 0 < end <= records.len(), records[end - 1].number > cap,
        forall|i: int| 0 <= i < previous.len() ==> previous[i].number <= cap,
        forall|i: int| 0 <= i < previous.len() ==> #[trigger] latest_record(records, end - 1, previous[i]),
    ensures forall|i: int| 0 <= i < previous.len() ==> #[trigger] latest_record(records, end, previous[i]),
{
    assert forall|i: int| 0 <= i < previous.len() implies
        #[trigger] latest_record(records, end, previous[i]) by {
        lemma_latest_preserved(records, end, previous[i]);
    }
}
proof fn lemma_advance_semantics(records: Seq<VolumeRecord>, end: int, cap: u64,
    previous: Seq<VolumeRecord>)
    requires 0 < end <= records.len(),
        normalization_semantics(records, end - 1, cap, previous),
    ensures normalization_semantics(records, end, cap,
        if records[end - 1].number <= cap { insert_record(previous, records[end - 1]) }
        else { previous }),
{
    let record = records[end - 1];
    if record.number <= cap {
        lemma_insert_ordered(previous, record);
        lemma_insert_capped(previous, record, cap);
        lemma_insert_latest(records, end, previous);
        lemma_insert_covers(records, end, cap, previous);
    } else {
        lemma_skipped_latest(records, end, cap, previous);
    }
}
proof fn lemma_normalized_semantics(records: Seq<VolumeRecord>, end: int, cap: u64)
    requires 0 <= end <= records.len(),
    ensures normalization_semantics(records, end, cap, normalized_prefix(records, end, cap)),
    decreases end,
{
    reveal_with_fuel(normalized_prefix, 1);
    if end > 0 {
        lemma_normalized_semantics(records, end - 1, cap);
        lemma_advance_semantics(records, end, cap, normalized_prefix(records, end - 1, cap));
    }
}

// Contextual model draft: not yet connected to Manifest::parse.
// These structures represent byte identities and ordered records, rather than
// executable Vec capacities or allocation identities.
pub ghost struct ManifestDigestValue {
    pub algorithm: DigestAlgorithm,
    pub name: Seq<u8>,
    pub hex: Seq<u8>,
}
pub ghost struct ManifestBoundaryValue {
    pub path: Seq<Seq<u8>>,
    pub block: Option<u64>,
}
pub ghost struct ManifestVolumeValue {
    pub number: u64,
    pub start: ManifestBoundaryValue,
    pub end: ManifestBoundaryValue,
    pub hashes: Seq<ManifestDigestValue>,
}
pub ghost struct ManifestChangeValue {
    pub action: FileAction,
    pub path: Seq<Seq<u8>>,
}
pub ghost struct ManifestValue {
    pub hostname: Option<Seq<u8>>,
    pub localdir: Option<Seq<u8>>,
    pub records: Seq<ManifestVolumeValue>,
    pub filelist: Option<Seq<ManifestChangeValue>>,
}
// Used only by erased contextual specifications.
#[allow(dead_code)]
ghost struct ManifestPendingValue {
    number: u64,
    start: Option<ManifestBoundaryValue>,
    end: Option<ManifestBoundaryValue>,
    hashes: Seq<ManifestDigestValue>,
}
// Used only by erased contextual specifications.
#[allow(dead_code)]
ghost struct ManifestStateValue {
    manifest: ManifestValue,
    pending: Option<ManifestPendingValue>,
    remaining: int,
    filelist_closed: bool,
    entries_reserved: int,
    metadata_reserved: int,
}
pub open spec fn digest_value(value: DeclaredDigest) -> ManifestDigestValue {
    ManifestDigestValue { algorithm: value.algorithm, name: value.name@, hex: value.hex@ }
}
pub open spec fn boundary_value(value: Boundary) -> ManifestBoundaryValue {
    ManifestBoundaryValue { path: value.path@, block: value.block }
}
pub open spec fn volume_value(value: VolumeRecord) -> ManifestVolumeValue {
    ManifestVolumeValue { number: value.number, start: boundary_value(value.start),
        end: boundary_value(value.end), hashes: value.hashes@.map_values(|item| digest_value(item)) }
}
pub open spec fn change_value(value: FileChange) -> ManifestChangeValue {
    ManifestChangeValue { action: value.action, path: value.path@ }
}
pub open spec fn manifest_value(value: Manifest) -> ManifestValue {
    ManifestValue {
        hostname: match value.hostname { None => None, Some(bytes) => Some(bytes@) },
        localdir: match value.localdir { None => None, Some(bytes) => Some(bytes@) },
        records: value.records@.map_values(|item| volume_value(item)),
        filelist: match value.filelist {
            None => None, Some(changes) => Some(changes@.map_values(|item| change_value(item))),
        },
    }
}
proof fn lemma_volume_map_push(records: Seq<VolumeRecord>, record: VolumeRecord)
    ensures records.push(record).map_values(|item| volume_value(item))
        == records.map_values(|item| volume_value(item)).push(volume_value(record)),
{
    assert(records.push(record).map_values(|item| volume_value(item)) =~=
        records.map_values(|item| volume_value(item)).push(volume_value(record)));
}
proof fn lemma_change_map_push(changes: Seq<FileChange>, change: FileChange)
    ensures changes.push(change).map_values(|item| change_value(item))
        == changes.map_values(|item| change_value(item)).push(change_value(change)),
{
    assert(changes.push(change).map_values(|item| change_value(item)) =~=
        changes.map_values(|item| change_value(item)).push(change_value(change)));
}
proof fn lemma_digest_map_push(hashes: Seq<DeclaredDigest>, hash: DeclaredDigest)
    ensures hashes.push(hash).map_values(|item| digest_value(item))
        == hashes.map_values(|item| digest_value(item)).push(digest_value(hash)),
{
    assert(hashes.push(hash).map_values(|item| digest_value(item)) =~=
        hashes.map_values(|item| digest_value(item)).push(digest_value(hash)));
}
spec fn pending_value(value: PendingVolume) -> ManifestPendingValue {
    ManifestPendingValue { number: value.number,
        start: match value.start { None => None, Some(boundary) => Some(boundary_value(boundary)) },
        end: match value.end { None => None, Some(boundary) => Some(boundary_value(boundary)) },
        hashes: value.hashes@.map_values(|item| digest_value(item)),
    }
}
spec fn state_value(manifest: Manifest, pending: Option<PendingVolume>, remaining: u64,
    closed: bool, entry_count: usize, metadata_reserved: usize) -> ManifestStateValue {
    ManifestStateValue { manifest: manifest_value(manifest),
        pending: match pending { None => None, Some(volume) => Some(pending_value(volume)) },
        remaining: remaining as int, filelist_closed: closed,
        entries_reserved: entry_count as int, metadata_reserved: metadata_reserved as int,
    }
}
spec fn initial_state() -> ManifestStateValue {
    ManifestStateValue {
        manifest: ManifestValue { hostname: None, localdir: None, records: Seq::empty(), filelist: None },
        pending: None, remaining: 0, filelist_closed: false,
        entries_reserved: 0, metadata_reserved: 0,
    }
}
spec fn digest_value_model(bytes: Seq<u8>) -> Result<ManifestDigestValue, ManifestError> {
    let (name, hex) = word_model(bytes);
    if !digest_valid(bytes) { Err(ManifestError::Malformed) }
    else { Ok(ManifestDigestValue { algorithm: digest_algorithm(name),
        name: normalized(name), hex: normalized(hex) }) }
}
spec fn boundary_value_model(bytes: Seq<u8>, limits: ManifestLimits)
    -> Result<ManifestBoundaryValue, ManifestError> {
    match boundary_model(bytes, limits) {
        Err(error) => Err(error),
        Ok((path, block)) => Ok(ManifestBoundaryValue { path, block }),
    }
}
spec fn complete_value(value: ManifestPendingValue) -> Result<ManifestVolumeValue, ManifestError> {
    match (value.start, value.end) {
        (Some(start), Some(end)) => {
            if crate::path::components_less(end.path, start.path) || value.hashes.len() == 0 {
                Err(ManifestError::Malformed)
            } else if !(exists|i: int| 0 <= i < value.hashes.len()
                && value.hashes[i].algorithm != DigestAlgorithm::Unsupported) {
                Err(ManifestError::UnsupportedIntegrity)
            } else { Ok(ManifestVolumeValue { number: value.number, start, end, hashes: value.hashes }) }
        },
        _ => Err(ManifestError::Malformed),
    }
}
spec fn digest_values(hashes: Seq<DeclaredDigest>) -> Seq<ManifestDigestValue> {
    hashes.map_values(|item| digest_value(item))
}
proof fn lemma_digest_value_members(hashes: Seq<DeclaredDigest>, hash: DeclaredDigest)
    ensures
        digest_present(hashes, hash) == (exists|i: int| 0 <= i < hashes.len()
            && #[trigger] ascii_equal(digest_values(hashes)[i].name, digest_value(hash).name)),
        digest_conflict(hashes, hash) == (exists|i: int| 0 <= i < hashes.len()
            && #[trigger] ascii_equal(digest_values(hashes)[i].name, digest_value(hash).name)
            && digest_values(hashes)[i].hex != digest_value(hash).hex),
{
    let mapped = digest_values(hashes);
    assert forall|i: int| 0 <= i < hashes.len() implies
        (digest_names_equal(hashes[i], hash) == ascii_equal(mapped[i].name, digest_value(hash).name)
        && (hashes[i].hex@ != hash.hex@) == (mapped[i].hex != digest_value(hash).hex)) by {
        assert(mapped[i] == digest_value(hashes[i]));
    }
}
proof fn lemma_finish_value(pending: PendingVolume)
    ensures match finish_model(pending) {
        Ok(value) => complete_value(pending_value(pending)) == Ok(volume_value(value)),
        Err(error) => complete_value(pending_value(pending)) == Err(error),
    },
{
    let mapped = pending_value(pending).hashes;
    assert(mapped.len() == pending.hashes@.len());
    assert forall|i: int| 0 <= i < mapped.len() implies
        mapped[i].algorithm == pending.hashes@[i].algorithm by {
        assert(mapped[i] == digest_value(pending.hashes@[i]));
    }
    if exists|i: int| 0 <= i < pending.hashes@.len()
        && pending.hashes@[i].algorithm != DigestAlgorithm::Unsupported {
        let i = choose|i: int| 0 <= i < pending.hashes@.len()
            && pending.hashes@[i].algorithm != DigestAlgorithm::Unsupported;
        assert(mapped[i].algorithm != DigestAlgorithm::Unsupported);
    }
    if exists|i: int| 0 <= i < mapped.len()
        && mapped[i].algorithm != DigestAlgorithm::Unsupported {
        let i = choose|i: int| 0 <= i < mapped.len()
            && mapped[i].algorithm != DigestAlgorithm::Unsupported;
        assert(pending.hashes@[i].algorithm != DigestAlgorithm::Unsupported);
    }
}
spec fn flush_state(state: ManifestStateValue) -> Result<ManifestStateValue, ManifestError> {
    match state.pending {
        None => Ok(state),
        Some(volume) => match complete_value(volume) {
            Err(error) => Err(error),
            Ok(record) => Ok(ManifestStateValue {
                manifest: ManifestValue { records: state.manifest.records.push(record), ..state.manifest },
                pending: None, ..state
            }),
        },
    }
}
// Exact lexical predicate; opacity confines pointwise quantifiers to its checked adapter.
#[verifier::opaque]
spec fn keyword_matches(left: Seq<u8>, right: Seq<u8>) -> bool {
    ascii_equal(left, right)
}
fn manifest_keyword(left: &[u8], right: &[u8]) -> (result: bool)
    ensures result == keyword_matches(left@, right@),
{
    proof { reveal(keyword_matches); }
    equal_word(left, right)
}
spec fn interpret_line(state: ManifestStateValue, raw: Seq<u8>, limits: ManifestLimits)
    -> Result<ManifestStateValue, ManifestError> {
    let line = trimmed(raw);
    if line.len() == 0 { Ok(state) }
    else if state.metadata_reserved + 512 + 80 * line.len() > limits.metadata_bytes {
        Err(ManifestError::Limit)
    } else {
        let state = ManifestStateValue {
            metadata_reserved: state.metadata_reserved + 512 + 80 * line.len(), ..state
        };
        let indented = raw.len() > 0 && spacing(raw[0]);
        let (field, value) = word_model(line);
        if state.remaining > 0 {
            if !indented { Err(ManifestError::Malformed) }
            else {
                let action = if keyword_matches(field, seq![b'n', b'e', b'w']) { Some(FileAction::New) }
                    else if keyword_matches(field, seq![b'c', b'h', b'a', b'n', b'g', b'e', b'd']) { Some(FileAction::Changed) }
                    else if keyword_matches(field, seq![b'd', b'e', b'l', b'e', b't', b'e', b'd']) { Some(FileAction::Deleted) }
                    else { None };
                match action {
                    None => Err(ManifestError::Malformed),
                    Some(action) => match manifest_token_model(value, limits.line_bytes as int) {
                        Err(error) => Err(error),
                        Ok((decoded, rest)) => {
                            if rest.len() != 0 { Err(ManifestError::Malformed) }
                            else {
                                match manifest_path_model(decoded, limits) {
                                    Err(error) => Err(error),
                                    Ok(path) => match state.manifest.filelist {
                                        None => Err(ManifestError::Malformed),
                                        Some(list) => Ok(ManifestStateValue {
                                            manifest: ManifestValue {
                                                filelist: Some(list.push(ManifestChangeValue { action, path })),
                                                ..state.manifest
                                            },
                                            remaining: state.remaining - 1, ..state
                                        }),
                                    },
                                }
                            }
                        },
                    },
                }
            }
        } else if !indented {
            if keyword_matches(field, seq![b'V', b'o', b'l', b'u', b'm', b'e'])
                || keyword_matches(field, seq![b'F', b'i', b'l', b'e', b'l', b'i', b's', b't']) {
                match flush_state(state) {
                    Err(error) => Err(error),
                    Ok(state) => {
                        if keyword_matches(field, seq![b'V', b'o', b'l', b'u', b'm', b'e']) {
                            if state.filelist_closed { Err(ManifestError::Malformed) }
                            else if state.entries_reserved >= limits.entries { Err(ManifestError::Limit) }
                            else if value.len() < 2 || value[value.len() as int - 1] != b':' {
                                Err(ManifestError::Malformed)
                            } else {
                                match decimal_model(value.take(value.len() as int - 1), true) {
                                    Err(error) => Err(error),
                                    Ok(number) => Ok(ManifestStateValue {
                                        entries_reserved: state.entries_reserved + 1,
                                        pending: Some(ManifestPendingValue {
                                            number, start: None, end: None, hashes: Seq::empty(),
                                        }), ..state
                                    }),
                                }
                            }
                        } else {
                            match state.manifest.filelist {
                                Some(_) => Err(ManifestError::Malformed),
                                None => match decimal_model(value, false) {
                                    Err(error) => Err(error),
                                    Ok(count) => {
                                        if count as int > limits.entries as int - state.entries_reserved {
                                            Err(ManifestError::Limit)
                                        } else { Ok(ManifestStateValue {
                                            remaining: count as int,
                                            entries_reserved: state.entries_reserved + count as int,
                                            filelist_closed: state.manifest.records.len() > 0,
                                            manifest: ManifestValue { filelist: Some(Seq::empty()), ..state.manifest },
                                            ..state
                                        }) }
                                    },
                                },
                            }
                        }
                    },
                }
            } else if keyword_matches(field, seq![b'H', b'o', b's', b't', b'n', b'a', b'm', b'e']) {
                if state.pending.is_some() || state.manifest.hostname.is_some() || value.len() == 0 {
                    Err(ManifestError::Malformed)
                } else { Ok(ManifestStateValue {
                    manifest: ManifestValue { hostname: Some(value), ..state.manifest }, ..state
                }) }
            } else if keyword_matches(field, seq![b'L', b'o', b'c', b'a', b'l', b'd', b'i', b'r']) {
                if state.pending.is_some() || state.manifest.localdir.is_some() { Err(ManifestError::Malformed) }
                else {
                    match manifest_token_model(value, limits.line_bytes as int) {
                        Err(error) => Err(error),
                        Ok((decoded, rest)) => {
                            if rest.len() != 0 { Err(ManifestError::Malformed) }
                            else { Ok(ManifestStateValue {
                                manifest: ManifestValue { localdir: Some(decoded), ..state.manifest }, ..state
                            }) }
                        },
                    }
                }
            } else { Err(ManifestError::Malformed) }
        } else {
            match state.pending {
                None => Err(ManifestError::Malformed),
                Some(volume) => {
                    if keyword_matches(field, seq![b'S', b't', b'a', b'r', b't', b'i', b'n', b'g', b'P', b'a', b't', b'h'])
                        || keyword_matches(field, seq![b'E', b'n', b'd', b'i', b'n', b'g', b'P', b'a', b't', b'h']) {
                        match boundary_value_model(value, limits) {
                            Err(error) => Err(error),
                            Ok(boundary) => {
                                if keyword_matches(field, seq![b'S', b't', b'a', b'r', b't', b'i', b'n', b'g', b'P', b'a', b't', b'h']) {
                                    if volume.start.is_some() { Err(ManifestError::Malformed) }
                                    else { Ok(ManifestStateValue { pending: Some(ManifestPendingValue {
                                        start: Some(boundary), ..volume
                                    }), ..state }) }
                                } else {
                                    if volume.end.is_some() { Err(ManifestError::Malformed) }
                                    else { Ok(ManifestStateValue { pending: Some(ManifestPendingValue {
                                        end: Some(boundary), ..volume
                                    }), ..state }) }
                                }
                            },
                        }
                    } else if keyword_matches(field, seq![b'H', b'a', b's', b'h']) {
                        match digest_value_model(value) {
                            Err(error) => Err(error),
                            Ok(hash) => {
                                let conflict = exists|i: int| 0 <= i < volume.hashes.len()
                                    && ascii_equal(volume.hashes[i].name, hash.name)
                                    && volume.hashes[i].hex != hash.hex;
                                let duplicate = exists|i: int| 0 <= i < volume.hashes.len()
                                    && ascii_equal(volume.hashes[i].name, hash.name);
                                if conflict { Err(ManifestError::Malformed) }
                                else if duplicate { Ok(state) }
                                else if state.entries_reserved >= limits.entries { Err(ManifestError::Limit) }
                                else { Ok(ManifestStateValue {
                                    entries_reserved: state.entries_reserved + 1,
                                    pending: Some(ManifestPendingValue {
                                        hashes: volume.hashes.push(hash), ..volume
                                    }), ..state
                                }) }
                            },
                        }
                    } else { Err(ManifestError::Malformed) }
                },
            }
        }
    }
}
spec fn finish_state(state: ManifestStateValue) -> Result<ManifestValue, ManifestError> {
    if state.remaining != 0 { Err(ManifestError::Malformed) }
    else {
        match flush_state(state) {
            Err(error) => Err(error),
            Ok(state) => {
                if state.manifest.records.len() == 0 { Err(ManifestError::Malformed) }
                else { Ok(state.manifest) }
            },
        }
    }
}
spec fn line_ending(bytes: Seq<u8>, cursor: int) -> int
    decreases bytes.len() - cursor,
{
    if 0 <= cursor < bytes.len() && bytes[cursor] != b'\n' { line_ending(bytes, cursor + 1) }
    else { cursor }
}
proof fn lemma_line_ending_bounds(bytes: Seq<u8>, cursor: int)
    requires 0 <= cursor <= bytes.len(),
    ensures cursor <= line_ending(bytes, cursor) <= bytes.len(),
        line_ending(bytes, cursor) < bytes.len() ==> bytes[line_ending(bytes, cursor)] == b'\n',
        forall|i: int| cursor <= i < line_ending(bytes, cursor) ==> bytes[i] != b'\n',
    decreases bytes.len() - cursor,
{
    reveal_with_fuel(line_ending, 1);
    if cursor < bytes.len() && bytes[cursor] != b'\n' {
        lemma_line_ending_bounds(bytes, cursor + 1);
    }
}
#[verifier::opaque]
spec fn parse_state_model(bytes: Seq<u8>, cursor: int, state: ManifestStateValue, limits: ManifestLimits)
    -> Result<ManifestValue, ManifestError>
    decreases bytes.len() - cursor
    via parse_state_decreases
{
    if !(0 <= cursor <= bytes.len()) { Err(ManifestError::Malformed) }
    else if cursor == bytes.len() { finish_state(state) }
    else {
        let end = line_ending(bytes, cursor);
        if end - cursor > limits.line_bytes { Err(ManifestError::Limit) }
        else {
            let stripped = if end > cursor && bytes[end - 1] == b'\r' { end - 1 } else { end };
            let raw = bytes.subrange(cursor, stripped);
            match interpret_line(state, raw, limits) {
                Err(error) => Err(error),
                Ok(next) => parse_state_model(bytes, if end < bytes.len() { end + 1 } else { end }, next, limits),
            }
        }
    }
}
#[verifier::decreases_by]
proof fn parse_state_decreases(bytes: Seq<u8>, cursor: int, state: ManifestStateValue, limits: ManifestLimits) {
    if 0 <= cursor < bytes.len() { lemma_line_ending_bounds(bytes, cursor); }
}
pub closed spec fn manifest_model(bytes: Seq<u8>, limits: ManifestLimits) -> Result<ManifestValue, ManifestError> {
    if bytes.len() > limits.metadata_bytes { Err(ManifestError::Limit) }
    else { parse_state_model(bytes, 0, initial_state(), limits) }
}

struct ParseState {
    manifest: Manifest,
    pending: Option<PendingVolume>,
    remaining: u64,
    filelist_closed: bool,
    entry_count: usize,
    metadata_reserved: usize,
}
spec fn parse_value(state: ParseState) -> ManifestStateValue {
    state_value(state.manifest, state.pending, state.remaining, state.filelist_closed,
        state.entry_count, state.metadata_reserved)
}
// This adapter preserves the original branch and error precedence. It moves
// existing vectors through the state; no clone or new allocation is introduced.
#[allow(clippy::byte_char_slices)]
// Verus expands the source's shorthand mutable destructuring into explicit fields.
#[allow(non_shorthand_field_patterns)]
#[verifier::spinoff_prover]
#[verifier::rlimit(20)]
fn parse_line(raw: &[u8], line: &[u8], limits: ManifestLimits, state: ParseState)
    -> (result: Result<ParseState, ManifestError>)
    requires line@ == trimmed(raw@), line.len() > 0,
        state.entry_count <= limits.entries,
        state.metadata_reserved <= limits.metadata_bytes,
    ensures match result {
        Ok(next) => interpret_line(parse_value(state), raw@, limits) == Ok(parse_value(next))
            && next.entry_count <= limits.entries && next.metadata_reserved <= limits.metadata_bytes,
        Err(error) => interpret_line(parse_value(state), raw@, limits) == Err(error),
    },
{
    let ParseState { mut manifest, mut pending, mut remaining, mut filelist_closed,
        mut entry_count, mut metadata_reserved } = state;
    let ghost previous_state = state_value(manifest, pending, remaining, filelist_closed,
        entry_count, metadata_reserved);
    // Reserve before decoding: bytes in fields plus conservative vector,
    // record and worst-case byte-component allocation overhead. This
    // intentionally overestimates retained metadata rather than treating
    // serialized bytes as the size of an allocated tree of vectors.
    if metadata_reserved > limits.metadata_bytes
        || limits.metadata_bytes - metadata_reserved < 512 { return Err(ManifestError::Limit); }
    metadata_reserved += 512;
    if line.len() > (limits.metadata_bytes - metadata_reserved) / 80 { return Err(ManifestError::Limit); }
    metadata_reserved += line.len() * 80;
    let ghost charged_state = state_value(manifest, pending, remaining, filelist_closed,
        entry_count, metadata_reserved);
    assert(charged_state == ManifestStateValue {
        metadata_reserved: previous_state.metadata_reserved + 512 + 80 * line@.len(),
        ..previous_state
    });
    let indented = raw.len() > 0 && space(raw[0]);
    let (field, value) = word(line);
    if remaining > 0 {
        if !indented { return Err(ManifestError::Malformed); }
        let action = if manifest_keyword(field, &[b'n', b'e', b'w']) { FileAction::New }
            else if manifest_keyword(field, &[b'c', b'h', b'a', b'n', b'g', b'e', b'd']) { FileAction::Changed }
            else if manifest_keyword(field, &[b'd', b'e', b'l', b'e', b't', b'e', b'd']) { FileAction::Deleted }
            else { return Err(ManifestError::Malformed); };
        let (decoded, rest) = match token(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
        if rest.len() != 0 { return Err(ManifestError::Malformed); }
        let path = match path(&decoded, &limits) { Ok(value) => value, Err(error) => return Err(error) };
        match &mut manifest.filelist {
            Some(list) => {
                proof { lemma_change_map_push(list@, FileChange { action, path }); }
                list.push(FileChange { action, path });
            },
            None => return Err(ManifestError::Malformed),
        }
        remaining -= 1;
        assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
            pending, remaining, filelist_closed, entry_count, metadata_reserved)));
        return Ok(ParseState { manifest, pending, remaining, filelist_closed, entry_count, metadata_reserved });
    }
    if !indented {
        if manifest_keyword(field, &[b'V', b'o', b'l', b'u', b'm', b'e']) || manifest_keyword(field, &[b'F', b'i', b'l', b'e', b'l', b'i', b's', b't']) {
            if let Some(record) = pending {
                proof { lemma_finish_value(record); }
                let record = match finish(record) { Ok(value) => value, Err(error) => return Err(error) };
                proof { lemma_volume_map_push(manifest.records@, record); }
                manifest.records.push(record);
            }
            pending = None;
            assert(flush_state(charged_state) == Ok(state_value(manifest, pending,
                remaining, filelist_closed, entry_count, metadata_reserved)));
            if manifest_keyword(field, &[b'V', b'o', b'l', b'u', b'm', b'e']) {
                if filelist_closed { return Err(ManifestError::Malformed); }
                if entry_count == limits.entries { return Err(ManifestError::Limit); }
                entry_count += 1;
                if value.len() < 2 || value[value.len() - 1] != b':' { return Err(ManifestError::Malformed); }
                let number = match decimal(vstd::slice::slice_subrange(value, 0, value.len() - 1), true) {
                    Ok(value) => value, Err(error) => return Err(error),
                };
                pending = Some(PendingVolume { number, start: None, end: None, hashes: Vec::new() });
                assert(pending_value(pending.unwrap()).hashes =~= Seq::empty());
            assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
                pending, remaining, filelist_closed, entry_count, metadata_reserved)));
            } else {
                if manifest.filelist.is_some() { return Err(ManifestError::Malformed); }
                remaining = match decimal(value, false) { Ok(value) => value, Err(error) => return Err(error) };
                if remaining > (limits.entries - entry_count) as u64 { return Err(ManifestError::Limit); }
                entry_count += remaining as usize;
                filelist_closed = manifest.records.len() > 0;
                manifest.filelist = Some(Vec::new());
                assert(manifest.filelist.unwrap()@.map_values(|item| change_value(item)) =~= Seq::empty());
            assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
                pending, remaining, filelist_closed, entry_count, metadata_reserved)));
            }
        } else if manifest_keyword(field, &[b'H', b'o', b's', b't', b'n', b'a', b'm', b'e']) {
            if pending.is_some() || manifest.hostname.is_some() || value.len() == 0 { return Err(ManifestError::Malformed); }
            manifest.hostname = Some(owned(value));
            assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
                pending, remaining, filelist_closed, entry_count, metadata_reserved)));
        } else if manifest_keyword(field, &[b'L', b'o', b'c', b'a', b'l', b'd', b'i', b'r']) {
            if pending.is_some() || manifest.localdir.is_some() { return Err(ManifestError::Malformed); }
            let (decoded, rest) = match token(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
            if rest.len() != 0 { return Err(ManifestError::Malformed); }
            manifest.localdir = Some(decoded);
            assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
                pending, remaining, filelist_closed, entry_count, metadata_reserved)));
        } else { return Err(ManifestError::Malformed); }
    } else {
        let record = match &mut pending { Some(value) => value, None => return Err(ManifestError::Malformed) };
        if manifest_keyword(field, &[b'S', b't', b'a', b'r', b't', b'i', b'n', b'g', b'P', b'a', b't', b'h']) || manifest_keyword(field, &[b'E', b'n', b'd', b'i', b'n', b'g', b'P', b'a', b't', b'h']) {
            let boundary = match boundary(value, &limits) { Ok(value) => value, Err(error) => return Err(error) };
            if manifest_keyword(field, &[b'S', b't', b'a', b'r', b't', b'i', b'n', b'g', b'P', b'a', b't', b'h']) {
                if record.start.is_some() { return Err(ManifestError::Malformed); }
                record.start = Some(boundary);
            } else {
                if record.end.is_some() { return Err(ManifestError::Malformed); }
                record.end = Some(boundary);
            }
        } else if manifest_keyword(field, &[b'H', b'a', b's', b'h']) {
            let hash = match digest(value) { Ok(value) => value, Err(error) => return Err(error) };
            proof { lemma_digest_value_members(record.hashes@, hash); }
            let duplicate = match digest_duplicate(&record.hashes, &hash) {
                Ok(value) => value, Err(error) => return Err(error),
            };
            if !duplicate {
                if entry_count == limits.entries { return Err(ManifestError::Limit); }
                entry_count += 1;
                proof { lemma_digest_map_push(record.hashes@, hash); }
                record.hashes.push(hash);
            }
        } else { return Err(ManifestError::Malformed); }
    }
    assert(interpret_line(previous_state, raw@, limits) == Ok(state_value(manifest,
        pending, remaining, filelist_closed, entry_count, metadata_reserved)));
    Ok(ParseState { manifest, pending, remaining, filelist_closed, entry_count, metadata_reserved })
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
    // Explicit grammar bytes remain visible to the checked equality contract.
    #[allow(clippy::byte_char_slices)]
    #[verifier::spinoff_prover]
    #[verifier::rlimit(20)]
    pub fn parse(bytes: &[u8], limits: ManifestLimits) -> (result: Result<Self, ManifestError>)
        ensures match result {
            Ok(value) => manifest_model(bytes@, limits) == Ok(manifest_value(value)),
            Err(error) => manifest_model(bytes@, limits) == Err(error),
        },
    {
        proof { reveal(manifest_model); }
        if bytes.len() > limits.metadata_bytes { return Err(ManifestError::Limit); }
        let mut state = ParseState {
            manifest: Self { hostname: None, localdir: None, records: Vec::new(), filelist: None },
            pending: None, remaining: 0, filelist_closed: false,
            entry_count: 0, metadata_reserved: 0,
        };
        let mut cursor = 0;
        proof {
            let initial = state_value(state.manifest, state.pending, state.remaining, state.filelist_closed, state.entry_count, state.metadata_reserved);
            assert(initial.manifest.records =~= Seq::empty());
            assert(initial == initial_state());
        }
        while cursor < bytes.len()
            invariant cursor <= bytes.len(), state.entry_count <= limits.entries,
                state.metadata_reserved <= limits.metadata_bytes,
                manifest_model(bytes@, limits) == parse_state_model(bytes@, cursor as int,
                    state_value(state.manifest, state.pending, state.remaining, state.filelist_closed, state.entry_count, state.metadata_reserved), limits),
            decreases bytes.len() - cursor,
        {
            let start = cursor;
            let ghost previous_state = state_value(state.manifest, state.pending, state.remaining, state.filelist_closed,
                state.entry_count, state.metadata_reserved);
            while cursor < bytes.len() && bytes[cursor] != b'\n'
                invariant start <= cursor <= bytes.len(), cursor - start <= limits.line_bytes,
                    line_ending(bytes@, start as int) == line_ending(bytes@, cursor as int),
                    manifest_model(bytes@, limits) == parse_state_model(bytes@, start as int, previous_state, limits),
                decreases bytes.len() - cursor,
            {
                if cursor - start >= limits.line_bytes {
                    assert(manifest_model(bytes@, limits) == Err(ManifestError::Limit)) by {
                        reveal_with_fuel(parse_state_model, 1);
                        lemma_line_ending_bounds(bytes@, cursor as int + 1);
                        reveal_with_fuel(line_ending, 1);
                    }
                    return Err(ManifestError::Limit);
                }
                proof { reveal_with_fuel(line_ending, 1); }
                cursor += 1;
            }
            proof { reveal_with_fuel(line_ending, 1); }
            let ghost newline_end = cursor as int;
            assert(line_ending(bytes@, start as int) == newline_end);
            let mut end = cursor;
            if end > start && bytes[end - 1] == b'\r' { end -= 1; }
            let raw = vstd::slice::slice_subrange(bytes, start, end);
            if cursor < bytes.len() { cursor += 1; }
            let ghost next_cursor = cursor as int;
            let line = trim(raw);
            assert(manifest_model(bytes@, limits) == match interpret_line(previous_state, raw@, limits) {
                Err(error) => Err(error),
                Ok(next) => parse_state_model(bytes@, next_cursor, next, limits),
            }) by { reveal_with_fuel(parse_state_model, 1); }
            if line.len() == 0 { continue; }
            state = match parse_line(raw, line, limits, state) {
                Ok(next) => next, Err(error) => return Err(error),
            };
        }
        let ghost terminal_state = state_value(state.manifest, state.pending, state.remaining, state.filelist_closed,
            state.entry_count, state.metadata_reserved);
        assert(manifest_model(bytes@, limits) == finish_state(terminal_state))
            by { reveal_with_fuel(parse_state_model, 1); }
        if state.remaining != 0 { return Err(ManifestError::Malformed); }
        if let Some(record) = state.pending {
            proof { lemma_finish_value(record); }
            let record = match finish(record) { Ok(value) => value, Err(error) => return Err(error) };
            proof { lemma_volume_map_push(state.manifest.records@, record); }
                        state.manifest.records.push(record);
        }
        if state.manifest.records.len() == 0 { return Err(ManifestError::Malformed); }
        assert(finish_state(terminal_state) == Ok(manifest_value(state.manifest)));
        Ok(state.manifest)
    }

    /// Ordered by volume number, last declaration wins, capped by final number.
    //= docs/parsing-requirements.md#parse-man-004
    //# Manifest interpretation MUST replace earlier records of a repeated volume number with its last record and discard volume records above the last recorded volume number.
    pub fn volumes(&self) -> (result: Vec<&VolumeRecord>)
        ensures
            record_values(result@) == normalized_records(self.records@),
            self.records.len() > 0 ==> normalization_semantics(self.records@,
                self.records.len() as int, self.records@.last().number, record_values(result@)),
    {
        let mut result: Vec<&VolumeRecord> = Vec::new();
        if self.records.len() == 0 { return result; }
        let last = self.records[self.records.len() - 1].number;
        let mut i = 0;
        while i < self.records.len()
            invariant i <= self.records.len(), self.records.len() > 0,
                last == self.records@[self.records.len() - 1].number,
                record_values(result@) == normalized_prefix(self.records@, i as int, last),
            decreases self.records.len() - i,
        {
            proof { reveal_with_fuel(normalized_prefix, 1); }
            let record = &self.records[i];
            if record.number <= last {
                let mut j = 0;
                while j < result.len() && result[j].number < record.number
                    invariant j <= result.len(),
                        insertion_index(record_values(result@), record.number, 0)
                            == insertion_index(record_values(result@), record.number, j as int),
                    decreases result.len() - j,
                {
                    reveal_with_fuel(insertion_index, 1);
                    j += 1;
                }
                proof { reveal_with_fuel(insertion_index, 1); }
                let ghost previous = record_values(result@);
                if j < result.len() && result[j].number == record.number { result[j] = record; }
                else { result.insert(j, record); }
                assert(record_values(result@) =~= insert_record(previous, *record));
            }
            i += 1;
        }
        proof { lemma_normalized_semantics(self.records@, self.records.len() as int, last); }
        result
    }
}

pub open spec fn is_descendant(path: Seq<Seq<u8>>, root: Seq<Seq<u8>>) -> bool {
    path.len() >= root.len() && forall|i: int| 0 <= i < root.len() ==> path[i] == root[i]
}
proof fn lemma_byte_order_asymmetric(left: Seq<u8>, right: Seq<u8>)
    requires crate::path::bytes_less(left, right),
    ensures !crate::path::bytes_less(right, left),
{
    broadcast use vstd::seq::group_seq_lemmas;
    if left.len() < right.len() && left == right.take(left.len() as int) {
        assert forall|j: int| 0 <= j < left.len() && j < right.len()
            implies !(right.take(j) == left.take(j) && right[j] < left[j]) by {
            assert(left[j] == right[j]);
        }
    } else {
        let i = choose|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && left[i] < right[i];
        assert(0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && left[i] < right[i]);
        if right.len() < left.len() && right == left.take(right.len() as int) {
            assert(left[i] == right[i]);
        }
        assert forall|j: int| 0 <= j < left.len() && j < right.len()
            implies !(right.take(j) == left.take(j) && right[j] < left[j]) by {
            if j < i {
                assert(left.take(i)[j] == left[j]);
                assert(right.take(i)[j] == right[j]);
                assert(left[j] == right[j]);
            } else if j > i && right.take(j) == left.take(j) {
                assert(left.take(j)[i] == left[i]);
                assert(right.take(j)[i] == right[i]);
                assert(left[i] == right[i]);
            }
        }
    }
}
proof fn lemma_component_order_asymmetric(left: Seq<Seq<u8>>, right: Seq<Seq<u8>>)
    requires crate::path::components_less(left, right),
    ensures !crate::path::components_less(right, left),
{
    broadcast use vstd::seq::group_seq_lemmas;
    if left.len() < right.len() && left == right.take(left.len() as int) {
        assert forall|j: int| 0 <= j < left.len() && j < right.len()
            implies !(right.take(j) == left.take(j) && crate::path::bytes_less(right[j], left[j])) by {
            assert(left[j] == right[j]);
            if crate::path::bytes_less(right[j], left[j]) {
                lemma_byte_order_asymmetric(right[j], left[j]);
            }
        }
    } else {
        let i = choose|i: int| 0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && crate::path::bytes_less(left[i], right[i]);
        assert(0 <= i < left.len() && i < right.len()
            && left.take(i) == right.take(i) && crate::path::bytes_less(left[i], right[i]));
        lemma_byte_order_asymmetric(left[i], right[i]);
        if right.len() < left.len() && right == left.take(right.len() as int) {
            assert(left[i] == right[i]);
        }
        assert forall|j: int| 0 <= j < left.len() && j < right.len()
            implies !(right.take(j) == left.take(j) && crate::path::bytes_less(right[j], left[j])) by {
            if j < i {
                assert(left.take(i)[j] == left[j]);
                assert(right.take(i)[j] == right[j]);
                assert(left[j] == right[j]);
                if crate::path::bytes_less(right[j], left[j]) {
                    lemma_byte_order_asymmetric(right[j], left[j]);
                }
            } else if j > i && right.take(j) == left.take(j) {
                assert(left.take(j)[i] == left[i]);
                assert(right.take(j)[i] == right[i]);
                assert(left[i] == right[i]);
            }
        }
    }
}
fn descendant(path: &BackupPath, root: &BackupPath) -> (result: bool)
    ensures result == is_descendant(path@, root@),
{
    if path.components().len() < root.components().len() { return false; }
    let mut i = 0;
    while i < root.components().len()
        invariant i <= root@.len(), path@.len() >= root@.len(),
            forall|j: int| 0 <= j < i ==> path@[j] == root@[j],
        decreases root@.len() - i,
    {
        if path.components()[i] != root.components()[i] {
            assert(path@[i as int] != root@[i as int]);
            return false;
        }
        i += 1;
    }
    true
}

fn path_before_or_equal(left: &BackupPath, right: &BackupPath) -> (result: bool)
    ensures result == !crate::path::components_less(right@, left@),
{
    let order = left.compare(right);
    proof {
        if order < 0 { lemma_component_order_asymmetric(left@, right@); }
        else if order == 0 && crate::path::components_less(right@, left@) {
            lemma_component_order_asymmetric(right@, left@);
        }
    }
    order <= 0
}
pub open spec fn volume_selection(start: Seq<Seq<u8>>, end: Seq<Seq<u8>>,
    selected: Seq<Seq<u8>>) -> bool {
    crate::path::components_less(end, start)
        || (!crate::path::components_less(selected, start)
            && !crate::path::components_less(end, selected))
        || is_descendant(start, selected) || is_descendant(end, selected)
}
proof fn lemma_descendant_prefix(path: Seq<Seq<u8>>, root: Seq<Seq<u8>>)
    requires is_descendant(path, root),
    ensures root == path.take(root.len() as int),
{
    assert(root =~= path.take(root.len() as int));
}
proof fn lemma_less_descendant_right(left: Seq<Seq<u8>>, root: Seq<Seq<u8>>, path: Seq<Seq<u8>>)
    requires crate::path::components_less(left, root), is_descendant(path, root),
    ensures crate::path::components_less(left, path),
{
    broadcast use vstd::seq::group_seq_lemmas;
    lemma_descendant_prefix(path, root);
    if left.len() < root.len() && left == root.take(left.len() as int) {
        assert(left =~= path.take(left.len() as int));
    } else {
        let i = choose|i: int| 0 <= i < left.len() && i < root.len()
            && left.take(i) == root.take(i) && crate::path::bytes_less(left[i], root[i]);
        assert(0 <= i < left.len() && i < root.len()
            && left.take(i) == root.take(i) && crate::path::bytes_less(left[i], root[i]));
        assert(root[i] == path[i]);
        assert(root.take(i) =~= path.take(i));
        assert(left.take(i) == path.take(i) && crate::path::bytes_less(left[i], path[i]));
    }
}
proof fn lemma_less_descendant_left(root: Seq<Seq<u8>>, right: Seq<Seq<u8>>, path: Seq<Seq<u8>>)
    requires crate::path::components_less(root, right), !is_descendant(right, root),
        is_descendant(path, root),
    ensures crate::path::components_less(path, right),
{
    broadcast use vstd::seq::group_seq_lemmas;
    lemma_descendant_prefix(path, root);
    if root.len() < right.len() && root == right.take(root.len() as int) {
        assert(is_descendant(right, root));
    }
    let i = choose|i: int| 0 <= i < root.len() && i < right.len()
        && root.take(i) == right.take(i) && crate::path::bytes_less(root[i], right[i]);
    assert(0 <= i < root.len() && i < right.len()
        && root.take(i) == right.take(i) && crate::path::bytes_less(root[i], right[i]));
    assert(root[i] == path[i]);
    assert(root.take(i) =~= path.take(i));
    assert(path.take(i) == right.take(i) && crate::path::bytes_less(path[i], right[i]));
}
/// Every entry in the inclusive interval whose components extend the selected
/// path causes selection, including entries shared by multipart boundaries.
pub proof fn lemma_selection_covers_descendant(start: Seq<Seq<u8>>, end: Seq<Seq<u8>>,
    selected: Seq<Seq<u8>>, entry: Seq<Seq<u8>>)
    requires is_descendant(entry, selected),
        !crate::path::components_less(entry, start),
        !crate::path::components_less(end, entry),
    ensures volume_selection(start, end, selected),
{
    if !volume_selection(start, end, selected) {
        if crate::path::components_less(selected, start) {
            lemma_less_descendant_left(selected, start, entry);
        } else if crate::path::components_less(end, selected) {
            lemma_less_descendant_right(end, selected, entry);
        }
    }
}
impl VolumeRecord {
    //= docs/parsing-requirements.md#parse-man-005
    //# Volume selection MUST include every volume whose declared inclusive path interval can contain a selected entry or descendant, including shared multipart boundary paths.
    pub fn may_contain(&self, selected: &BackupPath) -> (result: bool)
        ensures result == volume_selection(self.start.path@, self.end.path@, selected@),
    {
        // Public records can be caller-constructed. Invalid ranges select all.
        if !path_before_or_equal(&self.start.path, &self.end.path) { return true; }
        (path_before_or_equal(&self.start.path, selected) && path_before_or_equal(selected, &self.end.path))
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
