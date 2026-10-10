//! Checked tar arithmetic kernels. These contracts do not yet establish
//! refinement of the whole tar/PAX controller or the generated header parser.
// Explicit scalar operations preserve the Verus contracts and solver-visible bounds.
#![allow(clippy::len_zero, clippy::manual_range_contains, clippy::assign_op_pattern)]
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
