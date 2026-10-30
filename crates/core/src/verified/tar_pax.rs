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

//= docs/parsing-requirements.md#parse-tar-010
//# Raw tar text fields MUST end at the first NUL byte, or use the entire field when no NUL is present.
pub open spec fn nul_terminated_prefix_model(input: Seq<u8>) -> Seq<u8> {
    match find(input, 0, input.len() as int, 0) {
        Some(end) => input.subrange(0, end),
        None => input,
    }
}

//= docs/parsing-requirements.md#parse-tar-010
//# Raw tar text fields MUST end at the first NUL byte, or use the entire field when no NUL is present.
pub fn nul_terminated_prefix<'a>(input: &'a [u8]) -> (result: &'a [u8])
    ensures result@ == nul_terminated_prefix_model(input@),
{
    match find_byte(input, 0, input.len(), 0) {
        Some(end) => &input[..end],
        None => input,
    }
}

//= docs/parsing-requirements.md#parse-tar-040
//# Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.
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
//= docs/parsing-requirements.md#parse-tar-040
//# Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.
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

//= docs/parsing-requirements.md#parse-tar-005
//# A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling.
pub fn update_pending_override(pending: bool, header_type: u8) -> (result: bool)
    ensures result == (pending || header_type == b'x' || header_type == b'L' || header_type == b'K'),
{
    pending || header_type == b'x' || header_type == b'L' || header_type == b'K'
}

//= docs/parsing-requirements.md#parse-tar-005
//# A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling.
pub fn clear_pending_override(_pending: bool) -> (result: bool)
    ensures result == false,
{
    false
}

//= docs/parsing-requirements.md#parse-tar-005
//# A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling.
pub fn dangling_override_at_eof(pending: bool) -> (result: bool)
    ensures result == pending,
{
    pending
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

//= docs/parsing-requirements.md#parse-tar-009
//# When PAX path and GNU long-name overrides are absent, a POSIX ustar header MUST use its raw name field if the prefix is empty and MUST join a nonempty prefix to the raw name with one slash byte.
pub open spec fn ustar_header_name_model(prefix: Seq<u8>, raw_name: Seq<u8>) -> Seq<u8> {
    if prefix.len() == 0 { raw_name }
    else { prefix + seq![b'/'] + raw_name }
}

pub open spec fn ustar_header_name_prefix_model(
    prefix: Seq<u8>, raw_name: Seq<u8>, copied: int,
) -> Seq<u8> {
    if prefix.len() == 0 { raw_name.subrange(0, copied) }
    else { prefix + seq![b'/'] + raw_name.subrange(0, copied) }
}

//= docs/parsing-requirements.md#parse-tar-009
//# When PAX path and GNU long-name overrides are absent, a POSIX ustar header MUST use its raw name field if the prefix is empty and MUST join a nonempty prefix to the raw name with one slash byte.
pub fn compose_ustar_header_name(prefix: &[u8], raw_name: &[u8]) -> (result: Vec<u8>)
    requires (prefix.len() as int) + (raw_name.len() as int)
        + (if prefix.len() == 0 { 0int } else { 1int }) <= usize::MAX as int,
    ensures result@ == ustar_header_name_model(prefix@, raw_name@),
{
    let capacity = if prefix.is_empty() {
        raw_name.len()
    } else {
        prefix.len() + 1 + raw_name.len()
    };
    let mut result = Vec::with_capacity(capacity);
    let mut i = 0;
    while i < prefix.len()
        invariant 0 <= i <= prefix.len(),
            result@ == prefix@.subrange(0, i as int),
        decreases prefix.len() - i,
    {
        result.push(prefix[i]);
        i += 1;
    }
    if prefix.len() > 0 {
        result.push(b'/');
    }
    let mut i = 0;
    while i < raw_name.len()
        invariant 0 <= i <= raw_name.len(),
            result@ == ustar_header_name_prefix_model(prefix@, raw_name@, i as int),
        decreases raw_name.len() - i,
    {
        result.push(raw_name[i]);
        i += 1;
    }
    assert(ustar_header_name_prefix_model(prefix@, raw_name@, raw_name.len() as int)
        == ustar_header_name_model(prefix@, raw_name@));
    result
}

//= docs/parsing-requirements.md#parse-tar-012
//# The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.
pub open spec fn effective_header_name_length_model(
    selected_override: Option<Seq<u8>>,
    prefix: Seq<u8>,
    raw_name: Seq<u8>,
) -> int {
    match selected_override {
        Some(name) => name.len() as int,
        None => ustar_header_name_model(prefix, raw_name).len() as int,
    }
}

//= docs/parsing-requirements.md#parse-tar-015
//# For each accepted tar entry, the output name MUST be the selected PAX path, GNU long-name, or raw ustar prefix/name, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
pub open spec fn effective_header_name_model(
    selected_override: Option<Seq<u8>>,
    prefix: Seq<u8>,
    raw_name: Seq<u8>,
) -> Seq<u8> {
    match selected_override {
        Some(name) => name,
        None => ustar_header_name_model(prefix, raw_name),
    }
}

//= docs/parsing-requirements.md#parse-tar-015
//# For each accepted tar entry, the output name MUST be the selected PAX path, GNU long-name, or raw ustar prefix/name, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
pub fn compose_effective_header_name(
    selected_override: Option<&[u8]>,
    prefix: &[u8],
    raw_name: &[u8],
    expected_length: usize,
) -> (result: Vec<u8>)
    requires expected_length as int == effective_header_name_length_model(
        match selected_override {
            None => None,
            Some(name) => Some(name@),
        }, prefix@, raw_name@,
    ),
    ensures result@ == effective_header_name_model(
        match selected_override {
            None => None,
            Some(name) => Some(name@),
        }, prefix@, raw_name@,
    ) && result.len() == expected_length,
{
    match selected_override {
        Some(name) => {
            let mut result = Vec::with_capacity(expected_length);
            let mut i = 0;
            while i < name.len()
                invariant 0 <= i <= name.len(),
                    result@ == name@.subrange(0, i as int),
                decreases name.len() - i,
            {
                result.push(name[i]);
                i += 1;
            }
            result
        },
        None => compose_ustar_header_name(prefix, raw_name),
    }
}

//= docs/parsing-requirements.md#parse-tar-012
//# The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.
#[allow(clippy::manual_map)]
pub fn effective_header_name_length_and_admission(
    selected_override: Option<&[u8]>,
    prefix: &[u8],
    raw_name: &[u8],
    max_path_bytes: usize,
) -> (result: Option<(usize, bool)>)
    ensures match result {
        None => effective_header_name_length_model(
            match selected_override {
                None => None,
                Some(name) => Some(name@),
            }, prefix@, raw_name@,
        ) > usize::MAX as int,
        Some((length, admitted)) =>
            length as int == effective_header_name_length_model(
                match selected_override {
                    None => None,
                    Some(name) => Some(name@),
                }, prefix@, raw_name@,
            )
            && admitted == (length <= max_path_bytes),
    },
{
    let length = match selected_override {
        Some(name) => Some(name.len()),
        None => {
            if prefix.is_empty() {
                Some(raw_name.len())
            } else {
                match prefix.len().checked_add(1) {
                    None => None,
                    Some(with_separator) => with_separator.checked_add(raw_name.len()),
                }
            }
        },
    };
    match length {
        None => None,
        Some(length) => Some((length, length <= max_path_bytes)),
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

//= docs/parsing-requirements.md#parse-tar-013
//# The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.
pub open spec fn effective_link_length_model(
    selected_link: Seq<u8>,
) -> int {
    selected_link.len() as int
}

//= docs/parsing-requirements.md#parse-tar-013
//# The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.
pub fn effective_link_length_and_admission(
    selected_link: &[u8],
    max_path_bytes: usize,
) -> (result: (usize, bool))
    ensures result.0 as int == effective_link_length_model(selected_link@)
        && result.1 == (result.0 <= max_path_bytes),
{
    let length = selected_link.len();
    (length, length <= max_path_bytes)
}

//= docs/parsing-requirements.md#parse-tar-016
//# For each accepted tar entry, the output link target MUST equal the selected PAX `linkpath`, GNU long-link, or raw header `linkname` bytes, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
pub open spec fn effective_link_target_model(selected_link: Seq<u8>) -> Seq<u8> {
    selected_link
}

//= docs/parsing-requirements.md#parse-tar-016
//# For each accepted tar entry, the output link target MUST equal the selected PAX `linkpath`, GNU long-link, or raw header `linkname` bytes, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
pub fn copy_effective_link_target(
    selected_link: &[u8],
    expected_length: usize,
) -> (result: Vec<u8>)
    requires expected_length == selected_link.len(),
    ensures result@ == effective_link_target_model(selected_link@)
        && result.len() == expected_length,
{
    let mut result = Vec::with_capacity(expected_length);
    let mut i = 0;
    while i < selected_link.len()
        invariant 0 <= i <= selected_link.len(),
            result@ == selected_link@.subrange(0, i as int),
        decreases selected_link.len() - i,
    {
        result.push(selected_link[i]);
        i += 1;
    }
    result
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

//= docs/parsing-requirements.md#parse-tar-014
//# The parser MUST compute the exact byte lengths of the selected PAX `uname` and `gname` values, or their raw header fallbacks, and reject values above `max_path_bytes` before allocating the output owner names.
pub open spec fn effective_owner_name_length_model(selected_owner_name: Seq<u8>) -> int {
    selected_owner_name.len() as int
}

//= docs/parsing-requirements.md#parse-tar-014
//# The parser MUST compute the exact byte lengths of the selected PAX `uname` and `gname` values, or their raw header fallbacks, and reject values above `max_path_bytes` before allocating the output owner names.
pub fn effective_owner_name_length_and_admission(
    selected_owner_name: &[u8],
    max_path_bytes: usize,
) -> (result: (usize, bool))
    ensures result.0 as int == effective_owner_name_length_model(selected_owner_name@)
        && result.1 == (result.0 <= max_path_bytes),
{
    let length = selected_owner_name.len();
    (length, length <= max_path_bytes)
}

//= docs/parsing-requirements.md#parse-tar-017
//# For each accepted tar entry, output `uname` and `gname` bytes MUST equal their selected PAX override or raw header fallback, and each output length MUST equal the precomputed length used for admission and metadata accounting.
pub fn copy_effective_owner_name(
    selected_owner_name: &[u8],
    expected_length: usize,
) -> (result: Vec<u8>)
    requires expected_length == selected_owner_name.len(),
    ensures result@ == selected_owner_name@ && result.len() == expected_length,
{
    let mut result = Vec::with_capacity(expected_length);
    let mut i = 0;
    while i < selected_owner_name.len()
        invariant 0 <= i <= selected_owner_name.len(),
            result@ == selected_owner_name@.subrange(0, i as int),
        decreases selected_owner_name.len() - i,
    {
        result.push(selected_owner_name[i]);
        i += 1;
    }
    result
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Return whether the resolved charset uses the parser's UTF-8 text policy.
//= docs/parsing-requirements.md#parse-tar-041
//# For a real entry, each present resolved PAX `uname` and `gname` value MUST be validated as UTF-8 when the effective `hdrcharset` is absent or selects a supported UTF-8 token (`UTF-8` or `ISO-IR 10646 2000 UTF-8`). Invalid bytes MUST produce TextEncoding. When the effective `hdrcharset` is `BINARY`, PAX owner-name values MUST be preserved byte-for-byte without UTF-8 validation. When a resolved PAX owner-name value is absent, the corresponding raw-header fallback MUST be preserved byte-for-byte and MUST NOT be validated under the PAX charset.
//= docs/parsing-requirements.md#parse-tar-042
//# For a real entry, each nonempty resolved PAX `path` and `linkpath` value MUST be validated as UTF-8 when the effective `hdrcharset` is absent or selects a supported UTF-8 token (`UTF-8` or `ISO-IR 10646 2000 UTF-8`). Invalid bytes MUST produce `TextEncoding`. When the effective `hdrcharset` is `BINARY`, selected PAX `path` and `linkpath` values MUST be retained byte-for-byte without UTF-8 validation. When a resolved PAX path or link override is absent, including after an empty-value deletion, the next selection source (GNU long-name/long-link or raw header field) MUST remain byte-preserving and MUST NOT be validated under the PAX charset. This requirement does not relax NUL rejection for PAX records.
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
//= docs/parsing-requirements.md#parse-tar-044
//# Each nonempty PAX `hdrcharset` record value MUST exactly equal one of the ASCII byte strings `BINARY`, `UTF-8`, or `ISO-IR 10646 2000 UTF-8`. Any other nonempty value MUST produce `UnsupportedPax`; matching MUST be byte-exact and MUST NOT trim, case-fold, or otherwise normalize the value. An empty `hdrcharset` value remains a deletion marker under PARSE-TAR-006 and MUST NOT be rejected as an unsupported token.
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
//= docs/parsing-requirements.md#parse-tar-036
//# After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.
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

//= docs/parsing-requirements.md#parse-tar-007
//# GNU and PAX extension headers MUST use the raw size in their own tar header to determine their payload extent.
//# A real entry MUST use the effective PAX `size` value when present, or its raw tar header size when absent.
pub fn payload_size(is_extension: bool, value: Option<&[u8]>, header_size: u64)
    -> (result: Result<u64, PaxSizeError>)
    ensures if is_extension {
        result == Ok(header_size)
    } else {
        match value {
            None => result == Ok(header_size),
            Some(bytes) => match result {
                Ok(size) => bytes.len() > 0
                    && decimal_prefix(bytes@, bytes.len() as int) == Some(size as int),
                Err(error) => error == PaxSizeError::Invalid
                    && (bytes.len() == 0 || decimal_prefix(bytes@, bytes.len() as int) is None),
            },
        }
    },
{
    if is_extension { Ok(header_size) } else { effective_size(value, header_size) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaxIdError {
    Invalid,
}

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Select a resolved PAX uid/gid override or retain the raw header value.
//= docs/parsing-requirements.md#parse-tar-036
//# After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.
//= docs/parsing-requirements.md#parse-tar-032
//# For each accepted real tar entry, the reader MUST set `TarEntry.uid` and `TarEntry.gid` from the resolved effective PAX `uid` and `gid` values when present; when either effective value is absent, the reader MUST retain that field's decoded raw header value. Invalid present effective values MUST produce Numeric rather than falling back to the raw header value.
//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.
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

pub type OptionalPaxTime = Option<(i64, u32)>;
pub type EntryOptionalPaxTimes = (OptionalPaxTime, OptionalPaxTime);

//= docs/parsing-requirements.md#parse-tar-003
//# The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

/// Use the resolved PAX mtime when present; otherwise keep the header seconds.
//= docs/parsing-requirements.md#parse-tar-029
//# For each real tar entry, the reader MUST use the effective PAX `mtime` value when present; otherwise it MUST use the decoded raw header mtime seconds with zero nanoseconds. A present but invalid PAX `mtime` MUST produce Numeric and MUST NOT fall back to the raw header timestamp.
//= docs/parsing-requirements.md#parse-tar-037
//# Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.
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
//= docs/parsing-requirements.md#parse-tar-037
//# Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.

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

//= docs/parsing-requirements.md#parse-tar-035
//# For each real tar entry, the reader MUST set `TarEntry.atime` and `TarEntry.ctime` from their respective resolved effective PAX values when present, using the reviewed signed PAX timestamp profile. When an effective value is absent, the corresponding output field MUST be `None`. A present but invalid timestamp MUST produce Numeric; it MUST NOT be treated as absent. An empty local value MUST delete the corresponding inherited global value, so that field MUST be `None` for the next real entry.
pub fn effective_entry_optional_times(
    atime: Option<&[u8]>,
    ctime: Option<&[u8]>,
) -> (result: Result<EntryOptionalPaxTimes, PaxTimeError>)
    ensures match result {
        Ok((effective_atime, effective_ctime)) =>
            optional_time_matches(atime, effective_atime)
                && optional_time_matches(ctime, effective_ctime),
        Err(error) => error == PaxTimeError::Invalid
            && (match atime {
                None => false,
                Some(bytes) => time_model(bytes@) is None,
            } || match ctime {
                None => false,
                Some(bytes) => time_model(bytes@) is None,
            }),
    },
{
    let effective_atime = match effective_optional_time(atime) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    let effective_ctime = match effective_optional_time(ctime) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    Ok((effective_atime, effective_ctime))
}

pub open spec fn optional_time_matches(
    input: Option<&[u8]>,
    output: OptionalPaxTime,
) -> bool {
    match input {
        None => output is None,
        Some(bytes) => match output {
            None => false,
            Some((seconds, nanoseconds)) =>
                time_model(bytes@) == Some((seconds as int, nanoseconds as int))
                    && nanoseconds < 1000000000,
        },
    }
}

//= docs/parsing-requirements.md#parse-tar-043
//# The reader MUST recognize exactly these PAX key byte strings and map them to their corresponding semantic fields: `path`, `linkpath`, `size`, `mtime`, `uid`, `gid`, `uname`, `gname`, `atime`, `ctime`, and `hdrcharset`. Any other key MUST produce `UnsupportedPax`, including a key defined by POSIX that is outside this reader profile. Key recognition MUST NOT normalize or case-fold input bytes. Empty values for recognized keys remain deletion markers under PARSE-TAR-003/006; this requirement does not change duplicate-key, NUL, or value-grammar handling.
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
//= docs/parsing-requirements.md#parse-tar-040
//# Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.

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
        admit_record, compose_effective_header_name, compose_ustar_header_name,
        copy_effective_link_target, copy_effective_owner_name, effective_entry_optional_times,
        effective_header_name_length_and_admission, effective_id,
        effective_link_length_and_admission, effective_link_override, effective_mtime,
        effective_name_override, effective_optional_time, effective_owner_name,
        effective_owner_name_length_and_admission, effective_size, hdrcharset_value_supported,
        ingest, nul_terminated_prefix, owner_name_requires_utf8, payload_size, record,
        resolve_values, PaxIdError, PaxIngestError, PaxRecordAdmissionError, PaxSizeError,
        PaxTimeError,
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
    //= docs/parsing-requirements.md#parse-tar-040
    //= type=test
    //# Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.
    //= docs/parsing-requirements.md#parse-tar-043
    //= type=test
    //# The reader MUST recognize exactly these PAX key byte strings and map them to their corresponding semantic fields: `path`, `linkpath`, `size`, `mtime`, `uid`, `gid`, `uname`, `gname`, `atime`, `ctime`, and `hdrcharset`. Any other key MUST produce `UnsupportedPax`, including a key defined by POSIX that is outside this reader profile. Key recognition MUST NOT normalize or case-fold input bytes. Empty values for recognized keys remain deletion markers under PARSE-TAR-003/006; this requirement does not change duplicate-key, NUL, or value-grammar handling.
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

    //= docs/parsing-requirements.md#parse-tar-040
    //= type=test
    //# Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.
    #[test]
    fn record_lengths_include_lf_accept_leading_zeroes_and_consume_contiguously() {
        let canonical = framed_record(b"path", b"snapshot/padded=tail");
        let space = canonical.iter().position(|byte| *byte == b' ').unwrap();
        let mut length = (canonical.len() + 1).to_string().into_bytes();
        length.insert(0, b'0');
        length.push(b' ');
        length.extend_from_slice(&canonical[space + 1..]);
        assert_eq!(length.len(), canonical.len() + 1);

        let next = framed_record(b"size", b"7");
        let stream = [length.clone(), next.clone()].concat();
        let (first_end, key, value) = record(&stream, 0).expect("zero-padded length is valid");
        assert_eq!(first_end, length.len());
        assert_eq!(key, b"path");
        assert_eq!(value, b"snapshot/padded=tail");
        let (second_end, second_key, second_value) =
            record(&stream, first_end).expect("second record begins at the first end");
        assert_eq!(second_end, stream.len());
        assert_eq!(second_key, b"size");
        assert_eq!(second_value, b"7");
        let slots = ingest(&stream).expect("all record bytes are consumed contiguously");
        assert_eq!(slots[0], Some(&b"snapshot/padded=tail"[..]));
        assert_eq!(slots[2], Some(&b"7"[..]));

        assert_eq!(record(&length[..length.len() - 1], 0), None);
        assert_eq!(
            ingest(&length[..length.len() - 1]),
            Err(PaxIngestError::Framing)
        );
        assert_eq!(record(b"0 x=y\n", 0), None);
        assert_eq!(ingest(b"0 x=y\n"), Err(PaxIngestError::Framing));
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
    fn pending_override_latch_distinguishes_local_extensions_from_global() {
        for header_type in *b"xLK" {
            assert!(super::update_pending_override(false, header_type));
            assert!(super::update_pending_override(true, header_type));
        }
        assert!(!super::update_pending_override(false, b'g'));
        assert!(super::update_pending_override(true, b'g'));
        assert!(!super::clear_pending_override(true));
        assert!(!super::clear_pending_override(false));
        assert!(super::dangling_override_at_eof(true));
        assert!(!super::dangling_override_at_eof(false));
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
    //= docs/parsing-requirements.md#parse-tar-036
    //= type=test
    //# After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.
    fn effective_pax_size_selects_override_fallback_and_invalid_values() {
        assert_eq!(effective_size(None, 17), Ok(17));
        assert_eq!(effective_size(Some(b"0"), 17), Ok(0));
        assert_eq!(effective_size(Some(b"000"), 17), Ok(0));
        assert_eq!(effective_size(Some(b"123"), 17), Ok(123));
        assert_eq!(
            effective_size(Some(b"18446744073709551615"), 17),
            Ok(u64::MAX)
        );
        assert_eq!(effective_size(Some(b""), 17), Err(PaxSizeError::Invalid));
        for invalid in [
            b"+1" as &[u8],
            b"-1",
            b" 1",
            b"1 ",
            b"1x",
            b"\xff",
            b"18446744073709551616",
        ] {
            assert_eq!(
                effective_size(Some(invalid), 17),
                Err(PaxSizeError::Invalid)
            );
        }
    }

    #[test]
    fn payload_size_uses_raw_extension_size_and_effective_entry_size() {
        assert_eq!(payload_size(true, Some(b"1"), 17), Ok(17));
        assert_eq!(payload_size(false, Some(b"1"), 17), Ok(1));
        assert_eq!(payload_size(false, None, 17), Ok(17));
        assert_eq!(
            payload_size(false, Some(b""), 17),
            Err(PaxSizeError::Invalid)
        );
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
    fn ustar_header_name_composition_preserves_empty_and_nonempty_prefix_rules() {
        assert_eq!(compose_ustar_header_name(b"", b"file"), b"file");
        assert_eq!(
            compose_ustar_header_name(b"snapshot/", b"file"),
            b"snapshot//file"
        );
    }

    #[test]
    fn effective_header_name_length_matches_override_or_raw_ustar_and_limit() {
        assert_eq!(
            effective_header_name_length_and_admission(Some(b"pax"), b"prefix", b"raw", 3),
            Some((3, true))
        );
        assert_eq!(
            effective_header_name_length_and_admission(Some(b"gnu-long"), b"prefix", b"raw", 7),
            Some((8, false))
        );
        assert_eq!(
            effective_header_name_length_and_admission(None, b"dir", b"file", 8),
            Some((8, true))
        );
        assert_eq!(
            effective_header_name_length_and_admission(None, b"dir", b"file", 7),
            Some((8, false))
        );
        assert_eq!(
            effective_header_name_length_and_admission(None, b"", b"file", 4),
            Some((4, true))
        );
    }

    //= docs/parsing-requirements.md#parse-tar-015
    //= type=test
    //# For each accepted tar entry, the output name MUST be the selected PAX path, GNU long-name, or raw ustar prefix/name, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
    #[test]
    fn effective_header_name_composition_matches_selected_source_and_length() {
        let pax = effective_name_override(Some(b"pax-path"), Some(b"gnu-path"));
        assert_eq!(
            compose_effective_header_name(pax, b"prefix", b"raw", 8),
            b"pax-path"
        );

        let gnu = effective_name_override(None, Some(b"gnu-path"));
        assert_eq!(
            compose_effective_header_name(gnu, b"prefix", b"raw", 8),
            b"gnu-path"
        );

        let raw = effective_name_override(None, None);
        assert_eq!(
            compose_effective_header_name(raw, b"dir", b"file", 8),
            b"dir/file"
        );
        assert_eq!(compose_effective_header_name(raw, b"", b"file", 4), b"file");
    }

    //= docs/parsing-requirements.md#parse-tar-016
    //= type=test
    //# For each accepted tar entry, the output link target MUST equal the selected PAX `linkpath`, GNU long-link, or raw header `linkname` bytes, and its byte length MUST equal the precomputed length used for admission and metadata accounting.
    #[test]
    fn copied_link_target_matches_selected_pax_gnu_or_header_bytes() {
        let pax = effective_link_override(Some(b"pax-target"), Some(b"gnu-target"))
            .unwrap_or(b"raw-target");
        assert_eq!(copy_effective_link_target(pax, pax.len()), b"pax-target");

        let gnu = effective_link_override(None, Some(b"gnu-target")).unwrap_or(b"raw-target");
        assert_eq!(copy_effective_link_target(gnu, gnu.len()), b"gnu-target");

        let raw = effective_link_override(None, None).unwrap_or(b"raw-target");
        assert_eq!(copy_effective_link_target(raw, raw.len()), b"raw-target");
    }

    #[test]
    fn nul_terminated_prefix_uses_first_zero_or_full_input() {
        assert_eq!(nul_terminated_prefix(b"text\0ignored"), b"text");
        assert_eq!(nul_terminated_prefix(b"text"), b"text");
        assert_eq!(nul_terminated_prefix(b"\0text"), b"");
        assert_eq!(nul_terminated_prefix(b""), b"");
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

    //= docs/parsing-requirements.md#parse-tar-013
    //= type=test
    //# The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.
    #[test]
    fn effective_link_length_matches_selected_source_and_limit() {
        assert_eq!(effective_link_length_and_admission(b"pax", 3), (3, true));
        assert_eq!(
            effective_link_length_and_admission(b"gnu-long", 7),
            (8, false)
        );
        assert_eq!(effective_link_length_and_admission(b"raw", 3), (3, true));
        assert_eq!(effective_link_length_and_admission(b"raw", 2), (3, false));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-tar-041
    //= type=test
    //# For a real entry, each present resolved PAX `uname` and `gname` value MUST be validated as UTF-8 when the effective `hdrcharset` is absent or selects a supported UTF-8 token (`UTF-8` or `ISO-IR 10646 2000 UTF-8`). Invalid bytes MUST produce TextEncoding. When the effective `hdrcharset` is `BINARY`, PAX owner-name values MUST be preserved byte-for-byte without UTF-8 validation. When a resolved PAX owner-name value is absent, the corresponding raw-header fallback MUST be preserved byte-for-byte and MUST NOT be validated under the PAX charset.
    fn effective_owner_name_and_charset_policy_select_exact_sources() {
        assert_eq!(effective_owner_name(None, b"header"), b"header");
        assert_eq!(effective_owner_name(Some(b"pax"), b"header"), b"pax");
        assert!(owner_name_requires_utf8(None));
        assert!(owner_name_requires_utf8(Some(b"UTF-8")));
        assert!(owner_name_requires_utf8(Some(b"ISO-IR 10646 2000 UTF-8")));
        assert!(!owner_name_requires_utf8(Some(b"BINARY")));
    }

    //= docs/parsing-requirements.md#parse-tar-014
    //= type=test
    //# The parser MUST compute the exact byte lengths of the selected PAX `uname` and `gname` values, or their raw header fallbacks, and reject values above `max_path_bytes` before allocating the output owner names.
    #[test]
    fn effective_owner_name_length_matches_selected_value_and_limit() {
        assert_eq!(
            effective_owner_name_length_and_admission(b"pax-user", 8),
            (8, true)
        );
        assert_eq!(
            effective_owner_name_length_and_admission(b"header", 6),
            (6, true)
        );
        assert_eq!(
            effective_owner_name_length_and_admission(b"raw-grp!", 7),
            (8, false)
        );
    }

    //= docs/parsing-requirements.md#parse-tar-017
    //= type=test
    //# For each accepted tar entry, output `uname` and `gname` bytes MUST equal their selected PAX override or raw header fallback, and each output length MUST equal the precomputed length used for admission and metadata accounting.
    #[test]
    fn copied_owner_name_matches_selected_pax_or_header_bytes_and_length() {
        let pax_uname = copy_effective_owner_name(b"pax-user", 8);
        let header_uname = copy_effective_owner_name(b"header-user", 11);
        let pax_gname = copy_effective_owner_name(b"pax-group", 9);
        let header_gname = copy_effective_owner_name(b"header-group", 12);
        assert_eq!(pax_uname, b"pax-user");
        assert_eq!(header_uname, b"header-user");
        assert_eq!(pax_gname, b"pax-group");
        assert_eq!(header_gname, b"header-group");
        assert_eq!(pax_uname.len(), 8);
        assert_eq!(header_uname.len(), 11);
        assert_eq!(pax_gname.len(), 9);
        assert_eq!(header_gname.len(), 12);
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
    //= docs/parsing-requirements.md#parse-tar-044
    //= type=test
    //# Each nonempty PAX `hdrcharset` record value MUST exactly equal one of the ASCII byte strings `BINARY`, `UTF-8`, or `ISO-IR 10646 2000 UTF-8`. Any other nonempty value MUST produce `UnsupportedPax`; matching MUST be byte-exact and MUST NOT trim, case-fold, or otherwise normalize the value. An empty `hdrcharset` value remains a deletion marker under PARSE-TAR-006 and MUST NOT be rejected as an unsupported token.
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

    //= docs/parsing-requirements.md#parse-tar-032
    //= type=test
    //# For each accepted real tar entry, the reader MUST set `TarEntry.uid` and `TarEntry.gid` from the resolved effective PAX `uid` and `gid` values when present; when either effective value is absent, the reader MUST retain that field's decoded raw header value. Invalid present effective values MUST produce Numeric rather than falling back to the raw header value.
    //= docs/parsing-requirements.md#parse-tar-036
    //= type=test
    //# After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.
    #[test]
    fn effective_pax_ids_select_fallback_and_exact_decimal_values() {
        assert_eq!(effective_id(None, 17), Ok(17));
        assert_eq!(effective_id(Some(b"0"), 17), Ok(0));
        assert_eq!(effective_id(Some(b"000"), 17), Ok(0));
        assert_eq!(effective_id(Some(b"123"), 17), Ok(123));
        assert_eq!(
            effective_id(Some(b"18446744073709551615"), 17),
            Ok(u64::MAX)
        );
        assert_eq!(effective_id(Some(b""), 17), Err(PaxIdError::Invalid));
        for invalid in [
            b"+1" as &[u8],
            b"-1",
            b" 1",
            b"1 ",
            b"1x",
            b"\xff",
            b"18446744073709551616",
        ] {
            assert_eq!(effective_id(Some(invalid), 17), Err(PaxIdError::Invalid));
        }
    }

    //= docs/parsing-requirements.md#parse-tar-029
    //= type=test
    //# For each real tar entry, the reader MUST use the effective PAX `mtime` value when present; otherwise it MUST use the decoded raw header mtime seconds with zero nanoseconds. A present but invalid PAX `mtime` MUST produce Numeric and MUST NOT fall back to the raw header timestamp.
    //= docs/parsing-requirements.md#parse-tar-037
    //= type=test
    //# Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.
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

    //= docs/parsing-requirements.md#parse-tar-037
    //= type=test
    //# Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.
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

    //= docs/parsing-requirements.md#parse-tar-035
    //= type=test
    //# For each real tar entry, the reader MUST set `TarEntry.atime` and `TarEntry.ctime` from their respective resolved effective PAX values when present, using the reviewed signed PAX timestamp profile. When an effective value is absent, the corresponding output field MUST be `None`. A present but invalid timestamp MUST produce Numeric; it MUST NOT be treated as absent. An empty local value MUST delete the corresponding inherited global value, so that field MUST be `None` for the next real entry.
    #[test]
    fn effective_entry_times_keep_slots_distinct_and_reject_invalid_values() {
        assert_eq!(
            effective_entry_optional_times(Some(b"-0.000000001"), Some(b"+2.25")),
            Ok((Some((-1, 999_999_999)), Some((2, 250_000_000))))
        );
        assert_eq!(
            effective_entry_optional_times(None, Some(b"3")),
            Ok((None, Some((3, 0))))
        );
        assert_eq!(
            effective_entry_optional_times(Some(b""), None),
            Err(PaxTimeError::Invalid)
        );
        assert_eq!(
            effective_entry_optional_times(None, Some(b"bad")),
            Err(PaxTimeError::Invalid)
        );
    }
}
