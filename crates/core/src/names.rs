//! Bounded, timezone-independent duplicity basename parsing.
//!
//! This native checkpoint has no formal assurance claim yet. Configuration-dependent
//! literal prefixes and Gregorian calendar arithmetic are explicit scalar kernels;
//! verification must refine them against the reviewed grammar before the reader gate
//! can become active. No regular expressions, locale, host timezone, or allocation.

use crate::generated::names::CanonicalTimestampFmt;
use vest_lib::core::exec::parser::Parser;

use vstd::prelude::*;

verus! {
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectKind {
    FullVolume,
    IncrementalVolume,
    FullManifest,
    IncrementalManifest,
    FullSignatures,
    IncrementalSignatures,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Envelope {
    Plain,
    Gzip,
    OpenPgp,
    /// Recognized for diagnostics; this reader does not support PKCS#7.
    Pkcs7,
}

/// Exact Unix seconds. The widened representation includes negative long dates
/// and every unsigned 64-bit short timestamp without truncation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Timestamp(pub i128);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectName {
    pub kind: ObjectKind,
    pub start: Timestamp,
    pub end: Timestamp,
    pub volume: Option<u64>,
    pub envelope: Envelope,
    pub partial: bool,
    pub short: bool,
}

pub open spec fn base36_digit_model(byte: u8) -> Option<int> {
    if b'0' <= byte && byte <= b'9' {
        Some((byte - b'0') as int)
    } else if b'a' <= byte && byte <= b'z' {
        Some((byte - b'a') as int + 10)
    } else {
        None
    }
}

/// Fold a prefix from left to right, returning None for an invalid digit or
/// the first value that cannot fit in an unsigned 64-bit integer.
pub open spec fn base36_value_model(bytes: Seq<u8>, prefix_len: int) -> Option<int>
    recommends 0 <= prefix_len <= bytes.len(),
    decreases prefix_len,
{
    if prefix_len <= 0 {
        Some(0)
    } else {
        match (base36_value_model(bytes, prefix_len - 1),
            base36_digit_model(bytes[prefix_len - 1])) {
            (Some(previous), Some(digit))
                if previous * 36 + digit <= u64::MAX as int =>
            {
                Some(previous * 36 + digit)
            }
            _ => None,
        }
    }
}

pub proof fn base36_model_failure_is_sticky(bytes: Seq<u8>, prefix_len: int)
    requires 0 <= prefix_len <= bytes.len(),
        base36_value_model(bytes, prefix_len).is_none(),
    ensures base36_value_model(bytes, bytes.len() as int).is_none(),
    decreases bytes.len() - prefix_len,
{
    if prefix_len < bytes.len() {
        reveal(base36_value_model);
        assert(base36_value_model(bytes, prefix_len + 1).is_none());
        base36_model_failure_is_sticky(bytes, prefix_len + 1);
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
pub fn decode_base36(input: &[u8]) -> (result: Option<u64>)
    ensures if input@.len() == 0 {
        result.is_none()
    } else {
        match base36_value_model(input@, input@.len() as int) {
            Some(value) => result matches Some(actual) && actual as int == value,
            None => result.is_none(),
        }
    },
{
    if input.is_empty() {
        return None;
    }
    let ghost bytes = input@;
    let mut result = 0u64;
    let mut index = 0usize;
    while index < input.len()
        invariant index <= input.len(),
            input@ == bytes,
            base36_value_model(bytes, index as int) == Some(result as int),
        decreases input.len() - index,
    {
        let byte = input[index];
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u64,
            b'a'..=b'z' => (byte - b'a') as u64 + 10,
            _ => {
                proof {
                    reveal(base36_value_model);
                    assert(base36_digit_model(byte).is_none());
                    assert(base36_value_model(bytes, index as int + 1).is_none());
                    base36_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        let product = result.checked_mul(36);
        let product = match product {
            Some(product) => product,
            None => {
                proof {
                    assert(result as int * 36 > u64::MAX as int);
                    reveal(base36_value_model);
                    assert(base36_digit_model(byte) == Some(digit as int));
                    assert(base36_value_model(bytes, index as int + 1).is_none());
                    base36_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        let next = product.checked_add(digit);
        let next = match next {
            Some(next) => next,
            None => {
                proof {
                    assert(product as int + digit as int > u64::MAX as int);
                    reveal(base36_value_model);
                    assert(base36_digit_model(byte) == Some(digit as int));
                    assert((result as int) * 36 == product as int);
                    assert(base36_value_model(bytes, index as int + 1).is_none());
                    base36_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        proof {
            reveal(base36_value_model);
            assert(base36_digit_model(byte) == Some(digit as int));
            assert((result as int) * 36 == product as int);
            assert(product as int + digit as int == next as int);
        }
        result = next;
        index += 1;
    }
    Some(result)
}

pub open spec fn decimal_digit_model(byte: u8) -> Option<int> {
    if b'0' <= byte && byte <= b'9' {
        Some((byte - b'0') as int)
    } else {
        None
    }
}

/// Fold a decimal digit prefix, rejecting invalid bytes and values above u64.
pub open spec fn decimal_value_model(bytes: Seq<u8>, prefix_len: int) -> Option<int>
    recommends 0 <= prefix_len <= bytes.len(),
    decreases prefix_len,
{
    if prefix_len <= 0 {
        Some(0)
    } else {
        match (decimal_value_model(bytes, prefix_len - 1),
            decimal_digit_model(bytes[prefix_len - 1])) {
            (Some(previous), Some(digit))
                if previous * 10 + digit <= u64::MAX as int =>
            {
                Some(previous * 10 + digit)
            }
            _ => None,
        }
    }
}

pub proof fn decimal_model_failure_is_sticky(bytes: Seq<u8>, prefix_len: int)
    requires 0 <= prefix_len <= bytes.len(),
        decimal_value_model(bytes, prefix_len).is_none(),
    ensures decimal_value_model(bytes, bytes.len() as int).is_none(),
    decreases bytes.len() - prefix_len,
{
    if prefix_len < bytes.len() {
        reveal(decimal_value_model);
        assert(decimal_value_model(bytes, prefix_len + 1).is_none());
        decimal_model_failure_is_sticky(bytes, prefix_len + 1);
    }
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
#[allow(
    clippy::manual_is_ascii_check,
    reason = "Keep the explicit digit range in Verus-supported syntax; the ASCII helper is unsupported by Verus."
)]
pub fn decode_decimal(input: &[u8]) -> (result: Option<u64>)
    ensures if input@.len() == 0 {
        result.is_none()
    } else {
        match decimal_value_model(input@, input@.len() as int) {
            Some(value) => result matches Some(actual) && actual as int == value,
            None => result.is_none(),
        }
    },
{
    if input.is_empty() {
        return None;
    }
    let ghost bytes = input@;
    let mut result = 0u64;
    let mut index = 0usize;
    while index < input.len()
        invariant index <= input.len(),
            input@ == bytes,
            decimal_value_model(bytes, index as int) == Some(result as int),
        decreases input.len() - index,
    {
        let byte = input[index];
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u64,
            _ => {
                proof {
                    reveal(decimal_value_model);
                    assert(decimal_digit_model(byte).is_none());
                    assert(decimal_value_model(bytes, index as int + 1).is_none());
                    decimal_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        let product = result.checked_mul(10);
        let product = match product {
            Some(product) => product,
            None => {
                proof {
                    assert(result as int * 10 > u64::MAX as int);
                    reveal(decimal_value_model);
                    assert(decimal_digit_model(byte) == Some(digit as int));
                    assert(decimal_value_model(bytes, index as int + 1).is_none());
                    decimal_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        let next = product.checked_add(digit);
        let next = match next {
            Some(next) => next,
            None => {
                proof {
                    assert(product as int + digit as int > u64::MAX as int);
                    reveal(decimal_value_model);
                    assert(decimal_digit_model(byte) == Some(digit as int));
                    assert((result as int) * 10 == product as int);
                    assert(decimal_value_model(bytes, index as int + 1).is_none());
                    decimal_model_failure_is_sticky(bytes, index as int + 1);
                }
                return None;
            }
        };
        proof {
            reveal(decimal_value_model);
            assert(decimal_digit_model(byte) == Some(digit as int));
            assert((result as int) * 10 == product as int);
            assert(product as int + digit as int == next as int);
        }
        result = next;
        index += 1;
    }
    Some(result)
}

/// The explicit reader policy for a configured separator in legacy timestamps.
pub open spec fn time_separator_valid_model(byte: u8) -> bool {
    0x21 <= byte && byte <= 0x7e
        && !(b'0' <= byte && byte <= b'9')
        && byte != b'/'
        && byte != b'.'
        && byte != b'+'
        && byte != b'-'
        && byte != b'T'
        && byte != b't'
        && byte != b'Z'
        && byte != b'z'
}

//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
#[allow(
    clippy::manual_is_ascii_check,
    reason = "Keep the explicit digit range in Verus-supported syntax; the ASCII helper is unsupported by Verus."
)]
pub(crate) fn time_separator_valid(byte: u8) -> (result: bool)
    ensures result == time_separator_valid_model(byte),
{
    matches!(byte, 0x21..=0x7e)
        && !matches!(byte, b'0'..=b'9')
        && byte != b'/'
        && byte != b'.'
        && byte != b'+'
        && byte != b'-'
        && byte != b'T'
        && byte != b't'
        && byte != b'Z'
        && byte != b'z'
}

/// Exact punctuation and length framing for the supported long timestamp
/// forms. Numeric fields and the zone token are validated by their own kernels.
pub open spec fn long_timestamp_framing_model(input: Seq<u8>, separator: u8) -> bool {
    (input.len() == 16
        && (input[8] == b'T' || input[8] == b't')
        && (input[15] == b'Z' || input[15] == b'z'))
        || ((input.len() == 20 || input.len() == 25)
            && input[4] == b'-'
            && input[7] == b'-'
            && (input[10] == b'T' || input[10] == b't')
            && input[13] == separator
            && input[16] == separator
            && if input.len() == 20 {
                input[19] == b'Z' || input[19] == b'z'
            } else {
                (input[19] == b'+' || input[19] == b'-') && input[22] == separator
            })
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn long_timestamp_framing_valid(input: &[u8], separator: u8) -> (result: bool)
    ensures result == long_timestamp_framing_model(input@, separator),
{
    match input.len() {
        16 => (input[8] == b'T' || input[8] == b't')
            && (input[15] == b'Z' || input[15] == b'z'),
        20 => input[4] == b'-'
            && input[7] == b'-'
            && (input[10] == b'T' || input[10] == b't')
            && input[13] == separator
            && input[16] == separator
            && (input[19] == b'Z' || input[19] == b'z'),
        25 => input[4] == b'-'
            && input[7] == b'-'
            && (input[10] == b'T' || input[10] == b't')
            && input[13] == separator
            && input[16] == separator
            && (input[19] == b'+' || input[19] == b'-')
            && input[22] == separator,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongTimestampField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// Byte bounds for a numeric field in an already framed long timestamp.
pub open spec fn long_timestamp_field_bounds_model(
    input: Seq<u8>,
    separator: u8,
    field: LongTimestampField,
) -> Option<(int, int)> {
    if !long_timestamp_framing_model(input, separator) {
        None
    } else if input.len() == 16 {
        match field {
            LongTimestampField::Year => Some((0, 4)),
            LongTimestampField::Month => Some((4, 6)),
            LongTimestampField::Day => Some((6, 8)),
            LongTimestampField::Hour => Some((9, 11)),
            LongTimestampField::Minute => Some((11, 13)),
            LongTimestampField::Second => Some((13, 15)),
        }
    } else {
        match field {
            LongTimestampField::Year => Some((0, 4)),
            LongTimestampField::Month => Some((5, 7)),
            LongTimestampField::Day => Some((8, 10)),
            LongTimestampField::Hour => Some((11, 13)),
            LongTimestampField::Minute => Some((14, 16)),
            LongTimestampField::Second => Some((17, 19)),
        }
    }
}

pub open spec fn long_timestamp_numeric_field_model(
    input: Seq<u8>,
    separator: u8,
    field: LongTimestampField,
) -> Option<int> {
    match long_timestamp_field_bounds_model(input, separator, field) {
        Some((start, end)) => decimal_value_model(input.subrange(start, end), end - start),
        None => None,
    }
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn long_timestamp_field_bounds(
    input: &[u8],
    separator: u8,
    field: LongTimestampField,
) -> (result: Option<(usize, usize)>)
    ensures match long_timestamp_field_bounds_model(input@, separator, field) {
        Some((start, end)) => result matches Some(bounds)
            && bounds.0 as int == start && bounds.1 as int == end,
        None => result.is_none(),
    },
{
    if !long_timestamp_framing_valid(input, separator) {
        proof {
            assert(!long_timestamp_framing_model(input@, separator));
            reveal(long_timestamp_field_bounds_model);
            assert(long_timestamp_field_bounds_model(input@, separator, field).is_none());
        }
        return None;
    }
    let result = if input.len() == 16 {
        match field {
            LongTimestampField::Year => (0, 4),
            LongTimestampField::Month => (4, 6),
            LongTimestampField::Day => (6, 8),
            LongTimestampField::Hour => (9, 11),
            LongTimestampField::Minute => (11, 13),
            LongTimestampField::Second => (13, 15),
        }
    } else {
        match field {
            LongTimestampField::Year => (0, 4),
            LongTimestampField::Month => (5, 7),
            LongTimestampField::Day => (8, 10),
            LongTimestampField::Hour => (11, 13),
            LongTimestampField::Minute => (14, 16),
            LongTimestampField::Second => (17, 19),
        }
    };
    proof {
        reveal(long_timestamp_field_bounds_model);
        assert(long_timestamp_field_bounds_model(input@, separator, field)
            == Some((result.0 as int, result.1 as int)));
    }
    Some(result)
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn decode_long_timestamp_numeric_field(
    input: &[u8],
    separator: u8,
    field: LongTimestampField,
) -> (result: Option<u64>)
    requires long_timestamp_framing_model(input@, separator),
    ensures match long_timestamp_numeric_field_model(input@, separator, field) {
        Some(value) => result matches Some(actual) && actual as int == value,
        None => result.is_none(),
    },
{
    let (start, end) = match long_timestamp_field_bounds(input, separator, field) {
        Some(bounds) => bounds,
        None => {
            proof {
                assert(long_timestamp_field_bounds_model(input@, separator, field).is_none());
                reveal(long_timestamp_numeric_field_model);
            }
            return None;
        }
    };
    let bytes = &input[start..end];
    proof {
        assert(bytes@ == input@.subrange(start as int, end as int));
        reveal(long_timestamp_numeric_field_model);
        reveal(long_timestamp_field_bounds_model);
    }
    decode_decimal(bytes)
}

/// Gregorian days between 1901-01-01 and January 1 of `year`.
///
/// This is the bounded calendar arithmetic kernel used by the native long-name
/// timestamp parser. The parser still owns and validates the date fields.
pub open spec fn days_before_year_model(year: int) -> int
    recommends year >= 1901,
    decreases year - 1901,
{
    if year <= 1901 {
        0
    } else {
        days_before_year_model(year - 1)
            + if year_is_leap(year - 1) { 366int } else { 365int }
    }
}

pub open spec fn year_is_leap(year: int) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub proof fn days_before_year_step(year: int)
    requires 1901 <= year < 2100,
    ensures days_before_year_model(year + 1)
        == days_before_year_model(year) + if year_is_leap(year) { 366int } else { 365int },
{
    reveal(days_before_year_model);
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
pub fn days_before_year(year: u16) -> (result: i64)
    requires 1901 <= year <= 2100,
    ensures result as int == days_before_year_model(year as int),
        0 <= result as int <= (year as int - 1901) * 366,
{
    let mut current = 1901u16;
    let mut days = 0i64;
    while current < year
        invariant 1901 <= current <= year <= 2100,
            days as int == days_before_year_model(current as int),
            0 <= days <= (current as i64 - 1901) * 366,
        decreases year - current,
    {
        proof { days_before_year_step(current as int); }
        days += if current.is_multiple_of(4)
            && (!current.is_multiple_of(100) || current.is_multiple_of(400))
        {
            366
        } else {
            365
        };
        current += 1;
    }
    days
}

pub open spec fn month_length_model(year: int, month: int) -> int {
    if month == 2 {
        if year_is_leap(year) { 29int } else { 28int }
    } else if month == 4 || month == 6 || month == 9 || month == 11 {
        30int
    } else {
        31int
    }
}

/// Number of Gregorian days before `month` in `year`.
pub open spec fn days_before_month_model(year: int, month: int) -> int
    recommends month >= 1,
    decreases month - 1,
{
    if month <= 1 {
        0
    } else {
        days_before_month_model(year, month - 1) + month_length_model(year, month - 1)
    }
}

pub proof fn days_before_month_step(year: int, month: int)
    requires 1 <= month <= 12,
    ensures days_before_month_model(year, month + 1)
        == days_before_month_model(year, month) + month_length_model(year, month),
{
    reveal(days_before_month_model);
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
pub fn month_length(year: u16, month: u8) -> (result: u8)
    requires 1901 <= year <= 2099, 1 <= month <= 12,
    ensures result as int == month_length_model(year as int, month as int),
{
    let leap_year = year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

pub open spec fn calendar_day_valid_model(year: int, month: int, day: int) -> bool {
    1 <= day && day <= month_length_model(year, month)
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
pub fn calendar_day_valid(year: u16, month: u8, day: u8) -> (result: bool)
    requires 1901 <= year <= 2099, 1 <= month <= 12,
    ensures result == calendar_day_valid_model(year as int, month as int, day as int),
{
    day >= 1 && day <= month_length(year, month)
}

pub open spec fn calendar_date_valid_model(year: int, month: int, day: int) -> bool {
    1901 <= year && year <= 2099
        && 1 <= month && month <= 12
        && calendar_day_valid_model(year, month, day)
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
pub fn calendar_date_valid(year: i64, month: i64, day: i64) -> (result: bool)
    ensures result == calendar_date_valid_model(year as int, month as int, day as int),
{
    if !matches!(year, 1901..=2099)
        || !matches!(month, 1..=12)
        || !matches!(day, 1..=31)
    {
        return false;
    }
    calendar_day_valid(year as u16, month as u8, day as u8)
}

pub open spec fn clock_fields_valid_model(hour: int, minute: int, second: int) -> bool {
    0 <= hour && hour <= 23 && 0 <= minute && minute <= 59 && 0 <= second && second <= 59
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
pub fn clock_fields_valid(hour: i64, minute: i64, second: i64) -> (result: bool)
    ensures result == clock_fields_valid_model(hour as int, minute as int, second as int),
{
    matches!(hour, 0..=23) && matches!(minute, 0..=59) && matches!(second, 0..=59)
}

pub open spec fn zone_offset_seconds_model(sign: int, hours: int, minutes: int) -> int {
    sign * (hours * 3600 + minutes * 60)
}

//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn zone_offset_seconds(sign: i64, hours: i64, minutes: i64) -> (result: i64)
    requires (sign == 1 || sign == -1), 0 <= hours <= 23, 0 <= minutes <= 59,
    ensures result as int == zone_offset_seconds_model(sign as int, hours as int, minutes as int),
{
    sign * (hours * 3600 + minutes * 60)
}

pub open spec fn zone_digits_model(zone: Seq<u8>, start: int) -> Option<int>
    recommends 0 <= start && start + 1 < zone.len(),
{
    match (decimal_digit_model(zone[start]), decimal_digit_model(zone[start + 1])) {
        (Some(tens), Some(ones)) => Some(tens * 10 + ones),
        _ => None,
    }
}

pub open spec fn zone_offset_token_model(zone: Seq<u8>, separator: u8) -> Option<int> {
    if zone.len() == 1 && (zone[0] == b'Z' || zone[0] == b'z') {
        Some(0)
    } else if zone.len() == 6
        && (zone[0] == b'+' || zone[0] == b'-')
        && zone[3] == separator
    {
        match (zone_digits_model(zone, 1), zone_digits_model(zone, 4)) {
            (Some(hours), Some(minutes)) if hours <= 23 && minutes <= 59 =>
                Some((if zone[0] == b'+' { 1 } else { -1 }) * (hours * 3600 + minutes * 60)),
            _ => None,
        }
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn decode_zone_offset_token(zone: &[u8], separator: u8) -> (result: Option<i64>)
    ensures match zone_offset_token_model(zone@, separator) {
        Some(offset) => result matches Some(actual) && actual as int == offset,
        None => result.is_none(),
    },
{
    if zone.len() == 1 {
        if zone[0] == b'Z' || zone[0] == b'z' {
            return Some(0);
        }
        return None;
    }
    if zone.len() != 6 || zone[3] != separator || !matches!(zone[0], b'+' | b'-') {
        return None;
    }
    let hour_tens = match zone[1] {
        b'0'..=b'9' => (zone[1] - b'0') as i64,
        _ => return None,
    };
    let hour_ones = match zone[2] {
        b'0'..=b'9' => (zone[2] - b'0') as i64,
        _ => return None,
    };
    let hours = hour_tens * 10 + hour_ones;
    proof {
        reveal(zone_offset_token_model);
        reveal(zone_digits_model);
        assert(decimal_digit_model(zone@[1]) == Some(hour_tens as int));
        assert(decimal_digit_model(zone@[2]) == Some(hour_ones as int));
        assert(zone_digits_model(zone@, 1) == Some(hours as int));
    }
    if hours > 23 {
        proof {
            reveal(zone_offset_token_model);
            assert(zone_offset_token_model(zone@, separator).is_none());
        }
        return None;
    }
    let minute_tens = match zone[4] {
        b'0'..=b'9' => (zone[4] - b'0') as i64,
        _ => return None,
    };
    let minute_ones = match zone[5] {
        b'0'..=b'9' => (zone[5] - b'0') as i64,
        _ => return None,
    };
    let minutes = minute_tens * 10 + minute_ones;
    proof {
        reveal(zone_offset_token_model);
        reveal(zone_digits_model);
        assert(decimal_digit_model(zone@[4]) == Some(minute_tens as int));
        assert(decimal_digit_model(zone@[5]) == Some(minute_ones as int));
        assert(zone_digits_model(zone@, 4) == Some(minutes as int));
    }
    if minutes > 59 {
        proof {
            reveal(zone_offset_token_model);
            assert(zone_offset_token_model(zone@, separator).is_none());
        }
        return None;
    }
    let sign = if zone[0] == b'+' { 1 } else { -1 };
    let result = zone_offset_seconds(sign, hours, minutes);
    proof {
        reveal(zone_offset_token_model);
        reveal(zone_digits_model);
        assert(zone_offset_token_model(zone@, separator)
            == Some(zone_offset_seconds_model(sign as int, hours as int, minutes as int)));
    }
    Some(result)
}

pub open spec fn calendar_fields_to_unix_seconds_model(
    year: int,
    month: int,
    day: int,
    hour: int,
    minute: int,
    second: int,
    offset_seconds: int,
) -> int
    recommends calendar_date_valid_model(year, month, day)
        && clock_fields_valid_model(hour, minute, second)
        && -86340 <= offset_seconds <= 86340,
{
    let days_since_epoch = days_before_year_model(year)
        + days_before_month_model(year, month)
        + day - 1 - 25202;
    days_since_epoch * 86400 + hour * 3600 + minute * 60 + second - offset_seconds
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn calendar_fields_to_unix_seconds(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    offset_seconds: i64,
) -> (result: Timestamp)
    requires 1901 <= year <= 2099,
        1 <= month <= 12,
        calendar_date_valid_model(year as int, month as int, day as int),
        clock_fields_valid_model(hour as int, minute as int, second as int),
        -86340 <= offset_seconds <= 86340,
    ensures result.0 as int == calendar_fields_to_unix_seconds_model(
        year as int,
        month as int,
        day as int,
        hour as int,
        minute as int,
        second as int,
        offset_seconds as int,
    ),
{
    proof {
        reveal(calendar_date_valid_model);
        assert(1901 <= year as int <= 2099);
        assert(1 <= month as int <= 12);
    }
    let year_days = days_before_year(year);
    let mut month_days = 0i128;
    let mut current_month = 1u8;
    while current_month < month
        invariant 1901 <= year <= 2099,
            1 <= current_month <= month <= 12,
            month_days as int == days_before_month_model(year as int, current_month as int),
            0 <= month_days as int <= (current_month as int - 1) * 31,
        decreases month as int - current_month as int,
    {
        proof {
            days_before_month_step(year as int, current_month as int);
        }
        let days_in_month = month_length(year, current_month);
        proof {
            reveal(month_length_model);
            assert(days_in_month as int <= 31);
        }
        month_days += days_in_month as i128;
        current_month += 1;
    }
    let days_since_epoch = year_days as i128 + month_days + day as i128 - 1 - 25202;
    proof {
        assert(month_days as int == days_before_month_model(year as int, month as int));
        assert(-100_000 <= days_since_epoch <= 100_000);
    }
    let seconds_since_epoch = days_since_epoch * 86400 + hour as i128 * 3600
        + minute as i128 * 60 + second as i128 - offset_seconds as i128;
    proof {
        assert(seconds_since_epoch as int == calendar_fields_to_unix_seconds_model(
            year as int,
            month as int,
            day as int,
            hour as int,
            minute as int,
            second as int,
            offset_seconds as int,
        ));
    }
    Timestamp(seconds_since_epoch)
}

impl ObjectName {
    /// A partial artifact cannot supply completed-set evidence.
    //= docs/parsing-requirements.md#parse-name-005
    //= type=implementation
    //# Objects marked partial MUST be excluded from completed restore sets.
    pub fn completed(self) -> (result: bool)
        ensures result == (!self.partial && self.envelope != Envelope::Pkcs7),
    {
        !self.partial && !matches!(self.envelope, Envelope::Pkcs7)
    }
}

pub open spec fn valid_set_interval_model(
    incremental: bool,
    start: Timestamp,
    end: Timestamp,
) -> bool {
    !incremental || end.0 > start.0
}

//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
pub fn valid_set_interval(
    incremental: bool,
    start: Timestamp,
    end: Timestamp,
) -> (result: bool)
    ensures result == valid_set_interval_model(incremental, start, end),
{
    !incremental || end.0 > start.0
}

}

#[derive(Clone, Copy, Debug)]
pub struct NameConfig<'a> {
    pub global_prefix: &'a [u8],
    pub archive_prefix: &'a [u8],
    pub manifest_prefix: &'a [u8],
    pub signature_prefix: &'a [u8],
    pub time_separator: u8,
    pub max_name_bytes: usize,
}

impl Default for NameConfig<'_> {
    fn default() -> Self {
        Self {
            global_prefix: b"",
            archive_prefix: b"",
            manifest_prefix: b"",
            signature_prefix: b"",
            time_separator: b':',
            max_name_bytes: 4096,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameError {
    Configuration,
    Limit,
    Malformed,
    Ambiguous,
}

fn numeral(input: &[u8], radix: u64) -> Result<u64, NameError> {
    if input.is_empty() {
        return Err(NameError::Malformed);
    }
    if radix == 10 {
        return decode_decimal(input).ok_or(NameError::Malformed);
    }
    if radix == 36 {
        return decode_base36(input).ok_or(NameError::Malformed);
    }
    let mut result = 0u64;
    for &byte in input {
        let digit = match byte {
            b'0'..=b'9' => u64::from(byte - b'0'),
            b'a'..=b'z' if radix == 36 => u64::from(byte - b'a') + 10,
            _ => return Err(NameError::Malformed),
        };
        if digit >= radix {
            return Err(NameError::Malformed);
        }
        result = result
            .checked_mul(radix)
            .and_then(|value| value.checked_add(digit))
            .ok_or(NameError::Malformed)?;
    }
    Ok(result)
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.

//= docs/parsing-requirements.md#parse-name-007
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.

/// Parse an exact canonical or legacy explicitly zoned Gregorian timestamp.
pub fn parse_long_timestamp(input: &[u8], separator: u8) -> Result<Timestamp, NameError> {
    if !time_separator_valid(separator) {
        return Err(NameError::Configuration);
    }
    if !long_timestamp_framing_valid(input, separator) {
        return Err(NameError::Malformed);
    }
    if input.len() == 16 {
        let (consumed, _) = CanonicalTimestampFmt
            .parse(&input)
            .map_err(|_| NameError::Malformed)?;
        if consumed != input.len() {
            return Err(NameError::Malformed);
        }
    }
    let zone = match input.len() {
        16 => &input[15..16],
        20 | 25 => &input[19..],
        _ => return Err(NameError::Malformed),
    };
    let y = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Year)
        .ok_or(NameError::Malformed)? as i64;
    let m = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Month)
        .ok_or(NameError::Malformed)? as i64;
    let d = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Day)
        .ok_or(NameError::Malformed)? as i64;
    let h = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Hour)
        .ok_or(NameError::Malformed)? as i64;
    let min = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Minute)
        .ok_or(NameError::Malformed)? as i64;
    let sec = decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Second)
        .ok_or(NameError::Malformed)? as i64;
    if !calendar_date_valid(y, m, d) || !clock_fields_valid(h, min, sec) {
        return Err(NameError::Malformed);
    }
    let offset = decode_zone_offset_token(zone, separator).ok_or(NameError::Malformed)?;
    Ok(calendar_fields_to_unix_seconds(
        y as u16, m as u8, d as u8, h as u8, min as u8, sec as u8, offset,
    ))
}

fn date(input: &[u8], short: bool, separator: u8) -> Result<Timestamp, NameError> {
    if short {
        Ok(Timestamp(i128::from(numeral(input, 36)?)))
    } else {
        parse_long_timestamp(input, separator)
    }
}

const KINDS: [ObjectKind; 6] = [
    ObjectKind::FullVolume,
    ObjectKind::IncrementalVolume,
    ObjectKind::FullManifest,
    ObjectKind::IncrementalManifest,
    ObjectKind::FullSignatures,
    ObjectKind::IncrementalSignatures,
];

fn stem(kind: ObjectKind, short: bool) -> &'static [u8] {
    match (kind, short) {
        (ObjectKind::FullVolume | ObjectKind::FullManifest, false) => b"duplicity-full.",
        (ObjectKind::IncrementalVolume | ObjectKind::IncrementalManifest, false) => {
            b"duplicity-inc."
        }
        (ObjectKind::FullSignatures, false) => b"duplicity-full-signatures.",
        (ObjectKind::IncrementalSignatures, false) => b"duplicity-new-signatures.",
        (ObjectKind::FullVolume | ObjectKind::FullManifest, true) => b"df.",
        (ObjectKind::IncrementalVolume | ObjectKind::IncrementalManifest, true) => b"di.",
        (ObjectKind::FullSignatures, true) => b"dfs.",
        (ObjectKind::IncrementalSignatures, true) => b"dns.",
    }
}

//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
fn body(
    input: &[u8],
    kind: ObjectKind,
    short: bool,
    separator: u8,
) -> Result<ObjectName, NameError> {
    let mut fields = input.split(|byte| *byte == b'.');
    let start = date(fields.next().ok_or(NameError::Malformed)?, short, separator)?;
    let incremental = matches!(
        kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures
    );
    let end = if incremental {
        if !short && fields.next() != Some(b"to") {
            return Err(NameError::Malformed);
        }
        date(fields.next().ok_or(NameError::Malformed)?, short, separator)?
    } else {
        start
    };
    if !valid_set_interval(incremental, start, end) {
        return Err(NameError::Malformed);
    }
    let volume = if matches!(kind, ObjectKind::FullVolume | ObjectKind::IncrementalVolume) {
        let field = fields.next().ok_or(NameError::Malformed)?;
        let number = numeral(
            if short {
                field
            } else {
                field.strip_prefix(b"vol").ok_or(NameError::Malformed)?
            },
            if short { 36 } else { 10 },
        )?;
        if number == 0 {
            return Err(NameError::Malformed);
        }
        Some(number)
    } else {
        None
    };
    let trailer: &[u8] = match (kind, short) {
        (ObjectKind::FullVolume | ObjectKind::IncrementalVolume, false) => b"difftar",
        (ObjectKind::FullManifest | ObjectKind::IncrementalManifest, false) => b"manifest",
        (_, false) => b"sigtar",
        (ObjectKind::FullVolume | ObjectKind::IncrementalVolume, true) => b"dt",
        (ObjectKind::FullManifest | ObjectKind::IncrementalManifest, true) => b"m",
        (_, true) => b"st",
    };
    if fields.next() != Some(trailer) {
        return Err(NameError::Malformed);
    }
    let mut suffix = fields.next();
    let partial = suffix == Some(if short { b"p" as &[u8] } else { b"part" });
    if partial {
        suffix = fields.next();
    }
    let envelope = match (suffix, short) {
        (None, _) => Envelope::Plain,
        (Some(b"z"), true) | (Some(b"gz"), false) => Envelope::Gzip,
        (Some(b"g"), true) | (Some(b"gpg"), false) => Envelope::OpenPgp,
        (Some(b"p7m"), false) => Envelope::Pkcs7,
        _ => return Err(NameError::Malformed),
    };
    if fields.next().is_some() {
        return Err(NameError::Malformed);
    }
    Ok(ObjectName {
        kind,
        start,
        end,
        volume,
        envelope,
        partial,
        short,
    })
}

//= docs/parsing-requirements.md#parse-name-001
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.

//= docs/parsing-requirements.md#parse-name-002
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.

//= docs/parsing-requirements.md#parse-name-003
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.

//= docs/parsing-requirements.md#parse-name-005
//# Objects marked partial MUST be excluded from completed restore sets.

//= docs/parsing-requirements.md#parse-name-006
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.

/// Parse a basename, not an S3 key. Unrelated names return `Ok(None)`.
/// Conflicting prefix configurations producing distinct successful classifications
/// return `Ambiguous`; malformed recognized stems are errors rather than ignored.
pub fn parse_name(input: &[u8], config: NameConfig<'_>) -> Result<Option<ObjectName>, NameError> {
    if !time_separator_valid(config.time_separator)
        || [
            config.global_prefix,
            config.archive_prefix,
            config.manifest_prefix,
            config.signature_prefix,
        ]
        .iter()
        .any(|prefix| prefix.contains(&0) || prefix.contains(&b'/'))
    {
        return Err(NameError::Configuration);
    }
    if input.len() > config.max_name_bytes {
        return Err(NameError::Limit);
    }
    if input.contains(&b'/') || input.contains(&0) {
        return Err(NameError::Malformed);
    }
    let Some(input) = input.strip_prefix(config.global_prefix) else {
        return Ok(None);
    };
    let mut recognized = false;
    let mut result = None;
    for kind in KINDS {
        let prefix = match kind {
            ObjectKind::FullVolume | ObjectKind::IncrementalVolume => config.archive_prefix,
            ObjectKind::FullManifest | ObjectKind::IncrementalManifest => config.manifest_prefix,
            _ => config.signature_prefix,
        };
        let Some(rest) = input.strip_prefix(prefix) else {
            continue;
        };
        for short in [false, true] {
            if let Some(rest) = rest.strip_prefix(stem(kind, short)) {
                recognized = true;
                if let Ok(parsed) = body(rest, kind, short, config.time_separator) {
                    if result.is_some_and(|previous| previous != parsed) {
                        return Err(NameError::Ambiguous);
                    }
                    result = Some(parsed);
                }
            }
        }
    }
    if result.is_none() && recognized {
        Err(NameError::Malformed)
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calendar_date_valid, calendar_day_valid, calendar_fields_to_unix_seconds,
        clock_fields_valid, decode_base36, decode_decimal, decode_long_timestamp_numeric_field,
        decode_zone_offset_token, long_timestamp_field_bounds, long_timestamp_framing_valid,
        parse_long_timestamp, time_separator_valid, LongTimestampField, NameError, Timestamp,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn zone_token_decoder_checks_every_byte_position_and_boundary() {
        let valid = *b"+00:00";
        for position in 0..valid.len() {
            for byte in 0u8..=u8::MAX {
                let mut token = valid;
                token[position] = byte;
                let valid_shape = matches!(token[0], b'+' | b'-')
                    && token[1].is_ascii_digit()
                    && token[2].is_ascii_digit()
                    && token[3] == b':'
                    && token[4].is_ascii_digit()
                    && token[5].is_ascii_digit();
                let valid_range = valid_shape
                    && (token[1] - b'0') * 10 + (token[2] - b'0') <= 23
                    && (token[4] - b'0') * 10 + (token[5] - b'0') <= 59;
                let decoded = decode_zone_offset_token(&token, b':');
                assert_eq!(
                    decoded.is_some(),
                    valid_range,
                    "position {position}, byte {byte}"
                );
            }
        }
        assert_eq!(decode_zone_offset_token(b"+23:59", b':'), Some(86_340));
        assert_eq!(decode_zone_offset_token(b"-23:59", b':'), Some(-86_340));
        assert_eq!(decode_zone_offset_token(b"Z", b':'), Some(0));
        assert_eq!(decode_zone_offset_token(b"z", b':'), Some(0));
        for token in [
            &b"+24:00"[..],
            &b"+23:60"[..],
            &b"+2:00"[..],
            &b"+023:00"[..],
            &b"+23;59"[..],
            &b"*23:59"[..],
            &b"+2a:00"[..],
        ] {
            assert_eq!(decode_zone_offset_token(token, b':'), None, "{token:?}");
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn calendar_fields_convert_to_exact_unix_seconds() {
        let cases = [
            (1901, 1, 1, 0, 0, 0, 0, -2_177_452_800),
            (1969, 12, 31, 23, 59, 59, 0, -1),
            (1970, 1, 1, 0, 0, 0, 0, 0),
            (1970, 1, 1, 23, 59, 0, 86_340, 0),
            (1969, 12, 31, 0, 1, 0, -86_340, 0),
            (1970, 1, 1, 0, 0, 0, 86_340, -86_340),
            (1970, 1, 1, 0, 0, 0, -86_340, 86_340),
            (2000, 2, 29, 0, 0, 0, 0, 951_782_400),
            (2099, 12, 31, 23, 59, 59, 0, 4_102_444_799),
        ];
        for (year, month, day, hour, minute, second, offset, expected) in cases {
            assert_eq!(
                calendar_fields_to_unix_seconds(year, month, day, hour, minute, second, offset),
                Timestamp(expected),
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02} offset {offset}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    fn clock_field_policy_matches_every_byte_and_boundary() {
        for byte in 0..=u8::MAX {
            let value = i64::from(byte);
            assert_eq!(clock_fields_valid(value, 12, 30), byte <= 23);
            assert_eq!(clock_fields_valid(12, value, 30), byte <= 59);
            assert_eq!(clock_fields_valid(12, 30, value), byte <= 59);
        }
        for (hour, minute, second, valid) in [
            (0, 0, 0, true),
            (23, 59, 59, true),
            (24, 0, 0, false),
            (0, 60, 0, false),
            (0, 0, 60, false),
            (-1, 0, 0, false),
            (0, -1, 0, false),
            (0, 0, -1, false),
        ] {
            assert_eq!(clock_fields_valid(hour, minute, second), valid);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    fn calendar_day_predicate_matches_gregorian_month_boundaries() {
        for year in 1901..=2099u16 {
            for month in 1..=12u8 {
                let days = match month {
                    2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
                    2 => 28,
                    4 | 6 | 9 | 11 => 30,
                    _ => 31,
                };
                for day in 0..=31u8 {
                    assert_eq!(
                        calendar_day_valid(year, month, day),
                        (1..=days).contains(&day),
                        "{year:04}-{month:02}-{day:02}"
                    );
                }
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    fn calendar_date_policy_rejects_out_of_profile_fields() {
        for year in [i64::MIN, 1900, 1901, 2099, 2100, i64::MAX] {
            for month in [i64::MIN, 0, 1, 12, 13, i64::MAX] {
                for day in [i64::MIN, 0, 1, 28, 29, 30, 31, 32, i64::MAX] {
                    let valid = matches!(year, 1901..=2099)
                        && matches!(month, 1..=12)
                        && if matches!(month, 1 | 3 | 5 | 7 | 8 | 10 | 12) {
                            matches!(day, 1..=31)
                        } else if month == 2 {
                            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                            if leap {
                                matches!(day, 1..=29)
                            } else {
                                matches!(day, 1..=28)
                            }
                        } else {
                            matches!(day, 1..=30)
                        };
                    assert_eq!(calendar_date_valid(year, month, day), valid);
                }
            }
        }
        for year in 1901..=2099i64 {
            for month in 1..=12i64 {
                let max_day = if month == 2 {
                    if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                        29
                    } else {
                        28
                    }
                } else if matches!(month, 4 | 6 | 9 | 11) {
                    30
                } else {
                    31
                };
                for day in 0..=max_day + 1 {
                    assert_eq!(
                        calendar_date_valid(year, month, day),
                        (1..=max_day).contains(&day)
                    );
                }
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-005
    //= type=test
    //# Objects marked partial MUST be excluded from completed restore sets.
    fn completed_artifact_policy_excludes_partial_and_unsupported_objects() {
        use super::{Envelope, ObjectKind, ObjectName, Timestamp};

        let kinds = [
            ObjectKind::FullVolume,
            ObjectKind::IncrementalVolume,
            ObjectKind::FullManifest,
            ObjectKind::IncrementalManifest,
            ObjectKind::FullSignatures,
            ObjectKind::IncrementalSignatures,
        ];
        let envelopes = [
            Envelope::Plain,
            Envelope::Gzip,
            Envelope::OpenPgp,
            Envelope::Pkcs7,
        ];
        for kind in kinds {
            for envelope in envelopes {
                for partial in [false, true] {
                    let object = ObjectName {
                        kind,
                        start: Timestamp(0),
                        end: Timestamp(1),
                        volume: None,
                        envelope,
                        partial,
                        short: false,
                    };
                    assert_eq!(
                        object.completed(),
                        !partial && envelope != Envelope::Pkcs7,
                        "{kind:?} {envelope:?} partial={partial}"
                    );
                }
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    fn decimal_decoder_matches_all_single_and_double_digit_values() {
        for byte in 0..=u8::MAX {
            let expected = match byte {
                b'0'..=b'9' => Some(u64::from(byte - b'0')),
                _ => None,
            };
            assert_eq!(decode_decimal(&[byte]), expected);
        }
        for high in 0..=9u8 {
            for low in 0..=9u8 {
                assert_eq!(
                    decode_decimal(&[b'0' + high, b'0' + low]),
                    Some(u64::from(high) * 10 + u64::from(low))
                );
            }
        }
        for value in [0, 1, 9, 10, 99, 100, u32::MAX as u64, u64::MAX] {
            let encoded = value.to_string();
            assert_eq!(decode_decimal(encoded.as_bytes()), Some(value));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    fn decimal_decoder_rejects_empty_invalid_and_overflowing_inputs() {
        assert_eq!(decode_decimal(b""), None);
        for invalid in [b"a" as &[u8], b"-", b"_", b"12a", &[0xff]] {
            assert_eq!(decode_decimal(invalid), None, "{invalid:?}");
        }
        let overflow = (u64::MAX as u128 + 1).to_string();
        assert_eq!(decode_decimal(overflow.as_bytes()), None);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn set_interval_policy_checks_incremental_order_at_i128_boundaries() {
        use super::{valid_set_interval, Timestamp};

        let min = Timestamp(i128::MIN);
        let min_next = Timestamp(i128::MIN + 1);
        let max_previous = Timestamp(i128::MAX - 1);
        let max = Timestamp(i128::MAX);

        for (start, end) in [(min, max), (max, min), (max, max)] {
            assert!(valid_set_interval(false, start, end));
        }
        assert!(valid_set_interval(true, min, min_next));
        assert!(valid_set_interval(true, max_previous, max));
        assert!(!valid_set_interval(true, min, min));
        assert!(!valid_set_interval(true, max, max));
        assert!(!valid_set_interval(true, max, max_previous));
        assert!(!valid_set_interval(true, max, min));
    }

    fn encode_base36(mut value: u128) -> Vec<u8> {
        if value == 0 {
            return vec![b'0'];
        }
        let mut result = Vec::new();
        while value != 0 {
            let digit = (value % 36) as u8;
            result.push(if digit < 10 {
                b'0' + digit
            } else {
                b'a' + digit - 10
            });
            value /= 36;
        }
        result.reverse();
        result
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn base36_decoder_matches_single_and_double_digit_values() {
        for first in 0..=u8::MAX {
            let first_value = match first {
                b'0'..=b'9' => Some(u64::from(first - b'0')),
                b'a'..=b'z' => Some(u64::from(first - b'a') + 10),
                _ => None,
            };
            assert_eq!(decode_base36(&[first]), first_value);
        }
        for high in 0..36u8 {
            for low in 0..36u8 {
                let digit = |value: u8| {
                    if value < 10 {
                        b'0' + value
                    } else {
                        b'a' + value - 10
                    }
                };
                assert_eq!(
                    decode_base36(&[digit(high), digit(low)]),
                    Some(u64::from(high) * 36 + u64::from(low))
                );
            }
        }
        for value in [0, 1, 35, 36, 1295, u32::MAX as u128, u64::MAX as u128] {
            let encoded = encode_base36(value);
            assert_eq!(decode_base36(&encoded), Some(value as u64));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn base36_decoder_rejects_empty_invalid_and_overflowing_inputs() {
        assert_eq!(decode_base36(b""), None);
        for invalid in [b"A" as &[u8], b"-", b"_", b"0A", b"a_"] {
            assert_eq!(decode_base36(invalid), None, "{invalid:?}");
        }
        assert_eq!(decode_base36(&encode_base36(u64::MAX as u128 + 1)), None);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn time_separator_predicate_matches_the_policy_for_every_byte() {
        for value in 0..=u8::MAX {
            let expected = (0x21..=0x7e).contains(&value)
                && !value.is_ascii_digit()
                && !matches!(value, b'/' | b'.' | b'+' | b'-' | b'T' | b't' | b'Z' | b'z');
            assert_eq!(time_separator_valid(value), expected, "byte {value:#04x}");
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn invalid_time_separator_is_a_configuration_error() {
        for separator in [
            0, b' ', b'/', b'.', b'+', b'-', b'0', b'T', b't', b'Z', b'z', 0x7f,
        ] {
            assert_eq!(
                parse_long_timestamp(b"2024-01-01T12:34:56Z", separator),
                Err(NameError::Configuration)
            );
        }
    }

    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    #[test]
    fn timestamp_framing_checks_all_supported_lengths_and_delimiters() {
        let samples: [(&[u8], u8, &[usize]); 3] = [
            (b"19700101T000000Z", b':', &[8, 15]),
            (b"1970-01-01T00:00:00Z", b':', &[4, 7, 10, 13, 16, 19]),
            (
                b"1970-01-01t00_00_00+00_00",
                b'_',
                &[4, 7, 10, 13, 16, 19, 22],
            ),
        ];
        for (sample, separator, positions) in samples {
            assert!(
                long_timestamp_framing_valid(sample, separator),
                "{sample:?}"
            );
            for position in 0..sample.len() {
                for byte in 0u8..=u8::MAX {
                    let mut mutated = sample.to_vec();
                    mutated[position] = byte;
                    let expected = if positions.contains(&position) {
                        match (sample.len(), position) {
                            (16, 8) | (20 | 25, 10) => matches!(byte, b'T' | b't'),
                            (16, 15) | (20, 19) => matches!(byte, b'Z' | b'z'),
                            (25, 19) => matches!(byte, b'+' | b'-'),
                            (25, 22) => byte == separator,
                            (20 | 25, 13 | 16) => byte == separator,
                            (20 | 25, 4 | 7) => byte == b'-',
                            _ => unreachable!("unclassified framing position"),
                        }
                    } else {
                        true
                    };
                    assert_eq!(
                        long_timestamp_framing_valid(&mutated, separator),
                        expected,
                        "sample {sample:?}, position {position}, byte {byte:#04x}"
                    );
                }
            }
        }
        for length in 0..=32 {
            if !matches!(length, 16 | 20 | 25) {
                assert!(!long_timestamp_framing_valid(&vec![b'0'; length], b':'));
            }
        }
    }

    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    #[test]
    fn timestamp_numeric_fields_use_exact_format_slices() {
        type FieldCase = (LongTimestampField, usize, usize, u64);
        let samples: [(&[u8], u8, [FieldCase; 6]); 3] = [
            (
                b"19700101T123456Z",
                b':',
                [
                    (LongTimestampField::Year, 0, 4, 1970),
                    (LongTimestampField::Month, 4, 6, 1),
                    (LongTimestampField::Day, 6, 8, 1),
                    (LongTimestampField::Hour, 9, 11, 12),
                    (LongTimestampField::Minute, 11, 13, 34),
                    (LongTimestampField::Second, 13, 15, 56),
                ],
            ),
            (
                b"1970-01-01T12:34:56Z",
                b':',
                [
                    (LongTimestampField::Year, 0, 4, 1970),
                    (LongTimestampField::Month, 5, 7, 1),
                    (LongTimestampField::Day, 8, 10, 1),
                    (LongTimestampField::Hour, 11, 13, 12),
                    (LongTimestampField::Minute, 14, 16, 34),
                    (LongTimestampField::Second, 17, 19, 56),
                ],
            ),
            (
                b"1970-01-01t12_34_56-23_59",
                b'_',
                [
                    (LongTimestampField::Year, 0, 4, 1970),
                    (LongTimestampField::Month, 5, 7, 1),
                    (LongTimestampField::Day, 8, 10, 1),
                    (LongTimestampField::Hour, 11, 13, 12),
                    (LongTimestampField::Minute, 14, 16, 34),
                    (LongTimestampField::Second, 17, 19, 56),
                ],
            ),
        ];
        for (sample, separator, fields) in samples {
            for (field, start, end, expected) in fields {
                assert_eq!(
                    long_timestamp_field_bounds(sample, separator, field),
                    Some((start, end)),
                    "{field:?} in {sample:?}"
                );
                assert_eq!(
                    decode_long_timestamp_numeric_field(sample, separator, field),
                    Some(expected),
                    "{field:?} in {sample:?}"
                );
                for position in start..end {
                    for byte in 0u8..=u8::MAX {
                        let mut mutated = sample.to_vec();
                        mutated[position] = byte;
                        let expected = if byte.is_ascii_digit() {
                            let value = sample[start..end]
                                .iter()
                                .enumerate()
                                .map(|(index, current)| {
                                    if start + index == position {
                                        byte
                                    } else {
                                        *current
                                    }
                                })
                                .fold(0u64, |value, digit| value * 10 + u64::from(digit - b'0'));
                            Some(value)
                        } else {
                            None
                        };
                        assert_eq!(
                            decode_long_timestamp_numeric_field(&mutated, separator, field),
                            expected,
                            "{field:?} in {sample:?}, byte {byte:#04x} at {position}"
                        );
                    }
                }
            }
        }
        let malformed = [
            (&b"1970_01-01t12_34_56Z"[..], b'_'),
            (&b"1970-01-01t12:34_56Z"[..], b'_'),
            (&b"1970-01-01t12_34_56"[..], b'_'),
            (&b"19700101T123456"[..], b':'),
        ];
        for (input, separator) in malformed {
            for field in [
                LongTimestampField::Year,
                LongTimestampField::Month,
                LongTimestampField::Day,
                LongTimestampField::Hour,
                LongTimestampField::Minute,
                LongTimestampField::Second,
            ] {
                assert_eq!(
                    long_timestamp_field_bounds(input, separator, field),
                    None,
                    "{field:?} in malformed timestamp {input:?}"
                );
            }
        }
    }
}
