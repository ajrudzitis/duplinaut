//! Exact fractional seconds interpretation and precision rejection for PAX.
// Explicit scalar predicates keep arithmetic and bounds visible to Verus.
#![allow(clippy::len_zero, clippy::manual_range_contains, clippy::question_mark)]
use super::tar_helpers::decimal;
#[cfg(verus_keep_ghost)]
use super::tar_helpers::decimal_prefix;
#[cfg(verus_keep_ghost)]
use super::tar_pax::find;
use super::tar_pax::find_byte;
use vstd::prelude::*;
verus! {
pub open spec fn pow10(n: int) -> int
    recommends n >= 0,
    decreases n,
{ if n <= 0 { 1 } else { 10 * pow10(n - 1) } }
pub open spec fn nanos_prefix(input: Seq<u8>, n: int) -> int
    recommends 0 <= n <= 9,
    decreases n,
{
    if n <= 0 { 0 } else {
        nanos_prefix(input, n - 1) * 10
            + if n - 1 < input.len() { input[n - 1] as int - 48 } else { 0 }
    }
}
pub open spec fn fraction_model(input: Seq<u8>) -> Option<int> {
    if input.len() == 0
        || !(forall|i: int| 0 <= i < input.len()
            ==> b'0' <= input[i] <= b'9' && (i < 9 || input[i] == b'0')) { None }
    else { Some(nanos_prefix(input, 9)) }
}
pub proof fn pow10_bounds(n: int)
    requires 0 <= n <= 9,
    ensures 1 <= pow10(n) <= 1000000000,
{
    reveal_with_fuel(pow10, 10);
}
pub fn fraction(input: &[u8]) -> (result: Option<u32>)
    ensures match result {
        Some(v) => fraction_model(input@) == Some(v as int) && v < 1000000000,
        None => fraction_model(input@) is None,
    },
{
    if input.len() == 0 { return None; }
    let mut j = 0usize;
    while j < input.len()
        invariant j <= input.len(), input.len() > 0,
            forall|k: int| 0 <= k < j ==> b'0' <= input@[k] <= b'9' && (k < 9 || input@[k] == b'0'),
        decreases input.len() - j,
    {
        let byte = input[j];
        if byte < b'0' || byte > b'9' || (j >= 9 && byte != b'0') {
            assert(!(forall|k: int| 0 <= k < input.len()
                ==> b'0' <= input@[k] <= b'9' && (k < 9 || input@[k] == b'0')));
            return None;
        }
        j += 1;
    }
    let mut value = 0u64;
    let mut i = 0usize;
    while i < 9
        invariant i <= 9, input.len() > 0,
            forall|k: int| 0 <= k < input.len() ==> b'0' <= input@[k] <= b'9' && (k < 9 || input@[k] == b'0'),
            value as int == nanos_prefix(input@, i as int),
            0 <= (value as int) < pow10(i as int),
        decreases 9 - i,
    {
        proof { pow10_bounds(i as int); }
        let digit = if i < input.len() { (input[i] - b'0') as u64 } else { 0u64 };
        value = value * 10 + digit;
        i += 1;
    }
    proof { pow10_bounds(9); }
    Some(value as u32)
}
pub open spec fn normalize_model(seconds: int, nanos: int, negative: bool) -> Option<(int, int)> {
    let total = (seconds * 1000000000 + nanos) * if negative { -1 } else { 1 };
    let whole = total / 1000000000;
    if whole < i64::MIN as int || whole > i64::MAX as int { None }
    else { Some((whole, total % 1000000000)) }
}
pub fn normalize(seconds: u64, nanos: u32, negative: bool) -> (result: Option<(i64, u32)>)
    requires nanos < 1000000000,
    ensures match result {
        Some((s, n)) => normalize_model(seconds as int, nanos as int, negative) == Some((s as int, n as int))
            && n < 1000000000,
        None => normalize_model(seconds as int, nanos as int, negative) is None,
    },
{
    assert((seconds as int * 1000000000 + nanos as int) / 1000000000 == seconds as int) by(nonlinear_arith)
        requires 0 <= (nanos as int) < 1000000000;
    assert((seconds as int * 1000000000 + nanos as int) % 1000000000 == nanos as int) by(nonlinear_arith)
        requires 0 <= (nanos as int) < 1000000000;
    if !negative {
        if seconds > i64::MAX as u64 { return None; }
        return Some((seconds as i64, nanos));
    }
    assert((seconds as int * 1000000000 + nanos as int) * -1
        == -(seconds as int * 1000000000 + nanos as int)) by(nonlinear_arith);
    if nanos == 0 {
        assert((-((seconds as int) * 1000000000)) / 1000000000 == -(seconds as int)) by(nonlinear_arith);
        assert((-((seconds as int) * 1000000000)) % 1000000000 == 0) by(nonlinear_arith);
        assert(normalize_model(seconds as int, nanos as int, negative)
            == if seconds as int > 9223372036854775808 { None }
                else { Some((-(seconds as int), 0)) });
        if seconds > 9223372036854775808u64 { return None; }
        return Some(((-(seconds as i128)) as i64, 0));
    }
    assert((-(seconds as int * 1000000000 + nanos as int)) / 1000000000 == -(seconds as int) - 1) by(nonlinear_arith)
        requires 0 < (nanos as int) < 1000000000;
    assert((-(seconds as int * 1000000000 + nanos as int)) % 1000000000 == 1000000000 - nanos as int) by(nonlinear_arith)
        requires 0 < (nanos as int) < 1000000000;
    assert(normalize_model(seconds as int, nanos as int, negative)
        == if seconds as int >= 9223372036854775808 { None }
            else { Some((-(seconds as int) - 1, 1000000000 - nanos as int)) });
    if seconds >= 9223372036854775808u64 { return None; }
    Some(((-(seconds as i128) - 1) as i64, 1000000000 - nanos))
}
pub open spec fn time_model(input: Seq<u8>) -> Option<(int, int)> {
    let signed = input.len() > 0 && (input[0] == b'-' || input[0] == b'+');
    let begin: int = if signed { 1 } else { 0 };
    let negative = input.len() > 0 && input[0] == b'-';
    let dot = find(input, begin, input.len() as int, b'.');
    let end = match dot { Some(i) => i, None => input.len() as int };
    let seconds = input.subrange(begin, end);
    if seconds.len() == 0 { None } else {
        match decimal_prefix(seconds, seconds.len() as int) {
            None => None,
            Some(value) => {
                let fraction = match dot { None => Some(0), Some(i) => fraction_model(input.subrange(i + 1, input.len() as int)) };
                match fraction { None => None, Some(nanos) => normalize_model(value, nanos, negative) }
            },
        }
    }
}
//= docs/parsing-requirements.md#parse-tar-037
//# Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.
pub fn time(input: &[u8]) -> (result: Option<(i64, u32)>)
    ensures match result {
        Some((s, n)) => time_model(input@) == Some((s as int, n as int)) && n < 1000000000,
        None => time_model(input@) is None,
    },
{
    let begin = if input.len() > 0 && (input[0] == b'-' || input[0] == b'+') { 1usize } else { 0usize };
    let negative = input.len() > 0 && input[0] == b'-';
    let dot = find_byte(input, begin, input.len(), b'.');
    let end = match dot { Some(i) => i, None => input.len() };
    let seconds = match decimal(&input[begin..end]) { None => return None, Some(v) => v };
    let nanos = match dot {
        None => 0u32,
        Some(i) => match fraction(&input[i + 1..]) { None => return None, Some(v) => v },
    };
    normalize(seconds, nanos, negative)
}
}
