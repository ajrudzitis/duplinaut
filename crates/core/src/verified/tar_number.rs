//! Exact octal and reviewed GNU base256 semantics, including checked i128
//! intermediate multiplication/addition. No broader base256 profile is implied.
// Explicit scalar operations preserve the Verus contracts and solver-visible bounds.
#![allow(clippy::len_zero, clippy::manual_range_contains)]
use vstd::prelude::*;
verus! {
pub open spec fn pad(byte: u8) -> bool { byte == 0 || byte == b' ' }
pub open spec fn first(input: Seq<u8>, i: int) -> int
    recommends 0 <= i <= input.len(),
    decreases input.len() - i,
{
    if i >= input.len() { input.len() as int }
    else if pad(input[i]) { first(input, i + 1) } else { i }
}
pub open spec fn last(input: Seq<u8>, i: int) -> int
    recommends 0 <= i <= input.len(),
    decreases i,
{
    if i <= 0 { 0 }
    else if pad(input[i - 1]) { last(input, i - 1) } else { i }
}
pub open spec fn fold(input: Seq<u8>, begin: int, end: int, binary: bool,
    initial: int) -> Option<int>
    recommends 0 <= begin <= end <= input.len(),
    decreases end - begin,
{
    if end <= begin { Some(initial) }
    else { match fold(input, begin, end - 1, binary, initial) {
        None => None,
        Some(v) => {
            let byte = input[end - 1];
            let radix: int = if binary { 256 } else { 8 };
            let digit = if binary { byte as int } else { byte as int - 48 };
            let multiplied = v * radix;
            if (!binary && (byte < b'0' || byte > b'7'))
                || multiplied < i128::MIN as int || multiplied > i128::MAX as int
                || multiplied + digit < i128::MIN as int || multiplied + digit > i128::MAX as int {
                None
            } else { Some(multiplied + digit) }
        },
    }}
}
pub open spec fn number_model(input: Seq<u8>) -> Option<int> {
    if input.len() > 0 && (input[0] == 0x80 || input[0] == 0xff) {
        fold(input, 1, input.len() as int, true, if input[0] == 0xff { -1 } else { 0 })
    } else {
        let begin = first(input, 0);
        let end = last(input, input.len() as int);
        fold(input, begin, if end < begin { begin } else { end }, false, 0)
    }
}
pub proof fn fold_failed_suffix(input: Seq<u8>, begin: int, middle: int, end: int,
    binary: bool, initial: int)
    requires 0 <= begin <= middle <= end <= input.len(),
        fold(input, begin, middle, binary, initial) is None,
    ensures fold(input, begin, end, binary, initial) is None,
    decreases end - middle,
{
    if end > middle { fold_failed_suffix(input, begin, middle, end - 1, binary, initial); }
}
pub fn fold_number(input: &[u8], begin: usize, end: usize, binary: bool, initial: i128)
    -> (result: Option<i128>)
    requires begin <= end <= input.len(),
    ensures match result {
        Some(v) => fold(input@, begin as int, end as int, binary, initial as int) == Some(v as int),
        None => fold(input@, begin as int, end as int, binary, initial as int) is None,
    },
{
    let mut value = initial;
    let mut i = begin;
    while i < end
        invariant begin <= i <= end <= input.len(),
            fold(input@, begin as int, i as int, binary, initial as int) == Some(value as int),
        decreases end - i,
    {
        let byte = input[i];
        if !binary && (byte < b'0' || byte > b'7') {
            proof { fold_failed_suffix(input@, begin as int, i as int + 1, end as int, binary, initial as int); }
            return None;
        }
        let radix = if binary { 256i128 } else { 8i128 };
        let digit = if binary { byte as i128 } else { (byte - b'0') as i128 };
        let multiplied = match value.checked_mul(radix) {
            Some(v) => v,
            None => {
                proof { fold_failed_suffix(input@, begin as int, i as int + 1, end as int, binary, initial as int); }
                return None;
            },
        };
        value = match multiplied.checked_add(digit) {
            Some(v) => v,
            None => {
                proof { fold_failed_suffix(input@, begin as int, i as int + 1, end as int, binary, initial as int); }
                return None;
            },
        };
        i += 1;
    }
    Some(value)
}
pub proof fn first_bounds(input: Seq<u8>, i: int)
    requires 0 <= i <= input.len(),
    ensures i <= first(input, i) <= input.len(),
    decreases input.len() - i,
{
    if i < input.len() && pad(input[i]) { first_bounds(input, i + 1); }
}
pub proof fn last_bounds(input: Seq<u8>, i: int)
    requires 0 <= i <= input.len(),
    ensures 0 <= last(input, i) <= i,
    decreases i,
{
    if i > 0 && pad(input[i - 1]) { last_bounds(input, i - 1); }
}
//= docs/parsing-requirements.md#parse-tar-026
//# The reader MUST decode a tar numeric field as either padded ASCII octal or the reviewed GNU base-256 profile. For octal, it MUST trim only leading and trailing NUL or ASCII-space padding, interpret an all-padding field as zero, and accept only bytes `0` through `7` in the remaining digits. For base-256, it MUST recognize only a first byte of `0x80` (initial accumulator zero) or `0xff` (initial accumulator negative one), then fold each following byte in big-endian base 256. Every multiply and addition MUST be checked against the `i128` range; unsupported markers, invalid octal digits, or overflow MUST produce Numeric. Fields used as unsigned values MUST additionally reject negative results and results greater than `u64::MAX`; signed mtime narrowing remains governed by PARSE-TAR-011.
pub fn number(input: &[u8]) -> (result: Option<i128>)
    ensures match result {
        Some(v) => number_model(input@) == Some(v as int),
        None => number_model(input@) is None,
    },
{
    if input.len() > 0 && (input[0] == 0x80 || input[0] == 0xff) {
        let initial = if input[0] == 0xff { -1i128 } else { 0i128 };
        return fold_number(input, 1, input.len(), true, initial);
    }
    let mut begin = 0usize;
    while begin < input.len() && (input[begin] == 0 || input[begin] == b' ')
        invariant begin <= input.len(), first(input@, begin as int) == first(input@, 0),
        decreases input.len() - begin,
    { begin += 1; }
    assert(first(input@, 0) == begin as int);
    let mut end = input.len();
    while end > begin && (input[end - 1] == 0 || input[end - 1] == b' ')
        invariant begin <= end <= input.len(), first(input@, 0) == begin as int,
            last(input@, end as int) == last(input@, input.len() as int),
        decreases end - begin,
    { end -= 1; }
    proof {
        last_bounds(input@, end as int);
        assert(end as int == if last(input@, input.len() as int) < begin as int {
            begin as int
        } else { last(input@, input.len() as int) });
    }
    fold_number(input, begin, end, false, 0)
}
pub open spec fn unsigned_model(input: Seq<u8>) -> Option<int> {
    match number_model(input) {
        Some(v) => if 0 <= v <= u64::MAX as int { Some(v) } else { None },
        None => None,
    }
}
//= docs/parsing-requirements.md#parse-tar-026
//# The reader MUST decode a tar numeric field as either padded ASCII octal or the reviewed GNU base-256 profile. For octal, it MUST trim only leading and trailing NUL or ASCII-space padding, interpret an all-padding field as zero, and accept only bytes `0` through `7` in the remaining digits. For base-256, it MUST recognize only a first byte of `0x80` (initial accumulator zero) or `0xff` (initial accumulator negative one), then fold each following byte in big-endian base 256. Every multiply and addition MUST be checked against the `i128` range; unsupported markers, invalid octal digits, or overflow MUST produce Numeric. Fields used as unsigned values MUST additionally reject negative results and results greater than `u64::MAX`; signed mtime narrowing remains governed by PARSE-TAR-011.
pub fn unsigned(input: &[u8]) -> (result: Option<u64>)
    ensures match result {
        Some(v) => unsigned_model(input@) == Some(v as int),
        None => unsigned_model(input@) is None,
    },
{
    match number(input) {
        Some(v) => if v < 0 || v > u64::MAX as i128 { None } else { Some(v as u64) },
        None => None,
    }
}

//= docs/parsing-requirements.md#parse-tar-011
//# Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.
//= docs/parsing-requirements.md#parse-tar-030
//# For each accepted real tar entry, the reader MUST preserve the decoded raw header `mode` after the `u32` admission required by PARSE-TAR-011. A PAX `mode` key MUST be rejected as UnsupportedPax; it MUST NOT replace the raw header mode.
pub fn narrow_mode(value: u64) -> (result: Option<u32>)
    ensures match result {
        Some(mode) => mode as u64 == value && value <= u32::MAX as u64,
        None => value > u32::MAX as u64,
    },
{
    if value > u32::MAX as u64 { None } else { Some(value as u32) }
}

//= docs/parsing-requirements.md#parse-tar-011
//# Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.
pub fn narrow_mtime(value: i128) -> (result: Option<i64>)
    ensures match result {
        Some(mtime) => mtime as i128 == value
            && i64::MIN as i128 <= value <= i64::MAX as i128,
        None => value < i64::MIN as i128 || value > i64::MAX as i128,
    },
{
    if value < i64::MIN as i128 || value > i64::MAX as i128 {
        None
    } else {
        Some(value as i64)
    }
}
}

