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
    if input.len() == 16 {
        let (consumed, _) = CanonicalTimestampFmt
            .parse(&input)
            .map_err(|_| NameError::Malformed)?;
        if consumed != input.len() {
            return Err(NameError::Malformed);
        }
    }
    let (year, month, day, hour, minute, second, zone) = match input.len() {
        16 if matches!(input[8], b'T' | b't') && matches!(input[15], b'Z' | b'z') => (
            &input[0..4],
            &input[4..6],
            &input[6..8],
            &input[9..11],
            &input[11..13],
            &input[13..15],
            &input[15..16],
        ),
        20 | 25
            if input[4] == b'-'
                && input[7] == b'-'
                && matches!(input[10], b'T' | b't')
                && input[13] == separator
                && input[16] == separator =>
        {
            (
                &input[0..4],
                &input[5..7],
                &input[8..10],
                &input[11..13],
                &input[14..16],
                &input[17..19],
                &input[19..],
            )
        }
        _ => return Err(NameError::Malformed),
    };
    let y = numeral(year, 10)? as i64;
    let m = numeral(month, 10)? as i64;
    let d = numeral(day, 10)? as i64;
    let h = numeral(hour, 10)? as i64;
    let min = numeral(minute, 10)? as i64;
    let sec = numeral(second, 10)? as i64;
    if !(1901..=2099).contains(&y) || !(1..=12).contains(&m) || h > 23 || min > 59 || sec > 59 {
        return Err(NameError::Malformed);
    }
    if !calendar_day_valid(y as u16, m as u8, d as u8) {
        return Err(NameError::Malformed);
    }
    let offset = match zone {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), a, b, sep, c, e] if *sep == separator => {
            let zh = numeral(&[*a, *b], 10)? as i64;
            let zm = numeral(&[*c, *e], 10)? as i64;
            if zh > 23 || zm > 59 {
                return Err(NameError::Malformed);
            }
            zone_offset_seconds(if *sign == b'+' { 1 } else { -1 }, zh, zm)
        }
        _ => return Err(NameError::Malformed),
    };
    let mut days = days_before_year(y as u16);
    for previous in 1..m {
        days += i64::from(month_length(y as u16, previous as u8));
    }
    // 1901-01-01 is exactly 25202 days before the Unix epoch.
    days += d - 1 - 25202;
    Ok(Timestamp(i128::from(
        days * 86400 + h * 3600 + min * 60 + sec - offset,
    )))
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
        calendar_day_valid, decode_base36, decode_decimal, parse_long_timestamp,
        time_separator_valid, NameError,
    };

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
}
