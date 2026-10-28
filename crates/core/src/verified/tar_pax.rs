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
{
    match local {
        None => global,
        Some(value) => if value.is_empty() { None } else { Some(value) },
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
}