#[cfg(test)]
mod tests {
    use super::{narrow_mode, narrow_mtime, number, unsigned};

    //= docs/parsing-requirements.md#parse-tar-026
    //= type=test
    //# The reader MUST decode a tar numeric field as either padded ASCII octal or the reviewed GNU base-256 profile. For octal, it MUST trim only leading and trailing NUL or ASCII-space padding, interpret an all-padding field as zero, and accept only bytes `0` through `7` in the remaining digits. For base-256, it MUST recognize only a first byte of `0x80` (initial accumulator zero) or `0xff` (initial accumulator negative one), then fold each following byte in big-endian base 256. Every multiply and addition MUST be checked against the `i128` range; unsupported markers, invalid octal digits, or overflow MUST produce Numeric. Fields used as unsigned values MUST additionally reject negative results and results greater than `u64::MAX`; signed mtime narrowing remains governed by PARSE-TAR-011.
    #[test]
    fn numeric_byte_decoding_matches_profile_boundaries() {
        assert_eq!(number(b""), Some(0));
        assert_eq!(number(b" \0 17\0 "), Some(15));
        assert_eq!(number(b"0007\0"), Some(7));
        assert_eq!(number(&[0x80, 0x01, 0x02]), Some(258));
        assert_eq!(number(&[0xff, 0xff, 0xfe]), Some(-2));
        assert_eq!(number(&[0xff]), Some(-1));

        for invalid in [
            &b"8"[..],
            &b"19"[..],
            &[b'1', 0, b'2'][..],
            &b"1 2"[..],
            &[0x81, b'0'][..],
            &[0x7f, b'0'][..],
            &[0x90, 0, 0][..],
        ] {
            assert_eq!(number(invalid), None, "accepted {invalid:?}");
        }

        let mut positive_overflow = vec![0x80];
        positive_overflow.extend([0xff; 17]);
        assert_eq!(number(&positive_overflow), None);
        let mut negative_overflow = vec![0xff];
        negative_overflow.extend([0x00; 17]);
        assert_eq!(number(&negative_overflow), None);
        assert_eq!(number(&[b'7'; 50]), None);
    }

