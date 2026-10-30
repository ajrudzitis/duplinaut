//! Checked tar arithmetic kernels. These contracts do not yet establish
//! refinement of the whole tar/PAX controller or the generated header parser.
// Explicit scalar operations preserve the Verus contracts and solver-visible bounds.
#![allow(
    clippy::len_zero,
    clippy::manual_range_contains,
    clippy::assign_op_pattern
)]
use vstd::prelude::*;

verus! {
pub open spec fn checksum_byte(input: Seq<u8>, i: int, signed: bool) -> int
    recommends 0 <= i < input.len(),
{
    if 148 <= i < 156 { 32 }
    else if signed && input[i] >= 128 { input[i] as int - 256 }
    else { input[i] as int }
}
pub open spec fn checksum_prefix(input: Seq<u8>, n: int, signed: bool) -> int
    recommends 0 <= n <= input.len(),
    decreases n,
{
    if n <= 0 { 0 }
    else { checksum_prefix(input, n - 1, signed) + checksum_byte(input, n - 1, signed) }
}
pub proof fn checksum_bounds(input: Seq<u8>, n: int, signed: bool)
    requires 0 <= n <= input.len(),
    ensures -128 * n <= checksum_prefix(input, n, signed) <= 255 * n,
        !signed ==> 0 <= checksum_prefix(input, n, signed),
    decreases n,
{
    if n > 0 { checksum_bounds(input, n - 1, signed); }
}
pub fn checksums(input: &[u8]) -> (sums: (u64, i64))
    requires input.len() == 512,
    ensures sums.0 as int == checksum_prefix(input@, 512, false),
        sums.1 as int == checksum_prefix(input@, 512, true),
{
    let mut unsigned_sum = 0u64;
    let mut signed_sum = 0i64;
    let mut i = 0usize;
    while i < 512
        invariant input.len() == 512, i <= 512,
            unsigned_sum as int == checksum_prefix(input@, i as int, false),
            signed_sum as int == checksum_prefix(input@, i as int, true),
            0 <= unsigned_sum as int <= 255 * i as int,
            -128 * i as int <= signed_sum as int <= 255 * i as int,
        decreases 512 - i,
    {
        let byte = if 148 <= i && i < 156 { 32u8 } else { input[i] };
        unsigned_sum = unsigned_sum + byte as u64;
        let signed = if byte >= 128 { byte as i64 - 256 } else { byte as i64 };
        signed_sum = signed_sum + signed;
        i = i + 1;
    }
    (unsigned_sum, signed_sum)
}

/// Decide whether a decoded tar checksum field matches either supported
/// unsigned or signed-byte checksum profile for this exact 512-byte header.
pub fn checksum_matches(input: &[u8], expected: u64) -> (result: bool)
    requires input.len() == 512,
    ensures result == (expected as int == checksum_prefix(input@, 512, false)
        || expected as int == checksum_prefix(input@, 512, true)),
{
    let (unsigned_sum, signed_sum) = checksums(input);
    let unsigned_match = expected == unsigned_sum;
    let signed_match = expected as i128 == signed_sum as i128;
    proof {
        assert(unsigned_sum as int == checksum_prefix(input@, 512, false));
        assert(signed_sum as int == checksum_prefix(input@, 512, true));
        assert(unsigned_match == (expected as int == checksum_prefix(input@, 512, false)));
        assert((expected as i128) as int == expected as int);
        assert((signed_sum as i128) as int == signed_sum as int);
        assert(signed_match == (expected as int == checksum_prefix(input@, 512, true)));
    }
    unsigned_match || signed_match
}

pub open spec fn decimal_prefix(input: Seq<u8>, n: int) -> Option<int>
    recommends 0 <= n <= input.len(),
    decreases n,
{
    if n <= 0 { Some(0) }
    else if input[n - 1] < b'0' || input[n - 1] > b'9' { None }
    else { match decimal_prefix(input, n - 1) {
        None => None,
        Some(v) => if v * 10 + input[n - 1] as int - 48 > u64::MAX as int { None }
            else { Some(v * 10 + input[n - 1] as int - 48) },
    }}
}
//= docs/parsing-requirements.md#parse-tar-036
//# After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.
pub fn decimal(input: &[u8]) -> (result: Option<u64>)
    ensures match result {
        Some(value) => input.len() > 0 && decimal_prefix(input@, input.len() as int) == Some(value as int),
        None => input.len() == 0 || decimal_prefix(input@, input.len() as int) is None,
    },
{
    if input.len() == 0 { return None; }
    let mut value = 0u64;
    let mut i = 0usize;
    while i < input.len()
        invariant i <= input.len(), decimal_prefix(input@, i as int) == Some(value as int),
        decreases input.len() - i,
    {
        let byte = input[i];
        if byte < b'0' || byte > b'9' {
            proof { decimal_invalid_suffix(input@, i as int + 1, input.len() as int); }
            return None;
        }
        let digit = (byte - b'0') as u64;
        if value > (u64::MAX - digit) / 10 {
            proof { decimal_invalid_suffix(input@, i as int + 1, input.len() as int); }
            return None;
        }
        value = value * 10 + digit;
        i = i + 1;
    }
    Some(value)
}
pub proof fn decimal_invalid_suffix(input: Seq<u8>, start: int, end: int)
    requires 0 <= start <= end <= input.len(), decimal_prefix(input, start) is None,
    ensures decimal_prefix(input, end) is None,
    decreases end - start,
{
    if end > start { decimal_invalid_suffix(input, start, end - 1); }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtentError { Overflow, Truncated }

pub open spec fn padded_size(size: int) -> int {
    size + (512 - size % 512) % 512
}
pub open spec fn extent_model(cursor: int, size: int, total: int)
    -> Result<(int, int, int), ExtentError>
{
    let start = cursor + 512;
    let end = start + size;
    let next = start + padded_size(size);
    if start > usize::MAX as int || end > usize::MAX as int || next > usize::MAX as int {
        Err(ExtentError::Overflow)
    } else if next > total { Err(ExtentError::Truncated) }
    else { Ok((start, end, next)) }
}
pub fn extent(cursor: usize, size: usize, total: usize)
    -> (result: Result<(usize, usize, usize), ExtentError>)
    ensures match result {
        Ok((start, end, next)) => extent_model(cursor as int, size as int, total as int)
                == Ok((start as int, end as int, next as int))
            && cursor < start <= end <= next <= total
            && end - start == size
            && next - end < 512
            && (cursor as int % 512 == 0 ==> next as int % 512 == 0),
        Err(error) => extent_model(cursor as int, size as int, total as int) == Err(error),
    },
{
    if cursor > usize::MAX - 512 { return Err(ExtentError::Overflow); }
    let start = cursor + 512;
    if size > usize::MAX - start { return Err(ExtentError::Overflow); }
    let end = start + size;
    let padding = (512 - size % 512) % 512;
    if padding > usize::MAX - end { return Err(ExtentError::Overflow); }
    let next = end + padding;
    if next > total { return Err(ExtentError::Truncated); }
    assert((size as int + (512 - size as int % 512) % 512) % 512 == 0) by(nonlinear_arith);
    assert(cursor as int % 512 == 0 ==> next as int % 512 == 0) by(nonlinear_arith)
        requires next as int == cursor as int + 512 + size as int + padding as int,
            (size as int + padding as int) % 512 == 0;
    Ok((start, end, next))
}
}
