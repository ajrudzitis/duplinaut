//! Counted PAX record framing. Key semantics and complete-map refinement are
//! separate obligations; this parser accepts no malformed count or delimiter.
// Explicit predicates, branch lowering and borrowed-record lifetimes remain visible to Verus.
#![allow(
    clippy::len_zero,
    clippy::manual_range_contains,
    clippy::question_mark,
    clippy::needless_lifetimes
)]
use super::tar_helpers::decimal;
#[cfg(verus_keep_ghost)]
use super::tar_helpers::decimal_prefix;
use super::tar_time::time;
#[cfg(verus_keep_ghost)]
use super::tar_time::time_model;
use vest_lib::core::exec::bytes_eq;
use vstd::prelude::*;
verus! {
pub open spec fn find(input: Seq<u8>, begin: int, end: int, byte: u8) -> Option<int>
    recommends 0 <= begin <= end <= input.len(),
    decreases end - begin,
{
    if begin >= end { None }
    else if input[begin] == byte { Some(begin) }
    else { find(input, begin + 1, end, byte) }
}
pub proof fn find_bounds(input: Seq<u8>, begin: int, end: int, byte: u8)
    requires 0 <= begin <= end <= input.len(),
    ensures match find(input, begin, end, byte) {
        Some(i) => begin <= i < end && input[i] == byte,
        None => true,
    },
    decreases end - begin,
{
    if begin < end && input[begin] != byte { find_bounds(input, begin + 1, end, byte); }
}
pub fn find_byte(input: &[u8], begin: usize, end: usize, byte: u8) -> (result: Option<usize>)
    requires begin <= end <= input.len(),
    ensures match result {
        Some(i) => find(input@, begin as int, end as int, byte) == Some(i as int)
            && begin <= i < end,
        None => find(input@, begin as int, end as int, byte) is None,
    },
{
    let mut i = begin;
    while i < end
        invariant begin <= i <= end <= input.len(),
            find(input@, i as int, end as int, byte) == find(input@, begin as int, end as int, byte),
        decreases end - i,
    {
        if input[i] == byte { return Some(i); }
        i += 1;
    }
    None
}
pub open spec fn record_model(input: Seq<u8>, cursor: int) -> Option<(int, Seq<u8>, Seq<u8>)> {
    if !(0 <= cursor < input.len()) { None }
    else { match find(input, cursor, input.len() as int, b' ') {
        None => None,
        Some(space) => {
            let digits = input.subrange(cursor, space);
            if digits.len() == 0 { None }
            else { match decimal_prefix(digits, digits.len() as int) {
                None => None,
                Some(size) => {
                    let end = cursor + size;
                    if size > usize::MAX as int || end > usize::MAX as int
                        || end > input.len() || end <= space + 1 || input[end - 1] != b'\n' { None }
                    else { match find(input, space + 1, end - 1, b'=') {
                        None => None,
                        Some(equal) => Some((end, input.subrange(space + 1, equal), input.subrange(equal + 1, end - 1))),
                    }}
                },
            }}
        },
    }}
}
pub fn record<'a>(input: &'a [u8], cursor: usize)
    -> (result: Option<(usize, &'a [u8], &'a [u8])>)
    ensures match result {
        Some((end, key, value)) => record_model(input@, cursor as int)
                == Some((end as int, key@, value@))
            && cursor < end <= input.len(),
        None => record_model(input@, cursor as int) is None,
    },
{
    if cursor >= input.len() { return None; }
    let space = match find_byte(input, cursor, input.len(), b' ') {
        Some(i) => i,
        None => return None,
    };
    let digits = &input[cursor..space];
    let size = match decimal(digits) {
        Some(n) => n,
        None => return None,
    };
    if size > usize::MAX as u64 { return None; }
    let size = size as usize;
    if size > usize::MAX - cursor { return None; }
    let end = cursor + size;
    if end > input.len() || end <= space + 1 || input[end - 1] != b'\n' { return None; }
    let equal = match find_byte(input, space + 1, end - 1, b'=') {
        Some(i) => i,
        None => return None,
    };
    Some((end, &input[space + 1..equal], &input[equal + 1..end - 1]))
}
pub open spec fn key_model(input: Seq<u8>) -> Option<usize> {
    if input == b"path"@ { Some(0) }
    else if input == b"linkpath"@ { Some(1) }
    else if input == b"size"@ { Some(2) }
    else if input == b"mtime"@ { Some(3) }
    else if input == b"uid"@ { Some(4) }
    else if input == b"gid"@ { Some(5) }
    else if input == b"uname"@ { Some(6) }
    else if input == b"gname"@ { Some(7) }
    else if input == b"atime"@ { Some(8) }
    else if input == b"ctime"@ { Some(9) }
    else if input == b"hdrcharset"@ { Some(10) }
    else { None }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Runtime slot selection used by the native PAX controller.
pub open spec fn resolve_value_model(
    global: Option<&[u8]>,
    local: Option<&[u8]>,
) -> Option<Seq<u8>> {
    match local {
        Some(value) => if value.len() == 0 { None } else { Some(value@) },
        None => match global {
            Some(value) => Some(value@),
            None => None,
        },
    }
}

pub fn resolve_value<'a>(global: Option<&'a [u8]>, local: Option<&'a [u8]>)
    -> (result: Option<&'a [u8]>)
    ensures match local {
        Some(value) => if value.len() == 0 {
            result is None
        } else {
            match result { Some(actual) => actual@ == value@, None => false }
        },
        None => match global {
            None => result is None,
            Some(value) => match result {
                Some(actual) => actual@ == value@,
                None => false,
            },
        },
    },
    resolve_value_model(global, local) == match result {
        None => None,
        Some(value) => Some(value@),
    },
{
    match local {
        None => global,
        Some(value) => if value.is_empty() { None } else { Some(value) },
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Resolve every supported PAX slot using the existing per-slot rule.
//= docs/parsing-requirements.md#parse-tar-006
//# Local fields MUST override global fields for the next real entry.
pub fn resolve_values<'a>(
    global: [Option<&'a [u8]>; 11],
    local: [Option<&'a [u8]>; 11],
) -> (result: [Option<&'a [u8]>; 11])
    ensures forall|i: int| 0 <= i < 11 ==> resolve_value_model(global[i], local[i])
        == match result[i] {
            None => None,
            Some(value) => Some(value@),
        },
{
    let mut result = global;
    let mut i = 0;
    while i < 11
        invariant 0 <= i <= 11,
            forall|j: int| 0 <= j < i ==> resolve_value_model(global[j], local[j])
                == match result[j] {
                    None => None,
                    Some(value) => Some(value@),
                },
        decreases 11 - i,
    {
        result[i] = resolve_value(global[i], local[i]);
        i += 1;
    }
    result
}

//= docs/parsing-requirements.md#parse-tar-006
//# Each present field in a global `g` header replaces the prior global value, an empty global value deletes it, and omitted global fields persist.
pub open spec fn global_update_model(
    previous: Option<&[u8]>,
    update: Option<&[u8]>,
) -> Option<Seq<u8>> {
    match update {
        None => match previous { None => None, Some(value) => Some(value@) },
        Some(value) => if value.len() == 0 { None } else { Some(value@) },
    }
}

pub open spec fn local_update_model(
    previous: Option<&[u8]>,
    update: Option<&[u8]>,
) -> Option<Seq<u8>> {
    match update {
        None => match previous { None => None, Some(value) => Some(value@) },
        Some(value) => Some(value@),
    }
}

//= docs/parsing-requirements.md#parse-tar-006
//# Each present field in a global `g` header replaces the prior global value, an empty global value deletes it, and omitted global fields persist.
pub fn apply_global_update<'a>(
    previous: [Option<&'a [u8]>; 11],
    update: [Option<&'a [u8]>; 11],
) -> (result: [Option<&'a [u8]>; 11])
    ensures forall|i: int| 0 <= i < 11 ==>
        global_update_model(previous[i], update[i]) == match result[i] {
            None => None,
            Some(value) => Some(value@),
        },
{
    let mut result = previous;
    let mut i = 0;
    while i < 11
        invariant 0 <= i <= 11,
            forall|j: int| 0 <= j < i ==>
                global_update_model(previous[j], update[j]) == match result[j] {
                    None => None,
                    Some(value) => Some(value@),
                },
        decreases 11 - i,
    {
        result[i] = match update[i] {
            None => previous[i],
            Some(value) => if value.is_empty() { None } else { Some(value) },
        };
        i += 1;
    }
    result
}

//= docs/parsing-requirements.md#parse-tar-006
//# Each present field in a local `x` header replaces the pending local value, including empty values, while omitted local fields persist across later extension headers.
pub fn apply_local_update<'a>(
    previous: [Option<&'a [u8]>; 11],
    update: [Option<&'a [u8]>; 11],
) -> (result: [Option<&'a [u8]>; 11])
    ensures forall|i: int| 0 <= i < 11 ==>
        local_update_model(previous[i], update[i]) == match result[i] {
            None => None,
            Some(value) => Some(value@),
        },
{
    let mut result = previous;
    let mut i = 0;
    while i < 11
        invariant 0 <= i <= 11,
            forall|j: int| 0 <= j < i ==>
                local_update_model(previous[j], update[j]) == match result[j] {
                    None => None,
                    Some(value) => Some(value@),
                },
        decreases 11 - i,
    {
        result[i] = match update[i] {
            None => previous[i],
            Some(_) => update[i],
        };
        i += 1;
    }
    result
}

//= docs/parsing-requirements.md#parse-tar-006
//# That local state MUST be consumed only after the real entry is accepted.
pub fn consume_local_update<'a>(
    _previous: [Option<&'a [u8]>; 11],
) -> (result: [Option<&'a [u8]>; 11])
    ensures forall|i: int| 0 <= i < 11 ==> result[i] is None,
{
    [None; 11]
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select the effective PAX path before a pending GNU long-name override.
pub fn effective_name_override<'a>(
    pax_path: Option<&'a [u8]>,
    gnu_long_name: Option<&'a [u8]>,
) -> (result: Option<&'a [u8]>)
    ensures match pax_path {
        Some(value) => match result {
            Some(actual) => actual@ == value@,
            None => false,
        },
        None => match gnu_long_name {
            Some(value) => match result {
                Some(actual) => actual@ == value@,
                None => false,
            },
            None => result is None,
        },
    },
{
    match pax_path {
        Some(value) => Some(value),
        None => gnu_long_name,
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select the effective PAX linkpath before a pending GNU long-link override.
pub fn effective_link_override<'a>(
    pax_link: Option<&'a [u8]>,
    gnu_long_link: Option<&'a [u8]>,
) -> (result: Option<&'a [u8]>)
    ensures match pax_link {
        Some(value) => match result {
            Some(actual) => actual@ == value@,
            None => false,
        },
        None => match gnu_long_link {
            Some(value) => match result {
                Some(actual) => actual@ == value@,
                None => false,
            },
            None => result is None,
        },
    },
{
    match pax_link {
        Some(value) => Some(value),
        None => gnu_long_link,
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select a PAX owner-name override or preserve the raw header owner name.
pub fn effective_owner_name<'a>(pax_name: Option<&'a [u8]>, header_name: &'a [u8])
    -> (result: &'a [u8])
    ensures match pax_name {
        Some(value) => result@ == value@,
        None => result@ == header_name@,
    },
{
    match pax_name {
        Some(value) => value,
        None => header_name,
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Return whether the resolved charset uses the parser's UTF-8 text policy.
pub fn owner_name_requires_utf8(charset: Option<&[u8]>) -> (result: bool)
    ensures result == match charset {
        Some(value) => value@ != b"BINARY"@,
        None => true,
    },
{
    match charset {
        Some(value) => !bytes_eq(value, b"BINARY"),
        None => true,
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

//= docs/parsing-requirements.md#parse-tar-004
//# Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

/// Recognize exactly the PAX header charset tokens supported by the native parser.
pub fn hdrcharset_value_supported(value: &[u8]) -> (result: bool)
    ensures result == (value@ == b"BINARY"@ || value@ == b"UTF-8"@
        || value@ == b"ISO-IR 10646 2000 UTF-8"@),
{
    bytes_eq(value, b"BINARY") || bytes_eq(value, b"UTF-8")
        || bytes_eq(value, b"ISO-IR 10646 2000 UTF-8")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxSizeError {
    Invalid,
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select the resolved PAX size override or retain the raw header size.
pub fn effective_size(value: Option<&[u8]>, header_size: u64)
    -> (result: Result<u64, PaxSizeError>)
    ensures match value {
        None => result == Ok(header_size),
        Some(bytes) => match result {
            Ok(size) => bytes.len() > 0
                && decimal_prefix(bytes@, bytes.len() as int) == Some(size as int),
            Err(error) => error == PaxSizeError::Invalid
                && (bytes.len() == 0 || decimal_prefix(bytes@, bytes.len() as int) is None),
        },
    },
{
    match value {
        None => Ok(header_size),
        Some(bytes) => match decimal(bytes) {
            Some(size) => Ok(size),
            None => Err(PaxSizeError::Invalid),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxIdError {
    Invalid,
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select a resolved PAX uid/gid override or retain the raw header value.
pub fn effective_id(value: Option<&[u8]>, header_id: u64)
    -> (result: Result<u64, PaxIdError>)
    ensures match value {
        None => result == Ok(header_id),
        Some(bytes) => match result {
            Ok(id) => bytes.len() > 0
                && decimal_prefix(bytes@, bytes.len() as int) == Some(id as int),
            Err(error) => error == PaxIdError::Invalid
                && (bytes.len() == 0 || decimal_prefix(bytes@, bytes.len() as int) is None),
        },
    },
{
    match value {
        None => Ok(header_id),
        Some(bytes) => match decimal(bytes) {
            Some(id) => Ok(id),
            None => Err(PaxIdError::Invalid),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxTimeError {
    Invalid,
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Use the resolved PAX mtime when present; otherwise keep the header seconds.
pub fn effective_mtime(value: Option<&[u8]>, header_seconds: i64)
    -> (result: Result<(i64, u32), PaxTimeError>)
    ensures match value {
        None => result == Ok((header_seconds, 0)),
        Some(bytes) => match result {
            Ok((seconds, nanos)) => time_model(bytes@) == Some((seconds as int, nanos as int))
                && nanos < 1000000000,
            Err(error) => error == PaxTimeError::Invalid && time_model(bytes@) is None,
        },
    },
{
    match value {
        None => Ok((header_seconds, 0)),
        Some(bytes) => match time(bytes) {
            Some(value) => Ok(value),
            None => Err(PaxTimeError::Invalid),
        },
    }
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Decode an optional PAX time field without inventing a value when absent.
pub fn effective_optional_time(value: Option<&[u8]>)
    -> (result: Result<Option<(i64, u32)>, PaxTimeError>)
    ensures match value {
        None => result == Ok(None),
        Some(bytes) => match result {
            Ok(Some((seconds, nanos))) =>
                time_model(bytes@) == Some((seconds as int, nanos as int))
                    && nanos < 1000000000,
            Ok(None) => false,
            Err(error) => error == PaxTimeError::Invalid && time_model(bytes@) is None,
        },
    },
{
    match value {
        None => Ok(None),
        Some(bytes) => match time(bytes) {
            Some(value) => Ok(Some(value)),
            None => Err(PaxTimeError::Invalid),
        },
    }
}

pub fn key_index(input: &[u8]) -> (result: Option<usize>)
    ensures result == key_model(input@), result is Some ==> result.unwrap() < 11,
{
    if bytes_eq(input, b"path") { Some(0) }
    else if bytes_eq(input, b"linkpath") { Some(1) }
    else if bytes_eq(input, b"size") { Some(2) }
    else if bytes_eq(input, b"mtime") { Some(3) }
    else if bytes_eq(input, b"uid") { Some(4) }
    else if bytes_eq(input, b"gid") { Some(5) }
    else if bytes_eq(input, b"uname") { Some(6) }
    else if bytes_eq(input, b"gname") { Some(7) }
    else if bytes_eq(input, b"atime") { Some(8) }
    else if bytes_eq(input, b"ctime") { Some(9) }
    else if bytes_eq(input, b"hdrcharset") { Some(10) }
    else { None }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxRecordAdmissionError {
    UnsupportedKey,
    Malformed,
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Admit one framed PAX key/value pair into an unoccupied semantic slot.
pub fn admit_record(key: &[u8], value: &[u8], occupied: bool)
    -> (result: Result<usize, PaxRecordAdmissionError>)
    ensures match key_model(key@) {
        None => result == Err(PaxRecordAdmissionError::UnsupportedKey),
        Some(index) => if occupied
            || find(value@, 0, value@.len() as int, 0) is Some {
                result == Err(PaxRecordAdmissionError::Malformed)
            } else {
                result == Ok(index)
            },
    },
{
    let index = match key_index(key) {
        Some(index) => index,
        None => return Err(PaxRecordAdmissionError::UnsupportedKey),
    };
    if occupied {
        return Err(PaxRecordAdmissionError::Malformed);
    }
    if find_byte(value, 0, value.len(), 0).is_some() {
        return Err(PaxRecordAdmissionError::Malformed);
    }
    Ok(index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxIngestError {
    Framing,
    UnsupportedKey,
    Malformed,
    Numeric,
    UnsupportedValue,
}

pub open spec fn pax_slots_model(slots: Seq<Option<&[u8]>>) -> Seq<Option<Seq<u8>>> {
    Seq::new(slots.len(), |i: int| match slots[i] {
        None => None,
        Some(value) => Some(value@),
    })
}

pub open spec fn pax_empty_model() -> Seq<Option<Seq<u8>>> {
    Seq::new(11, |_i: int| None)
}

pub open spec fn pax_value_model(index: int, value: Seq<u8>) -> bool {
    if value.len() == 0 { true }
    else if index == 2 || index == 4 || index == 5 {
        decimal_prefix(value, value.len() as int) is Some
    }
    else if index == 3 || index == 8 || index == 9 {
        time_model(value) is Some
    }
    else if index == 10 {
        value == b"BINARY"@ || value == b"UTF-8"@
            || value == b"ISO-IR 10646 2000 UTF-8"@
    }
    else { true }
}

/// Model the successful suffix of a framed PAX byte stream from its current
/// cursor and current slot map. Errors are intentionally represented as None;
/// this model does not specify which diagnostic malformed input produces.
pub open spec fn pax_stream_model(
    input: Seq<u8>,
    cursor: int,
    slots: Seq<Option<Seq<u8>>>,
) -> Option<Seq<Option<Seq<u8>>>>
    recommends 0 <= cursor <= input.len(), slots.len() == 11,
    decreases input.len() - cursor,
{
    if cursor == input.len() { Some(slots) }
    else { match record_model(input, cursor) {
        None => None,
        Some((end, key, value)) => if end <= cursor || end > input.len() { None }
        else { match key_model(key) {
            None => None,
            Some(index) => if index >= slots.len() { None }
            else if slots[index as int] is Some
                || find(value, 0, value.len() as int, 0) is Some
                || !pax_value_model(index as int, value) { None }
            else {
                pax_stream_model(input, end, slots.update(index as int, Some(value)))
            },
        }},
    }}
}

/// Classify the first error in a PAX stream suffix. A successful record
/// advances to the next record with its semantic slot stored; later errors
/// cannot override an earlier error.
pub open spec fn pax_first_error_model(
    input: Seq<u8>,
    cursor: int,
    slots: Seq<Option<Seq<u8>>>,
) -> Option<PaxIngestError>
    recommends 0 <= cursor <= input.len(), slots.len() == 11,
    decreases input.len() - cursor,
{
    if cursor == input.len() { None }
    else { match record_model(input, cursor) {
        None => Some(PaxIngestError::Framing),
        Some((end, key, value)) => if end <= cursor || end > input.len() {
            Some(PaxIngestError::Framing)
        }
        else { match key_model(key) {
            None => Some(PaxIngestError::UnsupportedKey),
            Some(index) => if index >= slots.len() {
                Some(PaxIngestError::UnsupportedKey)
            }
            else if slots[index as int] is Some
                || find(value, 0, value.len() as int, 0) is Some {
                Some(PaxIngestError::Malformed)
            }
            else if !pax_value_model(index as int, value) {
                if index == 10 {
                    Some(PaxIngestError::UnsupportedValue)
                } else {
                    Some(PaxIngestError::Numeric)
                }
            }
            else {
                pax_first_error_model(input, end, slots.update(index as int, Some(value)))
            },
        }}
    }}
}

//= docs/parsing-requirements.md#parse-tar-003
//# For a successful PAX extended-header stream, the reader MUST process records in input order and accumulate each supported, unique, NUL-free key/value pair into its assigned one of the eleven metadata slots.

//= docs/parsing-requirements.md#parse-tar-003
//# For each PAX extended-header stream, the reader MUST process records in input order and report the first failing record's error class using framing before key support, key support before duplicate/NUL admission, and admission before value syntax: malformed framing or duplicate/NUL values for a supported key classify as Pax, unsupported keys (including those with NUL values) and unsupported nonempty hdrcharset values as UnsupportedPax, and invalid decimal/time values as Numeric; empty values are accepted, and no later record changes the first error.

/// Parse and accumulate a complete PAX stream into its eleven semantic slots.
/// The success postcondition relates the actual borrowed slot map to the
/// recursive framing/key/value/uniqueness/NUL model and the exact first-error
/// classifier. Existing decimal, time and hdrcharset validator contracts
/// establish the modeled value predicates.
pub fn ingest<'a>(input: &'a [u8])
    -> (result: Result<[Option<&'a [u8]>; 11], PaxIngestError>)
    ensures match result {
        Ok(slots) => pax_stream_model(input@, 0, pax_empty_model())
                == Some(pax_slots_model(slots@))
            && pax_first_error_model(input@, 0, pax_empty_model()) is None,
        Err(error) => pax_first_error_model(input@, 0, pax_empty_model()) == Some(error),
    },
{
    let mut slots: [Option<&'a [u8]>; 11] = [None; 11];
    let mut cursor = 0usize;
    let ghost whole_model = pax_stream_model(input@, 0, pax_empty_model());
    proof {
        reveal(pax_slots_model);
        reveal(pax_empty_model);
        assert(pax_slots_model(slots@) == pax_empty_model());
    }
    while cursor < input.len()
        invariant cursor <= input.len(),
            pax_stream_model(input@, cursor as int, pax_slots_model(slots@))
                == whole_model,
            pax_first_error_model(input@, cursor as int, pax_slots_model(slots@))
                == pax_first_error_model(input@, 0, pax_empty_model()),
        decreases input.len() - cursor,
    {
        proof {
            reveal(pax_stream_model);
            reveal(pax_first_error_model);
        }
        let ghost old_cursor = cursor as int;
        let ghost old_slots = pax_slots_model(slots@);
        let (end, key, value) = match record(input, cursor) {
            Some(record) => record,
            None => {
                proof {
                    assert(pax_first_error_model(
                        input@,
                        cursor as int,
                        pax_slots_model(slots@),
                    ) == Some(PaxIngestError::Framing));
                    assert(pax_first_error_model(
                        input@,
                        0,
                        pax_empty_model(),
                    ) == Some(PaxIngestError::Framing));
                }
                return Err(PaxIngestError::Framing);
            }
        };
        let key_slot = match key_index(key) {
            Some(index) => index,
            None => {
                proof {
                    assert(pax_first_error_model(
                        input@,
                        cursor as int,
                        pax_slots_model(slots@),
                    ) == Some(PaxIngestError::UnsupportedKey));
                    assert(pax_first_error_model(
                        input@,
                        0,
                        pax_empty_model(),
                    ) == Some(PaxIngestError::UnsupportedKey));
                }
                return Err(PaxIngestError::UnsupportedKey);
            }
        };
        let index = match admit_record(key, value, slots[key_slot].is_some()) {
            Ok(index) => index,
            Err(PaxRecordAdmissionError::UnsupportedKey) => {
                proof { assert(false); }
                return Err(PaxIngestError::UnsupportedKey);
            }
            Err(PaxRecordAdmissionError::Malformed) => {
                proof {
                    assert(pax_first_error_model(
                        input@,
                        cursor as int,
                        pax_slots_model(slots@),
                    ) == Some(PaxIngestError::Malformed));
                    assert(pax_first_error_model(
                        input@,
                        0,
                        pax_empty_model(),
                    ) == Some(PaxIngestError::Malformed));
                }
                return Err(PaxIngestError::Malformed);
            }
        };
        if !value.is_empty() {
            match index {
                2 | 4 | 5 => {
                    let parsed = super::tar_helpers::decimal(value);
                    if parsed.is_none() {
                        proof {
                            assert(pax_first_error_model(
                                input@,
                                cursor as int,
                                pax_slots_model(slots@),
                            ) == Some(PaxIngestError::Numeric));
                            assert(pax_first_error_model(
                                input@,
                                0,
                                pax_empty_model(),
                            ) == Some(PaxIngestError::Numeric));
                        }
                        return Err(PaxIngestError::Numeric);
                    }
                    proof {
                        assert(super::tar_helpers::decimal_prefix(
                            value@,
                            value.len() as int,
                        ) is Some);
                    }
                }
                3 | 8 | 9 => {
                    let parsed = time(value);
                    if parsed.is_none() {
                        proof {
                            assert(pax_first_error_model(
                                input@,
                                cursor as int,
                                pax_slots_model(slots@),
                            ) == Some(PaxIngestError::Numeric));
                            assert(pax_first_error_model(
                                input@,
                                0,
                                pax_empty_model(),
                            ) == Some(PaxIngestError::Numeric));
                        }
                        return Err(PaxIngestError::Numeric);
                    }
                    proof { assert(time_model(value@) is Some); }
                }
                10 => {
                    if !hdrcharset_value_supported(value) {
                        proof {
                            assert(pax_first_error_model(
                                input@,
                                cursor as int,
                                pax_slots_model(slots@),
                            ) == Some(PaxIngestError::UnsupportedValue));
                            assert(pax_first_error_model(
                                input@,
                                0,
                                pax_empty_model(),
                            ) == Some(PaxIngestError::UnsupportedValue));
                        }
                        return Err(PaxIngestError::UnsupportedValue);
                    }
                    proof { assert(pax_value_model(10, value@)); }
                }
                _ => {}
            }
        }
        proof {
            reveal(pax_stream_model);
            reveal(pax_first_error_model);
            assert(record_model(input@, old_cursor)
                == Some((end as int, key@, value@)));
            assert(key_slot == index);
            assert(key_model(key@) == Some(index));
            assert(index < 11);
            assert(old_slots[index as int] is None);
            assert(find(value@, 0, value.len() as int, 0) is None);
            assert(pax_value_model(index as int, value@));
            assert(pax_stream_model(input@, old_cursor, old_slots)
                == pax_stream_model(
                    input@,
                    end as int,
                    old_slots.update(index as int, Some(value@)),
                ));
            assert(pax_first_error_model(input@, old_cursor, old_slots)
                == pax_first_error_model(
                    input@,
                    end as int,
                    old_slots.update(index as int, Some(value@)),
                ));
        }
        slots[index] = Some(value);
        proof {
            reveal(pax_slots_model);
            assert(pax_slots_model(slots@)
                == old_slots.update(index as int, Some(value@)));
        }
        cursor = end;
    }
    proof {
        reveal(pax_stream_model);
        reveal(pax_first_error_model);
        assert(pax_first_error_model(input@, 0, pax_empty_model()) is None);
    }
    Ok(slots)
}
}

#[cfg(test)]
mod tests {
    use super::{
        admit_record, effective_id, effective_link_override, effective_mtime,
        effective_name_override, effective_optional_time, effective_owner_name, effective_size,
        hdrcharset_value_supported, ingest, owner_name_requires_utf8, resolve_values, PaxIdError,
        PaxIngestError, PaxRecordAdmissionError, PaxSizeError, PaxTimeError,
    };

    fn framed_record(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(key);
        body.push(b'=');
        body.extend_from_slice(value);
        body.push(b'\n');
        let mut size = body.len() + 2;
        loop {
            let next = body.len() + size.to_string().len() + 1;
            if next == size {
                break;
            }
            size = next;
        }
        let mut encoded = size.to_string().into_bytes();
        encoded.push(b' ');
        encoded.extend_from_slice(&body);
        encoded
    }

    #[test]
    fn ingest_accumulates_a_successful_stream_into_all_eleven_slots() {
        let fields: [(&[u8], &[u8]); 11] = [
            (b"path", b"snapshot/file"),
            (b"linkpath", b""),
            (b"size", b"3"),
            (b"mtime", b"13.25"),
            (b"uid", b"11"),
            (b"gid", b"12"),
            (b"uname", b"user"),
            (b"gname", b"group"),
            (b"atime", b"14"),
            (b"ctime", b"15"),
            (b"hdrcharset", b"UTF-8"),
        ];
        let mut stream = Vec::new();
        for (key, value) in fields {
            stream.extend_from_slice(&framed_record(key, value));
        }
        let slots = ingest(&stream).expect("all eleven supported records are valid");
        for (index, (_, expected)) in fields.iter().enumerate() {
            assert_eq!(slots[index], Some(*expected), "slot {index}");
        }
    }

    #[test]
    fn extension_state_updates_preserve_persistence_merge_and_consumption_semantics() {
        let absent = [None; 11];
        let mut global_update = [None; 11];
        global_update[0] = Some(&b"global-a"[..]);
        global_update[4] = Some(&b"99"[..]);
        let global = super::apply_global_update(absent, global_update);
        let mut local_update = [None; 11];
        local_update[0] = Some(&b"local"[..]);
        local_update[4] = Some(&b""[..]);
        let local = super::apply_local_update(absent, local_update);

        let mut second_global_update = [None; 11];
        second_global_update[0] = Some(&b"global-b"[..]);
        second_global_update[4] = Some(&b""[..]);
        let global = super::apply_global_update(global, second_global_update);

        let effective = resolve_values(global, local);
        assert_eq!(effective[0], Some(&b"local"[..]));
        assert_eq!(effective[4], None);
        assert_eq!(global[0], Some(&b"global-b"[..]));
        assert_eq!(global[4], None);

        let consumed = super::consume_local_update(local);
        assert_eq!(consumed, [None; 11]);
        let after_entry = resolve_values(global, consumed);
        assert_eq!(after_entry[0], Some(&b"global-b"[..]));
        assert_eq!(after_entry[4], None);
    }

    #[test]
    fn ingest_classifies_the_first_error_in_stream_and_record_order() {
        let cases: [(Vec<u8>, PaxIngestError); 9] = [
            (
                [
                    framed_record(b"size", b"bad"),
                    framed_record(b"unknown", b"v"),
                ]
                .concat(),
                PaxIngestError::Numeric,
            ),
            (
                [
                    framed_record(b"mtime", b"not-a-time"),
                    framed_record(b"unknown", b"v"),
                ]
                .concat(),
                PaxIngestError::Numeric,
            ),
            (
                [framed_record(b"unknown", b"v"), b"bad frame".to_vec()].concat(),
                PaxIngestError::UnsupportedKey,
            ),
            (
                [
                    framed_record(b"path", b"value\0tail"),
                    framed_record(b"size", b"bad"),
                ]
                .concat(),
                PaxIngestError::Malformed,
            ),
            (
                [framed_record(b"size", b"1"), b"bad frame".to_vec()].concat(),
                PaxIngestError::Framing,
            ),
            (
                [framed_record(b"size", b"1"), framed_record(b"size", b"bad")].concat(),
                PaxIngestError::Malformed,
            ),
            (
                [
                    framed_record(b"size", b"1\0"),
                    framed_record(b"path", b"later"),
                ]
                .concat(),
                PaxIngestError::Malformed,
            ),
            (
                [
                    framed_record(b"hdrcharset", b"unknown"),
                    b"bad frame".to_vec(),
                ]
                .concat(),
                PaxIngestError::UnsupportedValue,
            ),
            (
                framed_record(b"unknown", b"bad\0value"),
                PaxIngestError::UnsupportedKey,
            ),
        ];
        for (stream, expected) in cases {
            assert_eq!(ingest(&stream), Err(expected), "{stream:?}");
        }
    }

    #[test]
    fn effective_pax_size_selects_override_fallback_and_invalid_values() {
        assert_eq!(effective_size(None, 17), Ok(17));
        assert_eq!(effective_size(Some(b"0"), 17), Ok(0));
        assert_eq!(effective_size(Some(b"123"), 17), Ok(123));
        assert_eq!(effective_size(Some(b""), 17), Err(PaxSizeError::Invalid));
        assert_eq!(
            effective_size(Some(b"18446744073709551616"), 17),
            Err(PaxSizeError::Invalid)
        );
        assert_eq!(effective_size(Some(b"1x"), 17), Err(PaxSizeError::Invalid));
    }

    #[test]
    fn effective_name_override_prefers_pax_then_gnu_then_header_fallback() {
        assert_eq!(effective_name_override(None, None), None);
        assert_eq!(
            effective_name_override(None, Some(b"gnu")),
            Some(&b"gnu"[..])
        );
        assert_eq!(
            effective_name_override(Some(b"pax"), None),
            Some(&b"pax"[..])
        );
        assert_eq!(
            effective_name_override(Some(b"pax"), Some(b"gnu")),
            Some(&b"pax"[..])
        );
    }

    #[test]
    fn effective_link_override_prefers_pax_then_gnu_then_header_fallback() {
        assert_eq!(effective_link_override(None, None), None);
        assert_eq!(
            effective_link_override(None, Some(b"gnu")),
            Some(&b"gnu"[..])
        );
        assert_eq!(
            effective_link_override(Some(b"pax"), None),
            Some(&b"pax"[..])
        );
        assert_eq!(
            effective_link_override(Some(b"pax"), Some(b"gnu")),
            Some(&b"pax"[..])
        );
    }

    #[test]
    fn effective_owner_name_and_charset_policy_select_exact_sources() {
        assert_eq!(effective_owner_name(None, b"header"), b"header");
        assert_eq!(effective_owner_name(Some(b"pax"), b"header"), b"pax");
        assert!(owner_name_requires_utf8(None));
        assert!(owner_name_requires_utf8(Some(b"UTF-8")));
        assert!(owner_name_requires_utf8(Some(b"ISO-IR 10646 2000 UTF-8")));
        assert!(!owner_name_requires_utf8(Some(b"BINARY")));
    }

    //= docs/parsing-requirements.md#parse-tar-003
    //= type=test
    //# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
    #[test]
    fn resolve_values_composes_inherit_delete_and_override_for_all_slots() {
        let global = [Some(&b"global"[..]); 11];
        let inherit = resolve_values(global, [None; 11]);
        let delete = resolve_values(global, [Some(&b""[..]); 11]);
        let override_local = resolve_values(global, [Some(&b"local"[..]); 11]);

        for i in 0..11 {
            assert_eq!(inherit[i], Some(&b"global"[..]), "inherit slot {i}");
            assert_eq!(delete[i], None, "delete slot {i}");
            assert_eq!(override_local[i], Some(&b"local"[..]), "override slot {i}");
        }
    }

    //= docs/parsing-requirements.md#parse-tar-003
    //= type=test
    //# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
    #[test]
    fn hdrcharset_value_admission_matches_exact_supported_tokens() {
        for value in [b"BINARY" as &[u8], b"UTF-8", b"ISO-IR 10646 2000 UTF-8"] {
            assert!(hdrcharset_value_supported(value));
        }
        for value in [
            b"binary" as &[u8],
            b"UTF8",
            b"ISO-IR 10646 2000 UTF-8 ",
            b"ASCII",
            b"unknown",
            b"",
        ] {
            assert!(!hdrcharset_value_supported(value));
        }
    }

    #[test]
    fn pax_record_admission_checks_index_duplicate_nul_and_preserves_empty_values() {
        let keys: [(&[u8], usize); 11] = [
            (b"path", 0),
            (b"linkpath", 1),
            (b"size", 2),
            (b"mtime", 3),
            (b"uid", 4),
            (b"gid", 5),
            (b"uname", 6),
            (b"gname", 7),
            (b"atime", 8),
            (b"ctime", 9),
            (b"hdrcharset", 10),
        ];
        for (key, expected_index) in keys {
            assert_eq!(admit_record(key, b"", false), Ok(expected_index));
            assert_eq!(
                admit_record(key, b"value", true),
                Err(PaxRecordAdmissionError::Malformed)
            );
            assert_eq!(
                admit_record(key, b"value\0tail", false),
                Err(PaxRecordAdmissionError::Malformed)
            );
        }
        assert_eq!(
            admit_record(b"unknown", b"", false),
            Err(PaxRecordAdmissionError::UnsupportedKey)
        );
        assert_eq!(
            admit_record(b"unknown", b"bad\0value", false),
            Err(PaxRecordAdmissionError::UnsupportedKey)
        );
    }

    #[test]
    fn effective_pax_ids_select_fallback_and_exact_decimal_values() {
        assert_eq!(effective_id(None, 17), Ok(17));
        assert_eq!(effective_id(Some(b"0"), 17), Ok(0));
        assert_eq!(
            effective_id(Some(b"18446744073709551615"), 17),
            Ok(u64::MAX)
        );
        assert_eq!(effective_id(Some(b""), 17), Err(PaxIdError::Invalid));
        assert_eq!(effective_id(Some(b"1x"), 17), Err(PaxIdError::Invalid));
        assert_eq!(
            effective_id(Some(b"18446744073709551616"), 17),
            Err(PaxIdError::Invalid)
        );
    }

    #[test]
    fn effective_pax_mtime_selects_header_fallback_and_verified_time_values() {
        assert_eq!(effective_mtime(None, -7), Ok((-7, 0)));
        assert_eq!(
            effective_mtime(Some(b"-0.000000001"), 7),
            Ok((-1, 999_999_999))
        );
        assert_eq!(
            effective_mtime(Some(b"+0.100000000000"), 7),
            Ok((0, 100_000_000))
        );
        for value in [
            b"1.1234567891" as &[u8],
            b"1.",
            b"18446744073709551616",
            b"-9223372036854775808.000000001",
        ] {
            assert_eq!(effective_mtime(Some(value), 7), Err(PaxTimeError::Invalid));
        }
    }

    #[test]
    fn optional_pax_times_preserve_absence_and_use_verified_time_values() {
        assert_eq!(effective_optional_time(None), Ok(None));
        assert_eq!(
            effective_optional_time(Some(b"-0.000000001")),
            Ok(Some((-1, 999_999_999)))
        );
        assert_eq!(
            effective_optional_time(Some(b"+0.100000000000")),
            Ok(Some((0, 100_000_000)))
        );
        for value in [b"1.1234567891" as &[u8], b"1.", b"18446744073709551616"] {
            assert_eq!(
                effective_optional_time(Some(value)),
                Err(PaxTimeError::Invalid)
            );
        }
    }
}