    //= docs/parsing-requirements.md#parse-tar-026
    //= type=test
    //# The reader MUST decode a tar numeric field as either padded ASCII octal or the reviewed GNU base-256 profile. For octal, it MUST trim only leading and trailing NUL or ASCII-space padding, interpret an all-padding field as zero, and accept only bytes `0` through `7` in the remaining digits. For base-256, it MUST recognize only a first byte of `0x80` (initial accumulator zero) or `0xff` (initial accumulator negative one), then fold each following byte in big-endian base 256. Every multiply and addition MUST be checked against the `i128` range; unsupported markers, invalid octal digits, or overflow MUST produce Numeric. Fields used as unsigned values MUST additionally reject negative results and results greater than `u64::MAX`; signed mtime narrowing remains governed by PARSE-TAR-011.
    #[test]
    fn unsigned_numeric_admission_is_bounded_by_u64() {
        let max_octal = format!("{:o}", u64::MAX);
        assert_eq!(unsigned(max_octal.as_bytes()), Some(u64::MAX));
        assert_eq!(
            unsigned(&[0x80, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Some(u64::MAX)
        );
        assert_eq!(unsigned(&[0x80, 1, 0, 0, 0, 0, 0, 0, 0, 0]), None);
        assert_eq!(unsigned(&[0xff]), None);
        assert_eq!(unsigned(b"20000000000000000000000"), None);
    }

    #[test]
    fn mode_narrowing_accepts_exact_u32_bounds_and_rejects_overflow() {
        assert_eq!(narrow_mode(0), Some(0));
        assert_eq!(narrow_mode(u32::MAX as u64), Some(u32::MAX));
        assert_eq!(narrow_mode(u32::MAX as u64 + 1), None);
        assert_eq!(narrow_mode(u64::MAX), None);
    }

    #[test]
    fn mtime_narrowing_accepts_exact_i64_bounds_and_rejects_overflow() {
        assert_eq!(narrow_mtime(i64::MIN as i128), Some(i64::MIN));
        assert_eq!(narrow_mtime(i64::MAX as i128), Some(i64::MAX));
        assert_eq!(narrow_mtime(i64::MIN as i128 - 1), None);
        assert_eq!(narrow_mtime(i64::MAX as i128 + 1), None);
    }
}
