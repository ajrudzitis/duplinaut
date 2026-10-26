//! Bounded, timezone-independent duplicity basename parsing.
//!
//! The complete native filename parser remains unrefined against the reviewed grammar,
//! though individual kernels have contracts. Configuration-dependent literal prefixes
//! and Gregorian calendar arithmetic are explicit scalar kernels. No regular
//! expressions, locale, or host timezone are used.
//! Delimiter fields use a bounded scalar scanner and return borrowed source slices.

use crate::generated::names::CanonicalTimestampFmt;
#[cfg(verus_keep_ghost)]
use vest_lib::core::exec::parser::parse_matches_spec;
use vest_lib::core::exec::parser::Parser;
#[cfg(verus_keep_ghost)]
use vest_lib::core::spec::SpecParser;

use vstd::prelude::*;

verus! {
closed spec fn dot_field_boundary_model(input: Seq<u8>, boundary: nat) -> bool {
    boundary <= input.len()
        && forall|index: int| 0 <= index < boundary ==> input[index] != b'.'
        && (boundary == input.len() || input[boundary as int] == b'.')
}

/// Return the first dot offset, or the input length if no dot occurs.
fn dot_field_boundary(input: &[u8]) -> (boundary: usize)
    ensures dot_field_boundary_model(input@, boundary as nat),
{
    let mut boundary = 0usize;
    while boundary < input.len() && input[boundary] != b'.'
        invariant boundary <= input.len(),
            forall|index: int| 0 <= index < boundary as int ==> input@[index] != b'.',
        decreases input.len() - boundary,
    {
        boundary += 1;
    }
    proof {
        reveal(dot_field_boundary_model);
        assert(boundary <= input.len());
        assert(forall|index: int| 0 <= index < boundary as int ==> input@[index] != b'.');
        if boundary < input.len() { assert(input@[boundary as int] == b'.'); }
    }
    boundary
}

/// Return one exact dot-delimited field and the exact bytes after its dot.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-gen-002
//= type=implementation
//# A successful parser MUST report a consumed input range wholly contained within the supplied input.
fn split_dot_field(input: &[u8]) -> (result: (&[u8], Option<&[u8]>))
    ensures dot_field_boundary_model(input@, result.0.len() as nat),
        result.0@ == input@.subrange(0, result.0.len() as int),
        match result.1 {
            Some(rest) => result.0.len() < input.len()
                && rest@ == input@.subrange(result.0.len() as int + 1, input@.len() as int),
            None => result.0.len() == input.len()
                && result.0@ == input@,
        },
{
    let boundary = dot_field_boundary(input);
    if boundary < input.len() {
        let field = &input[..boundary];
        let rest = &input[boundary + 1..];
        proof {
            assert(field@ == input@.subrange(0, boundary as int));
            assert(rest@ == input@.subrange(boundary as int + 1, input@.len() as int));
        }
        (field, Some(rest))
    } else {
        (input, None)
    }
}

fn take_dot_field<'a>(remaining: &mut Option<&'a [u8]>) -> (field: Option<&'a [u8]>)
    ensures match *old(remaining) {
        Some(input) => match field {
            Some(value) => dot_field_boundary_model(input@, value.len() as nat)
                && value@ == input@.subrange(0, value.len() as int)
                && match *final(remaining) {
                    Some(rest) => value.len() < input.len()
                        && rest@ == input@.subrange(value.len() as int + 1, input@.len() as int),
                    None => value.len() == input.len() && value@ == input@,
                },
            None => false,
        },
        None => field.is_none() && *final(remaining) == None,
    },
{
    match remaining.take() {
        Some(input) => {
            let (field, rest) = split_dot_field(input);
            *remaining = rest;
            Some(field)
        }
        None => None,
    }
}

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

pub open spec fn optional_short_token(input: Option<&[u8]>) -> Option<Seq<u8>> {
    match input {
        Some(token) => Some(token@),
        None => None,
    }
}

pub open spec fn optional_short_token_matches(input: Option<&[u8]>, byte: u8) -> bool {
    match input {
        Some(token) => single_short_token_model(token@, byte),
        None => false,
    }
}

pub open spec fn single_short_token_model(token: Seq<u8>, byte: u8) -> bool {
    token.len() == 1 && token[0] == byte
}

pub fn short_token_matches(token: &[u8], byte: u8) -> (result: bool)
    ensures result == single_short_token_model(token@, byte),
{
    token.len() == 1 && token[0] == byte
}

pub open spec fn short_trailer_matches_model(kind: ObjectKind, trailer: Seq<u8>) -> bool {
    match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => {
            trailer.len() == 2 && trailer[0] == b'd' && trailer[1] == b't'
        }
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => {
            trailer.len() == 1 && trailer[0] == b'm'
        }
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => {
            trailer.len() == 2 && trailer[0] == b's' && trailer[1] == b't'
        }
    }
}

/// Exact short-name trailer, optional partial marker, envelope suffix, and no extras.
pub open spec fn short_trailer_model(
    kind: ObjectKind,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(Envelope, bool)> {
    if !short_trailer_matches_model(kind, trailer) || third.is_some() {
        None
    } else {
        match (first, second) {
            (None, None) => Some((Envelope::Plain, false)),
            (Some(token), None) if single_short_token_model(token, b'p') => {
                Some((Envelope::Plain, true))
            }
            (Some(token), None) if single_short_token_model(token, b'z') => {
                Some((Envelope::Gzip, false))
            }
            (Some(token), None) if single_short_token_model(token, b'g') => {
                Some((Envelope::OpenPgp, false))
            }
            (Some(marker), Some(token))
                if single_short_token_model(marker, b'p')
                    && single_short_token_model(token, b'z') =>
            {
                Some((Envelope::Gzip, true))
            }
            (Some(marker), Some(token))
                if single_short_token_model(marker, b'p')
                    && single_short_token_model(token, b'g') =>
            {
                Some((Envelope::OpenPgp, true))
            }
            _ => None,
        }
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub fn decode_short_trailer(
    kind: ObjectKind,
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Option<(Envelope, bool)>)
    ensures result == short_trailer_model(
        kind,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ),
{
    let trailer_matches = match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => {
            trailer.len() == 2 && trailer[0] == b'd' && trailer[1] == b't'
        }
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => {
            trailer.len() == 1 && trailer[0] == b'm'
        }
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => {
            trailer.len() == 2 && trailer[0] == b's' && trailer[1] == b't'
        }
    };
    proof {
        reveal(short_trailer_matches_model);
        match kind {
            ObjectKind::FullVolume | ObjectKind::IncrementalVolume => {
                assert(trailer_matches == short_trailer_matches_model(kind, trailer@));
            }
            ObjectKind::FullManifest | ObjectKind::IncrementalManifest => {
                assert(trailer_matches == short_trailer_matches_model(kind, trailer@));
            }
            ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => {
                assert(trailer_matches == short_trailer_matches_model(kind, trailer@));
            }
        }
    }
    if !trailer_matches {
        proof {
            reveal(short_trailer_model);
            assert(short_trailer_model(
                kind,
                trailer@,
                optional_short_token(first),
                optional_short_token(second),
                optional_short_token(third),
            ).is_none());
        }
        return None;
    }
    if third.is_some() {
        proof {
            reveal(short_trailer_model);
            assert(short_trailer_model(
                kind,
                trailer@,
                optional_short_token(first),
                optional_short_token(second),
                optional_short_token(third),
            ).is_none());
        }
        return None;
    }
    let first_p = match first {
        Some(token) => short_token_matches(token, b'p'),
        None => false,
    };
    let first_z = match first {
        Some(token) => short_token_matches(token, b'z'),
        None => false,
    };
    let first_g = match first {
        Some(token) => short_token_matches(token, b'g'),
        None => false,
    };
    let second_z = match second {
        Some(token) => short_token_matches(token, b'z'),
        None => false,
    };
    let second_g = match second {
        Some(token) => short_token_matches(token, b'g'),
        None => false,
    };
    let result = if first.is_none() && second.is_none() {
        Some((Envelope::Plain, false))
    } else if first_p && second.is_none() {
        Some((Envelope::Plain, true))
    } else if first_z && second.is_none() {
        Some((Envelope::Gzip, false))
    } else if first_g && second.is_none() {
        Some((Envelope::OpenPgp, false))
    } else if first_p && second_z {
        Some((Envelope::Gzip, true))
    } else if first_p && second_g {
        Some((Envelope::OpenPgp, true))
    } else {
        None
    };
    proof {
        reveal(short_trailer_model);
        reveal(optional_short_token);
        reveal(single_short_token_model);
        assert(first_p == optional_short_token_matches(first, b'p'));
        assert(first_z == optional_short_token_matches(first, b'z'));
        assert(first_g == optional_short_token_matches(first, b'g'));
        assert(second_z == optional_short_token_matches(second, b'z'));
        assert(second_g == optional_short_token_matches(second, b'g'));
        assert(result == short_trailer_model(
            kind,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ));
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongSuffixKind {
    Part,
    Gzip,
    OpenPgp,
    Pkcs7,
}

pub open spec fn long_suffix_token_model(token: Seq<u8>, kind: LongSuffixKind) -> bool {
    match kind {
        LongSuffixKind::Part => {
            token.len() == 4 && token[0] == b'p' && token[1] == b'a'
                && token[2] == b'r' && token[3] == b't'
        }
        LongSuffixKind::Gzip => token.len() == 2 && token[0] == b'g' && token[1] == b'z',
        LongSuffixKind::OpenPgp => {
            token.len() == 3 && token[0] == b'g' && token[1] == b'p' && token[2] == b'g'
        }
        LongSuffixKind::Pkcs7 => {
            token.len() == 3 && token[0] == b'p' && token[1] == b'7' && token[2] == b'm'
        }
    }
}

pub open spec fn optional_long_suffix_matches(
    token: Option<&[u8]>,
    kind: LongSuffixKind,
) -> bool {
    match token {
        Some(bytes) => long_suffix_token_model(bytes@, kind),
        None => false,
    }
}

pub fn long_suffix_token_matches(token: &[u8], kind: LongSuffixKind) -> (result: bool)
    ensures result == long_suffix_token_model(token@, kind),
{
    match kind {
        LongSuffixKind::Part => {
            token.len() == 4 && token[0] == b'p' && token[1] == b'a'
                && token[2] == b'r' && token[3] == b't'
        }
        LongSuffixKind::Gzip => token.len() == 2 && token[0] == b'g' && token[1] == b'z',
        LongSuffixKind::OpenPgp => {
            token.len() == 3 && token[0] == b'g' && token[1] == b'p' && token[2] == b'g'
        }
        LongSuffixKind::Pkcs7 => {
            token.len() == 3 && token[0] == b'p' && token[1] == b'7' && token[2] == b'm'
        }
    }
}

pub open spec fn long_trailer_matches_model(kind: ObjectKind, trailer: Seq<u8>) -> bool {
    match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => {
            trailer.len() == 7 && trailer[0] == b'd' && trailer[1] == b'i'
                && trailer[2] == b'f' && trailer[3] == b'f' && trailer[4] == b't'
                && trailer[5] == b'a' && trailer[6] == b'r'
        }
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => {
            trailer.len() == 8 && trailer[0] == b'm' && trailer[1] == b'a'
                && trailer[2] == b'n' && trailer[3] == b'i' && trailer[4] == b'f'
                && trailer[5] == b'e' && trailer[6] == b's' && trailer[7] == b't'
        }
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => {
            trailer.len() == 6 && trailer[0] == b's' && trailer[1] == b'i'
                && trailer[2] == b'g' && trailer[3] == b't' && trailer[4] == b'a'
                && trailer[5] == b'r'
        }
    }
}

pub fn long_trailer_matches(kind: ObjectKind, trailer: &[u8]) -> (result: bool)
    ensures result == long_trailer_matches_model(kind, trailer@),
{
    match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => {
            trailer.len() == 7 && trailer[0] == b'd' && trailer[1] == b'i'
                && trailer[2] == b'f' && trailer[3] == b'f' && trailer[4] == b't'
                && trailer[5] == b'a' && trailer[6] == b'r'
        }
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => {
            trailer.len() == 8 && trailer[0] == b'm' && trailer[1] == b'a'
                && trailer[2] == b'n' && trailer[3] == b'i' && trailer[4] == b'f'
                && trailer[5] == b'e' && trailer[6] == b's' && trailer[7] == b't'
        }
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => {
            trailer.len() == 6 && trailer[0] == b's' && trailer[1] == b'i'
                && trailer[2] == b'g' && trailer[3] == b't' && trailer[4] == b'a'
                && trailer[5] == b'r'
        }
    }
}

pub open spec fn long_trailer_suffix_model(
    kind: ObjectKind,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(Envelope, bool)> {
    if !long_trailer_matches_model(kind, trailer) || third.is_some() {
        None
    } else {
        match (first, second) {
            (None, None) => Some((Envelope::Plain, false)),
            (Some(token), None) if long_suffix_token_model(token, LongSuffixKind::Part) => {
                Some((Envelope::Plain, true))
            }
            (Some(token), None) if long_suffix_token_model(token, LongSuffixKind::Gzip) => {
                Some((Envelope::Gzip, false))
            }
            (Some(token), None) if long_suffix_token_model(token, LongSuffixKind::OpenPgp) => {
                Some((Envelope::OpenPgp, false))
            }
            (Some(token), None) if long_suffix_token_model(token, LongSuffixKind::Pkcs7) => {
                Some((Envelope::Pkcs7, false))
            }
            (Some(marker), Some(token))
                if long_suffix_token_model(marker, LongSuffixKind::Part)
                    && long_suffix_token_model(token, LongSuffixKind::Gzip) =>
            {
                Some((Envelope::Gzip, true))
            }
            (Some(marker), Some(token))
                if long_suffix_token_model(marker, LongSuffixKind::Part)
                    && long_suffix_token_model(token, LongSuffixKind::OpenPgp) =>
            {
                Some((Envelope::OpenPgp, true))
            }
            (Some(marker), Some(token))
                if long_suffix_token_model(marker, LongSuffixKind::Part)
                    && long_suffix_token_model(token, LongSuffixKind::Pkcs7) =>
            {
                Some((Envelope::Pkcs7, true))
            }
            _ => None,
        }
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub fn decode_long_trailer_suffix(
    kind: ObjectKind,
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Option<(Envelope, bool)>)
    ensures result == long_trailer_suffix_model(
        kind,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ),
{
    let trailer_matches = long_trailer_matches(kind, trailer);
    if !trailer_matches || third.is_some() {
        proof {
            reveal(long_trailer_suffix_model);
            assert(long_trailer_suffix_model(
                kind,
                trailer@,
                optional_short_token(first),
                optional_short_token(second),
                optional_short_token(third),
            ).is_none());
        }
        return None;
    }
    let first_part = match first {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::Part),
        None => false,
    };
    let first_gzip = match first {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::Gzip),
        None => false,
    };
    let first_openpgp = match first {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::OpenPgp),
        None => false,
    };
    let first_pkcs7 = match first {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::Pkcs7),
        None => false,
    };
    let second_gzip = match second {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::Gzip),
        None => false,
    };
    let second_openpgp = match second {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::OpenPgp),
        None => false,
    };
    let second_pkcs7 = match second {
        Some(token) => long_suffix_token_matches(token, LongSuffixKind::Pkcs7),
        None => false,
    };
    let result = if first.is_none() && second.is_none() {
        Some((Envelope::Plain, false))
    } else if first_part && second.is_none() {
        Some((Envelope::Plain, true))
    } else if first_gzip && second.is_none() {
        Some((Envelope::Gzip, false))
    } else if first_openpgp && second.is_none() {
        Some((Envelope::OpenPgp, false))
    } else if first_pkcs7 && second.is_none() {
        Some((Envelope::Pkcs7, false))
    } else if first_part && second_gzip {
        Some((Envelope::Gzip, true))
    } else if first_part && second_openpgp {
        Some((Envelope::OpenPgp, true))
    } else if first_part && second_pkcs7 {
        Some((Envelope::Pkcs7, true))
    } else {
        None
    };
    proof {
        reveal(long_trailer_suffix_model);
        reveal(long_suffix_token_model);
        reveal(optional_short_token);
        assert(first_part == optional_long_suffix_matches(first, LongSuffixKind::Part));
        assert(first_gzip == optional_long_suffix_matches(first, LongSuffixKind::Gzip));
        assert(first_openpgp == optional_long_suffix_matches(first, LongSuffixKind::OpenPgp));
        assert(first_pkcs7 == optional_long_suffix_matches(first, LongSuffixKind::Pkcs7));
        assert(second_gzip == optional_long_suffix_matches(second, LongSuffixKind::Gzip));
        assert(second_openpgp == optional_long_suffix_matches(second, LongSuffixKind::OpenPgp));
        assert(second_pkcs7 == optional_long_suffix_matches(second, LongSuffixKind::Pkcs7));
        assert(result == long_trailer_suffix_model(
            kind,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ));
    }
    result
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameError {
    Configuration,
    Limit,
    Malformed,
    Ambiguous,
}

closed spec fn object_kind_equal_model(left: ObjectKind, right: ObjectKind) -> bool {
    match (left, right) {
        (ObjectKind::FullVolume, ObjectKind::FullVolume)
        | (ObjectKind::IncrementalVolume, ObjectKind::IncrementalVolume)
        | (ObjectKind::FullManifest, ObjectKind::FullManifest)
        | (ObjectKind::IncrementalManifest, ObjectKind::IncrementalManifest)
        | (ObjectKind::FullSignatures, ObjectKind::FullSignatures)
        | (ObjectKind::IncrementalSignatures, ObjectKind::IncrementalSignatures) => true,
        _ => false,
    }
}

closed spec fn envelope_equal_model(left: Envelope, right: Envelope) -> bool {
    match (left, right) {
        (Envelope::Plain, Envelope::Plain)
        | (Envelope::Gzip, Envelope::Gzip)
        | (Envelope::OpenPgp, Envelope::OpenPgp)
        | (Envelope::Pkcs7, Envelope::Pkcs7) => true,
        _ => false,
    }
}

closed spec fn optional_volume_equal_model(left: Option<u64>, right: Option<u64>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

closed spec fn object_name_equal_model(left: ObjectName, right: ObjectName) -> bool {
    object_kind_equal_model(left.kind, right.kind)
        && left.start.0 == right.start.0
        && left.end.0 == right.end.0
        && optional_volume_equal_model(left.volume, right.volume)
        && envelope_equal_model(left.envelope, right.envelope)
        && left.partial == right.partial
        && left.short == right.short
}

#[allow(clippy::match_like_matches_macro)]
fn object_kind_equal(left: ObjectKind, right: ObjectKind) -> (result: bool)
    ensures result == object_kind_equal_model(left, right),
{
    match (left, right) {
        (ObjectKind::FullVolume, ObjectKind::FullVolume)
        | (ObjectKind::IncrementalVolume, ObjectKind::IncrementalVolume)
        | (ObjectKind::FullManifest, ObjectKind::FullManifest)
        | (ObjectKind::IncrementalManifest, ObjectKind::IncrementalManifest)
        | (ObjectKind::FullSignatures, ObjectKind::FullSignatures)
        | (ObjectKind::IncrementalSignatures, ObjectKind::IncrementalSignatures) => true,
        _ => false,
    }
}

#[allow(clippy::match_like_matches_macro)]
fn envelope_equal(left: Envelope, right: Envelope) -> (result: bool)
    ensures result == envelope_equal_model(left, right),
{
    match (left, right) {
        (Envelope::Plain, Envelope::Plain)
        | (Envelope::Gzip, Envelope::Gzip)
        | (Envelope::OpenPgp, Envelope::OpenPgp)
        | (Envelope::Pkcs7, Envelope::Pkcs7) => true,
        _ => false,
    }
}

fn optional_volume_equal(left: Option<u64>, right: Option<u64>) -> (result: bool)
    ensures result == optional_volume_equal_model(left, right),
{
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn object_name_equal(left: ObjectName, right: ObjectName) -> (result: bool)
    ensures result == object_name_equal_model(left, right),
{
    object_kind_equal(left.kind, right.kind)
        && left.start.0 == right.start.0
        && left.end.0 == right.end.0
        && optional_volume_equal(left.volume, right.volume)
        && envelope_equal(left.envelope, right.envelope)
        && left.partial == right.partial
        && left.short == right.short
}

closed spec fn name_candidate_merge_model(
    current: Option<ObjectName>,
    candidate: Option<ObjectName>,
) -> Result<Option<ObjectName>, NameError> {
    match candidate {
        None => Ok(current),
        Some(parsed) => match current {
            None => Ok(Some(parsed)),
            Some(previous) if object_name_equal_model(previous, parsed) => Ok(Some(previous)),
            Some(_) => Err(NameError::Ambiguous),
        },
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn merge_name_candidate(
    current: Option<ObjectName>,
    candidate: Option<ObjectName>,
) -> (result: Result<Option<ObjectName>, NameError>)
    ensures result == name_candidate_merge_model(current, candidate),
{
    proof {
        reveal(name_candidate_merge_model);
    }
    match candidate {
        None => {
            proof {
                reveal(name_candidate_merge_model);
                assert(name_candidate_merge_model(current, candidate) == Ok(current));
            }
            Ok(current)
        }
        Some(parsed) => match current {
            None => {
                proof {
                    reveal(name_candidate_merge_model);
                    assert(candidate == Some(parsed));
                    assert(name_candidate_merge_model(current, candidate) == Ok(Some(parsed)));
                }
                Ok(Some(parsed))
            }
            Some(previous) if object_name_equal(previous, parsed) => {
                proof {
                    reveal(name_candidate_merge_model);
                    assert(candidate == Some(parsed));
                    assert(name_candidate_merge_model(current, candidate) == Ok(Some(previous)));
                }
                Ok(Some(previous))
            }
            Some(_) => {
                proof {
                    reveal(name_candidate_merge_model);
                    assert(candidate == Some(parsed));
                    assert(name_candidate_merge_model(current, candidate) == Err(NameError::Ambiguous));
                }
                Err(NameError::Ambiguous)
            }
        },
    }
}

pub closed spec fn name_finalization_model(
    recognized: bool,
    candidate: Option<ObjectName>,
) -> Result<Option<ObjectName>, NameError> {
    match candidate {
        Some(parsed) => Ok(Some(parsed)),
        None if recognized => Err(NameError::Malformed),
        None => Ok(None),
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn finalize_name_classification(
    recognized: bool,
    candidate: Option<ObjectName>,
) -> (result: Result<Option<ObjectName>, NameError>)
    ensures result == name_finalization_model(recognized, candidate),
{
    proof {
        reveal(name_finalization_model);
    }
    match candidate {
        Some(parsed) => Ok(Some(parsed)),
        None if recognized => Err(NameError::Malformed),
        None => Ok(None),
    }
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

/// Unix-epoch seconds represented by a short-name base36 timestamp.
pub open spec fn short_timestamp_seconds_model(input: Seq<u8>) -> Option<int> {
    if input.len() == 0 {
        None
    } else {
        base36_value_model(input, input.len() as int)
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
pub fn decode_short_timestamp(input: &[u8]) -> (result: Option<Timestamp>)
    ensures match short_timestamp_seconds_model(input@) {
        Some(seconds) => result matches Some(timestamp) && timestamp.0 as int == seconds,
        None => result.is_none(),
    },
{
    match decode_base36(input) {
        Some(seconds) => {
            proof {
                reveal(short_timestamp_seconds_model);
                assert(input@.len() > 0);
                assert(base36_value_model(input@, input@.len() as int) == Some(seconds as int));
            }
            Some(Timestamp(seconds as i128))
        }
        None => {
            proof {
                reveal(short_timestamp_seconds_model);
                if input@.len() > 0 {
                    assert(base36_value_model(input@, input@.len() as int).is_none());
                }
            }
            None
        }
    }
}

/// Positive volume number encoded in a short duplicity filename.
pub open spec fn short_volume_number_model(input: Seq<u8>) -> Option<int> {
    match base36_value_model(input, input.len() as int) {
        Some(value) if value > 0 => Some(value),
        _ => None,
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
pub fn decode_short_volume_number(input: &[u8]) -> (result: Option<u64>)
    ensures match short_volume_number_model(input@) {
        Some(value) => result matches Some(actual) && actual as int == value,
        None => result.is_none(),
    },
{
    match decode_base36(input) {
        Some(number) if number > 0 => {
            proof {
                reveal(short_volume_number_model);
                assert(base36_value_model(input@, input@.len() as int) == Some(number as int));
            }
            Some(number)
        }
        Some(_) => {
            proof {
                reveal(short_volume_number_model);
                assert(base36_value_model(input@, input@.len() as int) == Some(0));
            }
            None
        }
        None => {
            proof {
                reveal(short_volume_number_model);
                if input@.len() == 0 {
                    reveal(base36_value_model);
                    assert(base36_value_model(input@, 0) == Some(0));
                } else {
                    assert(base36_value_model(input@, input@.len() as int).is_none());
                }
            }
            None
        }
    }
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

pub open spec fn decimal_max_value(width: int) -> int
    recommends width >= 0,
    decreases width,
{
    if width <= 0 { 0 } else { 10 * decimal_max_value(width - 1) + 9 }
}

pub proof fn decimal_max_two()
    ensures decimal_max_value(2) == 99,
{
    reveal_with_fuel(decimal_max_value, 3);
    assert(decimal_max_value(2) == 99);
}

pub proof fn decimal_max_four()
    ensures decimal_max_value(4) == 9999,
{
    reveal_with_fuel(decimal_max_value, 5);
    assert(decimal_max_value(4) == 9999);
}

pub proof fn decimal_value_is_bounded(bytes: Seq<u8>, prefix_len: int)
    requires 0 <= prefix_len <= bytes.len(),
        decimal_value_model(bytes, prefix_len).is_some(),
    ensures match decimal_value_model(bytes, prefix_len) {
        Some(value) => value <= decimal_max_value(prefix_len),
        None => true,
    },
    decreases prefix_len,
{
    if prefix_len > 0 {
        reveal(decimal_value_model);
        match decimal_value_model(bytes, prefix_len - 1) {
            Some(previous) => {
                decimal_value_is_bounded(bytes, prefix_len - 1);
                reveal(decimal_digit_model);
                assert(decimal_digit_model(bytes[prefix_len - 1]).is_some());
                let digit = match decimal_digit_model(bytes[prefix_len - 1]) {
                    Some(value) => value,
                    None => 0,
                };
                assert(digit <= 9);
                assert(previous * 10 + digit
                    <= decimal_max_value(prefix_len - 1) * 10 + 9);
                reveal(decimal_max_value);
            },
            None => {},
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

pub open spec fn long_volume_number_model(field: Seq<u8>) -> Option<int> {
    if field.len() < 3 {
        None
    } else if field[0] != b'v' || field[1] != b'o' || field[2] != b'l' || field.len() == 3 {
        None
    } else {
        match decimal_value_model(field.subrange(3, field.len() as int), field.len() as int - 3) {
            Some(value) if value > 0 => Some(value),
            _ => None,
        }
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-gen-003
//= type=implementation
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
pub fn decode_long_volume_number(field: &[u8]) -> (result: Option<u64>)
    ensures match long_volume_number_model(field@) {
        Some(value) => result matches Some(actual) && actual as int == value,
        None => result.is_none(),
    },
{
    if field.len() < 3 {
        proof {
            reveal(long_volume_number_model);
            assert(long_volume_number_model(field@).is_none());
        }
        return None;
    }
    if field[0] != b'v' || field[1] != b'o' || field[2] != b'l' {
        proof {
            reveal(long_volume_number_model);
            assert(long_volume_number_model(field@).is_none());
        }
        return None;
    }
    if field.len() == 3 {
        proof {
            reveal(long_volume_number_model);
            assert(long_volume_number_model(field@).is_none());
        }
        return None;
    }
    let digits = &field[3..];
    proof {
        reveal(long_volume_number_model);
        assert(field@[0] == b'v' && field@[1] == b'o' && field@[2] == b'l');
        assert(digits@ == field@.subrange(3, field@.len() as int));
    }
    match decode_decimal(digits) {
        Some(number) if number > 0 => {
            proof {
                reveal(long_volume_number_model);
                assert(decimal_value_model(digits@, digits@.len() as int) == Some(number as int));
            }
            Some(number)
        }
        Some(_) => {
            proof {
                reveal(long_volume_number_model);
                assert(decimal_value_model(digits@, digits@.len() as int) == Some(0));
            }
            None
        }
        None => {
            proof {
                reveal(long_volume_number_model);
                assert(decimal_value_model(digits@, digits@.len() as int).is_none());
            }
            None
        }
    }
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

/// Exact UTC seconds denoted by a valid canonical or legacy long timestamp.
/// Invalid separators are kept distinct by the parser as configuration errors.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub closed spec fn long_timestamp_numeric_components_model(
    input: Seq<u8>, separator: u8,
) -> Option<(int, int, int, int, int, int)> {
    match (
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Year),
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Month),
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Day),
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Hour),
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Minute),
        long_timestamp_numeric_field_model(input, separator, LongTimestampField::Second),
    ) {
        (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(s)) => Some((y, mo, d, h, mi, s)),
        _ => None,
    }
}

pub closed spec fn canonical_timestamp_components_model(
    input: Seq<u8>, separator: u8,
) -> Option<(int, int, int, int, int, int, int)> {
    if input.len() != 16 || !long_timestamp_framing_model(input, separator) {
        None
    } else {
        match CanonicalTimestampFmt.spec_parse(input) {
            Some((consumed, _parsed)) if consumed == input.len() =>
                match (long_timestamp_numeric_components_model(input, separator),
                    zone_offset_token_model(input.subrange(15, 16), separator)) {
                    (Some((y, mo, d, h, mi, s)), Some(offset)) => Some((y, mo, d, h, mi, s, offset)),
                    _ => None,
                },
            _ => None,
        }
    }
}

pub closed spec fn legacy_timestamp_components_model(
    input: Seq<u8>, separator: u8,
) -> Option<(int, int, int, int, int, int, int)> {
    if (input.len() != 20 && input.len() != 25) || !long_timestamp_framing_model(input, separator) {
        None
    } else {
        match (long_timestamp_numeric_components_model(input, separator),
            zone_offset_token_model(input.subrange(19, input.len() as int), separator)) {
            (Some((y, mo, d, h, mi, s)), Some(offset)) => Some((y, mo, d, h, mi, s, offset)),
            _ => None,
        }
    }
}

pub closed spec fn long_timestamp_components_model(
    input: Seq<u8>, separator: u8,
) -> Option<(int, int, int, int, int, int, int)> {
    if input.len() == 16 {
        canonical_timestamp_components_model(input, separator)
    } else {
        legacy_timestamp_components_model(input, separator)
    }
}

/// Exact UTC seconds denoted by a valid canonical or legacy long timestamp.
/// Invalid separators are kept distinct by the parser as configuration errors.
pub closed spec fn long_timestamp_result_model(input: Seq<u8>, separator: u8) -> Option<int> {
    if !time_separator_valid_model(separator) {
        None
    } else {
        match long_timestamp_components_model(input, separator) {
            Some((year, month, day, hour, minute, second, offset)) =>
                compose_timestamp_fields_model(year, month, day, hour, minute, second, offset),
            None => None,
        }
    }
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

pub proof fn long_timestamp_numeric_field_is_bounded(
    input: Seq<u8>, separator: u8, field: LongTimestampField,
)
    requires long_timestamp_numeric_field_model(input, separator, field).is_some(),
    ensures match long_timestamp_numeric_field_model(input, separator, field) {
        Some(value) => if field == LongTimestampField::Year { value <= 9999 } else { value <= 99 },
        None => false,
    },
{
    reveal(long_timestamp_numeric_field_model);
    reveal(long_timestamp_field_bounds_model);
    match long_timestamp_field_bounds_model(input, separator, field) {
        Some((start, end)) => {
            decimal_value_is_bounded(input.subrange(start, end), end - start);
            match field {
                LongTimestampField::Year => {
                    assert(end - start == 4);
                    decimal_max_four();
                    assert(decimal_max_value(end - start) == 9999);
                },
                _ => {
                    assert(end - start == 2);
                    decimal_max_two();
                    assert(decimal_max_value(end - start) == 99);
                },
            }
        },
        None => {},
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

/// Model the admission and UTC composition of already decoded timestamp
/// components. Calendar and clock fields are validated before the seconds
/// formula is used; offsets follow the explicit-zone bound.
pub open spec fn compose_timestamp_fields_model(
    year: int,
    month: int,
    day: int,
    hour: int,
    minute: int,
    second: int,
    offset_seconds: int,
) -> Option<int> {
    if calendar_date_valid_model(year, month, day)
        && clock_fields_valid_model(hour, minute, second)
        && -86340 <= offset_seconds
        && offset_seconds <= 86340
    {
        Some(calendar_fields_to_unix_seconds_model(
            year, month, day, hour, minute, second, offset_seconds,
        ))
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn compose_timestamp_fields(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    offset_seconds: i64,
) -> (result: Option<Timestamp>)
    ensures match compose_timestamp_fields_model(
        year as int, month as int, day as int,
        hour as int, minute as int, second as int, offset_seconds as int,
    ) {
        Some(seconds) => result matches Some(timestamp) && timestamp.0 as int == seconds,
        None => result.is_none(),
    },
{
    let date_valid = calendar_date_valid(year, month, day);
    let clock_valid = clock_fields_valid(hour, minute, second);
    if !date_valid
        || !clock_valid
        || !(-86340..=86340).contains(&offset_seconds)
    {
        proof {
            reveal(compose_timestamp_fields_model);
            if !date_valid {
                reveal(calendar_date_valid_model);
            }
            if !clock_valid {
                reveal(clock_fields_valid_model);
            }
            assert(compose_timestamp_fields_model(
                year as int, month as int, day as int,
                hour as int, minute as int, second as int, offset_seconds as int,
            ).is_none());
        }
        return None;
    }

    proof {
        reveal(compose_timestamp_fields_model);
        reveal(calendar_date_valid_model);
        reveal(clock_fields_valid_model);
        assert(calendar_date_valid_model(year as int, month as int, day as int));
        assert(clock_fields_valid_model(hour as int, minute as int, second as int));
        assert(-86340 <= offset_seconds as int <= 86340);
        assert(compose_timestamp_fields_model(
            year as int, month as int, day as int,
            hour as int, minute as int, second as int, offset_seconds as int,
        ) == Some(calendar_fields_to_unix_seconds_model(
            year as int, month as int, day as int,
            hour as int, minute as int, second as int, offset_seconds as int,
        )));
        assert(1901 <= year as int <= 2099);
        assert(1 <= month as int <= 12);
        assert(1 <= day as int <= 31);
        assert(0 <= hour as int <= 23);
        assert(0 <= minute as int <= 59);
        assert(0 <= second as int <= 59);
    }
    Some(calendar_fields_to_unix_seconds(
        year as u16,
        month as u8,
        day as u8,
        hour as u8,
        minute as u8,
        second as u8,
        offset_seconds,
    ))
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

pub open spec fn short_kind_is_incremental_model(kind: ObjectKind) -> bool {
    matches!(kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures)
}

pub open spec fn short_interval_model(
    kind: ObjectKind,
    start: Seq<u8>,
    end: Option<Seq<u8>>,
) -> Option<(int, int)> {
    match (short_kind_is_incremental_model(kind), short_timestamp_seconds_model(start), end) {
        (false, Some(start_seconds), None) => Some((start_seconds, start_seconds)),
        (true, Some(start_seconds), Some(end_bytes)) =>
            match short_timestamp_seconds_model(end_bytes) {
                Some(end_seconds) if end_seconds > start_seconds => Some((start_seconds, end_seconds)),
                _ => None,
            },
        _ => None,
    }
}

closed spec fn short_full_volume_fields_model(
    start_field: Seq<u8>,
    volume_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, int, Envelope, bool)> {
    match (
        short_interval_model(ObjectKind::FullVolume, start_field, None),
        short_volume_number_model(volume_field),
        short_trailer_model(ObjectKind::FullVolume, trailer, first, second, None),
    ) {
        (Some((start, end)), Some(volume), Some((envelope, partial))) =>
            Some((start, end, volume, envelope, partial)),
        _ => None,
    }
}

closed spec fn short_full_manifest_fields_model(
    start_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        short_interval_model(ObjectKind::FullManifest, start_field, None),
        short_trailer_model(ObjectKind::FullManifest, trailer, first, second, None),
    ) {
        (Some((start, end)), Some((envelope, partial))) =>
            Some((start, end, envelope, partial)),
        _ => None,
    }
}

closed spec fn short_full_signatures_fields_model(
    start_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        short_interval_model(ObjectKind::FullSignatures, start_field, None),
        short_trailer_model(ObjectKind::FullSignatures, trailer, first, second, None),
    ) {
        (Some((start, end)), Some((envelope, partial))) =>
            Some((start, end, envelope, partial)),
        _ => None,
    }
}


closed spec fn short_incremental_signatures_fields_model(
    start_field: Seq<u8>,
    end_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        short_interval_model(
            ObjectKind::IncrementalSignatures,
            start_field,
            Some(end_field),
        ),
        short_trailer_model(
            ObjectKind::IncrementalSignatures,
            trailer,
            first,
            second,
            None,
        ),
    ) {
        (Some((start, end)), Some((envelope, partial))) => {
            Some((start, end, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn short_incremental_manifest_fields_model(
    start_field: Seq<u8>,
    end_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        short_interval_model(
            ObjectKind::IncrementalManifest,
            start_field,
            Some(end_field),
        ),
        short_trailer_model(
            ObjectKind::IncrementalManifest,
            trailer,
            first,
            second,
            None,
        ),
    ) {
        (Some((start, end)), Some((envelope, partial))) => {
            Some((start, end, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn short_incremental_volume_fields_model(
    start_field: Seq<u8>,
    end_field: Seq<u8>,
    volume_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
) -> Option<(int, int, int, Envelope, bool)> {
    match (
        short_interval_model(
            ObjectKind::IncrementalVolume,
            start_field,
            Some(end_field),
        ),
        short_volume_number_model(volume_field),
        short_trailer_model(ObjectKind::IncrementalVolume, trailer, first, second, None),
    ) {
        (Some((start, end)), Some(volume), Some((envelope, partial))) =>
            Some((start, end, volume, envelope, partial)),
        _ => None,
    }
}

//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
pub fn decode_short_interval(
    kind: ObjectKind,
    start: &[u8],
    end: Option<&[u8]>,
) -> (result: Option<(Timestamp, Timestamp)>)
    ensures match short_interval_model(kind, start@, optional_short_token(end)) {
        Some((start_seconds, end_seconds)) => result matches Some(interval)
            && interval.0.0 as int == start_seconds
            && interval.1.0 as int == end_seconds,
        None => result.is_none(),
    },
{
    let incremental = matches!(
        kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures
    );
    let start_timestamp = decode_short_timestamp(start)?;
    match (incremental, end) {
        (false, None) => {
            proof {
                reveal(short_interval_model);
                reveal(short_kind_is_incremental_model);
                reveal(short_timestamp_seconds_model);
                assert(short_interval_model(kind, start@, optional_short_token(end))
                    == Some((start_timestamp.0 as int, start_timestamp.0 as int)));
            }
            Some((start_timestamp, start_timestamp))
        }
        (true, Some(end_bytes)) => match decode_short_timestamp(end_bytes) {
            Some(end_timestamp) if valid_set_interval(true, start_timestamp, end_timestamp) => {
                proof {
                    reveal(short_interval_model);
                    reveal(short_kind_is_incremental_model);
                    reveal(short_timestamp_seconds_model);
                    assert(start_timestamp.0 as int
                        == short_timestamp_seconds_model(start@).unwrap());
                    assert(end_timestamp.0 as int
                        == short_timestamp_seconds_model(end_bytes@).unwrap());
                    assert(end_timestamp.0 > start_timestamp.0);
                }
                Some((start_timestamp, end_timestamp))
            }
            _ => {
                proof {
                    reveal(short_interval_model);
                    reveal(short_kind_is_incremental_model);
                    if let Some(end_bytes) = end {
                        if let Some(end_seconds) = short_timestamp_seconds_model(end_bytes@) {
                            assert(end_seconds <= start_timestamp.0 as int);
                        }
                    }
                }
                None
            }
        },
        _ => {
            proof {
                reveal(short_interval_model);
                reveal(short_kind_is_incremental_model);
            }
            None
        }
    }
}

pub open spec fn long_kind_is_incremental_model(kind: ObjectKind) -> bool {
    matches!(kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures)
}

pub open spec fn long_to_marker_model(marker: Seq<u8>) -> bool {
    marker.len() == 2 && marker[0] == b't' && marker[1] == b'o'
}

pub fn long_to_marker_matches(marker: &[u8]) -> (result: bool)
    ensures result == long_to_marker_model(marker@),
{
    marker.len() == 2 && marker[0] == b't' && marker[1] == b'o'
}

pub open spec fn long_interval_model(
    kind: ObjectKind,
    start: Seq<u8>,
    separator: u8,
    marker: Option<Seq<u8>>,
    end: Option<Seq<u8>>,
) -> Option<(int, int)> {
    if !time_separator_valid_model(separator) {
        None
    } else if long_kind_is_incremental_model(kind) {
        match (marker, end) {
            (Some(marker_bytes), Some(end_bytes))
                if long_to_marker_model(marker_bytes) => match (
                long_timestamp_result_model(start, separator),
                long_timestamp_result_model(end_bytes, separator),
            ) {
                (Some(start_seconds), Some(end_seconds)) if end_seconds > start_seconds => {
                    Some((start_seconds, end_seconds))
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        match (marker, end, long_timestamp_result_model(start, separator)) {
            (None, None, Some(start_seconds)) => Some((start_seconds, start_seconds)),
            _ => None,
        }
    }
}

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
#[allow(clippy::manual_map)]
pub fn decode_long_interval(
    kind: ObjectKind,
    start: &[u8],
    separator: u8,
    marker: Option<&[u8]>,
    end: Option<&[u8]>,
) -> (result: Option<(Timestamp, Timestamp)>)
    ensures match long_interval_model(
        kind,
        start@,
        separator,
        optional_short_token(marker),
        optional_short_token(end),
    ) {
        Some((start_seconds, end_seconds)) => result matches Some(interval)
            && interval.0.0 as int == start_seconds
            && interval.1.0 as int == end_seconds,
        None => result.is_none(),
    },
{
    if !time_separator_valid(separator) {
        proof {
            reveal(long_interval_model);
            reveal(time_separator_valid_model);
            assert(!time_separator_valid_model(separator));
        }
        return None;
    }
    proof {
        assert(time_separator_valid_model(separator));
    }
    let incremental = matches!(
        kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures
    );
    let marker_is_to = match marker {
        Some(token) => Some(long_to_marker_matches(token)),
        None => None,
    };
    proof {
        reveal(long_to_marker_model);
        match marker {
            Some(token) => assert(marker_is_to == Some(long_to_marker_model(token@))),
            None => assert(marker_is_to.is_none()),
        }
        reveal(long_kind_is_incremental_model);
        assert(incremental == long_kind_is_incremental_model(kind));
    }
    match (incremental, marker_is_to, end) {
        (false, None, None) => {
            let start_result = parse_long_timestamp(start, separator);
            let start_timestamp = match start_result {
                Ok(timestamp) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        match long_timestamp_result_model(start@, separator) {
                            Some(seconds) => {
                                assert(start_result == Ok(Timestamp(seconds as i128)));
                                assert(timestamp == Timestamp(seconds as i128));
                            }
                            None => assert(false),
                        }
                    }
                    timestamp
                }
                Err(_) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        assert(long_timestamp_result_model(start@, separator).is_none());
                        reveal(long_interval_model);
                        assert(long_interval_model(
                            kind,
                            start@,
                            separator,
                            optional_short_token(marker),
                            optional_short_token(end),
                        ).is_none());
                    }
                    return None;
                }
            };
            proof {
                reveal(long_interval_model);
                reveal(long_kind_is_incremental_model);
                reveal(long_timestamp_result_model);
                assert(!long_kind_is_incremental_model(kind));
                assert(marker.is_none());
                assert(end.is_none());
                assert(long_timestamp_result_model(start@, separator)
                    == Some(start_timestamp.0 as int));
                assert(long_interval_model(
                    kind,
                    start@,
                    separator,
                    optional_short_token(marker),
                    optional_short_token(end),
                ) == Some((start_timestamp.0 as int, start_timestamp.0 as int)));
            }
            Some((start_timestamp, start_timestamp))
        }
        (true, Some(true), Some(end_bytes)) => {
            let start_result = parse_long_timestamp(start, separator);
            let start_timestamp = match start_result {
                Ok(timestamp) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        match long_timestamp_result_model(start@, separator) {
                            Some(seconds) => {
                                assert(start_result == Ok(Timestamp(seconds as i128)));
                                assert(timestamp == Timestamp(seconds as i128));
                            }
                            None => assert(false),
                        }
                    }
                    timestamp
                }
                Err(_) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        assert(long_timestamp_result_model(start@, separator).is_none());
                        reveal(long_interval_model);
                        assert(long_interval_model(
                            kind,
                            start@,
                            separator,
                            optional_short_token(marker),
                            optional_short_token(end),
                        ).is_none());
                    }
                    return None;
                }
            };
            let end_result = parse_long_timestamp(end_bytes, separator);
            let end_timestamp = match end_result {
                Ok(timestamp) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        match long_timestamp_result_model(end_bytes@, separator) {
                            Some(seconds) => {
                                assert(end_result == Ok(Timestamp(seconds as i128)));
                                assert(timestamp == Timestamp(seconds as i128));
                            }
                            None => assert(false),
                        }
                    }
                    timestamp
                }
                Err(_) => {
                    proof {
                        reveal(long_timestamp_result_model);
                        assert(long_timestamp_result_model(end_bytes@, separator).is_none());
                        reveal(long_interval_model);
                        assert(long_timestamp_result_model(start@, separator).is_some());
                        assert(long_interval_model(
                            kind,
                            start@,
                            separator,
                            optional_short_token(marker),
                            optional_short_token(end),
                        ).is_none());
                    }
                    return None;
                }
            };
            if !valid_set_interval(true, start_timestamp, end_timestamp) {
                proof {
                    reveal(long_interval_model);
                    reveal(long_kind_is_incremental_model);
                    reveal(long_timestamp_result_model);
                    assert(long_timestamp_result_model(start@, separator)
                        == Some(start_timestamp.0 as int));
                    assert(long_timestamp_result_model(end_bytes@, separator)
                        == Some(end_timestamp.0 as int));
                    assert(end_timestamp.0 <= start_timestamp.0);
                    assert(long_interval_model(
                        kind,
                        start@,
                        separator,
                        optional_short_token(marker),
                        optional_short_token(end),
                    ).is_none());
                }
                return None;
            }
            proof {
                reveal(long_interval_model);
                reveal(long_kind_is_incremental_model);
                reveal(long_timestamp_result_model);
                assert(long_timestamp_result_model(start@, separator)
                    == Some(start_timestamp.0 as int));
                assert(long_timestamp_result_model(end_bytes@, separator)
                    == Some(end_timestamp.0 as int));
                assert(end_timestamp.0 > start_timestamp.0);
                assert(long_interval_model(
                    kind,
                    start@,
                    separator,
                    optional_short_token(marker),
                    optional_short_token(end),
                ) == Some((start_timestamp.0 as int, end_timestamp.0 as int)));
            }
            Some((start_timestamp, end_timestamp))
        }
        _ => {
            proof {
                reveal(long_interval_model);
                reveal(long_kind_is_incremental_model);
                reveal(long_timestamp_result_model);
                assert(long_interval_model(
                    kind,
                    start@,
                    separator,
                    optional_short_token(marker),
                    optional_short_token(end),
                ).is_none());
            }
            None
        }
    }
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

//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
pub fn parse_long_timestamp(input: &[u8], separator: u8) -> (result: Result<Timestamp, NameError>)
    ensures if !time_separator_valid_model(separator) {
        result == Err(NameError::Configuration)
    } else {
        (match long_timestamp_result_model(input@, separator) {
            Some(seconds) => result == Ok(Timestamp(seconds as i128)),
            None => result == Err(NameError::Malformed),
        })
        && match result {
            Ok(timestamp) => long_timestamp_result_model(input@, separator)
                == Some(timestamp.0 as int),
            Err(NameError::Malformed) => long_timestamp_result_model(input@, separator).is_none(),
            _ => false,
        }
    },
{
    if !time_separator_valid(separator) {
        return Err(NameError::Configuration);
    }
    if !long_timestamp_framing_valid(input, separator) {
        return Err(NameError::Malformed);
    }
    if input.len() == 16 {
        let vest_result = CanonicalTimestampFmt.parse(&input);
        proof {
            assert(parse_matches_spec(
                vest_result,
                CanonicalTimestampFmt.spec_parse(input@),
            ));
        }
        let (consumed, _parsed) = match vest_result {
            Ok(parsed) => parsed,
            Err(_error) => return Err(NameError::Malformed),
        };
        if consumed != input.len() {
            return Err(NameError::Malformed);
        }
    }
    let zone = match input.len() {
        16 => &input[15..16],
        20 | 25 => &input[19..],
        _ => return Err(NameError::Malformed),
    };
    let year_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Year) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Year) == Some(year_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Year);
        assert(year_raw as int <= 9999);
    }
    let year = year_raw as i64;
    let month_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Month) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Month) == Some(month_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Month);
        assert(month_raw as int <= 99);
    }
    let month = month_raw as i64;
    let day_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Day) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Day) == Some(day_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Day);
        assert(day_raw as int <= 99);
    }
    let day = day_raw as i64;
    let hour_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Hour) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Hour) == Some(hour_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Hour);
        assert(hour_raw as int <= 99);
    }
    let hour = hour_raw as i64;
    let minute_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Minute) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Minute) == Some(minute_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Minute);
        assert(minute_raw as int <= 99);
    }
    let minute = minute_raw as i64;
    let second_raw = match decode_long_timestamp_numeric_field(input, separator, LongTimestampField::Second) {
        Some(value) => value,
        None => return Err(NameError::Malformed),
    };
    proof {
        assert(long_timestamp_numeric_field_model(input@, separator, LongTimestampField::Second) == Some(second_raw as int));
        long_timestamp_numeric_field_is_bounded(input@, separator, LongTimestampField::Second);
        assert(second_raw as int <= 99);
    }
    let second = second_raw as i64;
    let offset = decode_zone_offset_token(zone, separator).ok_or(NameError::Malformed)?;
    let composed = compose_timestamp_fields(year, month, day, hour, minute, second, offset);
    proof {
        reveal(long_timestamp_result_model);
        reveal(long_timestamp_components_model);
        if input.len() == 16 {
            reveal(canonical_timestamp_components_model);
        } else {
            reveal(legacy_timestamp_components_model);
        }
        reveal(long_timestamp_numeric_components_model);
        assert(long_timestamp_numeric_components_model(input@, separator)
            == Some((year as int, month as int, day as int, hour as int, minute as int, second as int)));
        reveal(zone_offset_token_model);
        assert(zone_offset_token_model(zone@, separator) == Some(offset as int));
        reveal(compose_timestamp_fields_model);
        assert(long_timestamp_result_model(input@, separator)
            == compose_timestamp_fields_model(year as int, month as int, day as int,
                hour as int, minute as int, second as int, offset as int));
    }
    composed.ok_or(NameError::Malformed)
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

const KINDS: [ObjectKind; 6] = [
    ObjectKind::FullVolume,
    ObjectKind::IncrementalVolume,
    ObjectKind::FullManifest,
    ObjectKind::IncrementalManifest,
    ObjectKind::FullSignatures,
    ObjectKind::IncrementalSignatures,
];

verus! {
closed spec fn stem_model(kind: ObjectKind, short: bool) -> Seq<u8> {
    match (kind, short) {
        (ObjectKind::FullVolume | ObjectKind::FullManifest, false) => b"duplicity-full."@,
        (ObjectKind::IncrementalVolume | ObjectKind::IncrementalManifest, false) => b"duplicity-inc."@,
        (ObjectKind::FullSignatures, false) => b"duplicity-full-signatures."@,
        (ObjectKind::IncrementalSignatures, false) => b"duplicity-new-signatures."@,
        (ObjectKind::FullVolume | ObjectKind::FullManifest, true) => b"df."@,
        (ObjectKind::IncrementalVolume | ObjectKind::IncrementalManifest, true) => b"di."@,
        (ObjectKind::FullSignatures, true) => b"dfs."@,
        (ObjectKind::IncrementalSignatures, true) => b"dns."@,
    }
}

fn stem(kind: ObjectKind, short: bool) -> (result: &'static [u8])
    ensures result@ == stem_model(kind, short),
{
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
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
verus! {
fn short_full_volume_body_fields(
    start_field: &[u8],
    volume_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_full_volume_fields_model(
            start_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_full_volume_fields_model(
            start_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, volume, envelope, partial)) =>
                name.kind == ObjectKind::FullVolume
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && (name.volume matches Some(actual) && actual as int == volume)
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(ObjectKind::FullVolume, start_field, None)
        .ok_or(NameError::Malformed)?;
    let volume = decode_short_volume_number(volume_field).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::FullVolume,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullVolume,
        start,
        end,
        volume: Some(volume),
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_full_volume_fields_model);
        assert(short_interval_model(ObjectKind::FullVolume, start_field@, None)
            == Some((start.0 as int, end.0 as int)));
        assert(short_volume_number_model(volume_field@) == Some(volume as int));
        assert(short_trailer_model(
            ObjectKind::FullVolume,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
verus! {
fn short_full_manifest_body_fields(
    start_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_full_manifest_fields_model(
            start_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_full_manifest_fields_model(
            start_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::FullManifest
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(ObjectKind::FullManifest, start_field, None)
        .ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::FullManifest,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullManifest,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_full_manifest_fields_model);
        assert(short_interval_model(ObjectKind::FullManifest, start_field@, None)
            == Some((start.0 as int, end.0 as int)));
        assert(short_trailer_model(
            ObjectKind::FullManifest,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
verus! {
fn short_full_signatures_body_fields(
    start_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_full_signatures_fields_model(
            start_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_full_signatures_fields_model(
            start_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::FullSignatures
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(ObjectKind::FullSignatures, start_field, None)
        .ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::FullSignatures,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullSignatures,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_full_signatures_fields_model);
        assert(short_interval_model(ObjectKind::FullSignatures, start_field@, None)
            == Some((start.0 as int, end.0 as int)));
        assert(short_trailer_model(
            ObjectKind::FullSignatures,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}
}


//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
verus! {
fn short_incremental_signatures_body_fields(
    start_field: &[u8],
    end_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_incremental_signatures_fields_model(
            start_field@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_incremental_signatures_fields_model(
            start_field@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalSignatures
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(
        ObjectKind::IncrementalSignatures,
        start_field,
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::IncrementalSignatures,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalSignatures,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_incremental_signatures_fields_model);
        assert(short_interval_model(
            ObjectKind::IncrementalSignatures,
            start_field@,
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(short_trailer_model(
            ObjectKind::IncrementalSignatures,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
verus! {
fn short_incremental_manifest_body_fields(
    start_field: &[u8],
    end_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_incremental_manifest_fields_model(
            start_field@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_incremental_manifest_fields_model(
            start_field@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalManifest
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(
        ObjectKind::IncrementalManifest,
        start_field,
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::IncrementalManifest,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalManifest,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_incremental_manifest_fields_model);
        assert(short_interval_model(
            ObjectKind::IncrementalManifest,
            start_field@,
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(short_trailer_model(
            ObjectKind::IncrementalManifest,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-003
//= type=implementation
//# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
verus! {
fn short_incremental_volume_body_fields(
    start_field: &[u8],
    end_field: &[u8],
    volume_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(_) => short_incremental_volume_fields_model(
            start_field@,
            end_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ).is_none(),
        Ok(name) => match short_incremental_volume_fields_model(
            start_field@,
            end_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, volume, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalVolume
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && (name.volume matches Some(actual) && actual as int == volume)
                    && name.envelope == envelope
                    && name.partial == partial
                    && name.short,
            None => false,
        },
    },
{
    let (start, end) = decode_short_interval(
        ObjectKind::IncrementalVolume,
        start_field,
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let volume = decode_short_volume_number(volume_field).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_short_trailer(
        ObjectKind::IncrementalVolume,
        trailer,
        first,
        second,
        None,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalVolume,
        start,
        end,
        volume: Some(volume),
        envelope,
        partial,
        short: true,
    };
    proof {
        reveal(short_incremental_volume_fields_model);
        assert(short_interval_model(
            ObjectKind::IncrementalVolume,
            start_field@,
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(short_volume_number_model(volume_field@) == Some(volume as int));
        assert(short_trailer_model(
            ObjectKind::IncrementalVolume,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            None,
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
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
    if kind == ObjectKind::IncrementalSignatures && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_incremental_signatures_body_fields(
            start_field,
            end_field,
            trailer,
            first,
            second,
        );
    }
    if kind == ObjectKind::IncrementalManifest && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_incremental_manifest_body_fields(
            start_field,
            end_field,
            trailer,
            first,
            second,
        );
    }
    if kind == ObjectKind::FullManifest && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_full_manifest_body_fields(start_field, trailer, first, second);
    }
    if kind == ObjectKind::FullSignatures && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_full_signatures_body_fields(start_field, trailer, first, second);
    }
    if kind == ObjectKind::IncrementalVolume && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_incremental_volume_body_fields(
            start_field,
            end_field,
            volume_field,
            trailer,
            first,
            second,
        );
    }
    if kind == ObjectKind::FullVolume && short {
        let mut fields = Some(input);
        let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if third.is_some() || fields.is_some() {
            return Err(NameError::Malformed);
        }
        return short_full_volume_body_fields(start_field, volume_field, trailer, first, second);
    }
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let incremental = matches!(
        kind,
        ObjectKind::IncrementalVolume
            | ObjectKind::IncrementalManifest
            | ObjectKind::IncrementalSignatures
    );
    let (start, end) = if short {
        let end_field = if incremental {
            Some(take_dot_field(&mut fields).ok_or(NameError::Malformed)?)
        } else {
            None
        };
        //= docs/parsing-requirements.md#parse-name-003
        //= type=implementation
        //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
        //= docs/parsing-requirements.md#parse-name-006
        //= type=implementation
        //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
        decode_short_interval(kind, start_field, end_field).ok_or(NameError::Malformed)?
    } else {
        let marker = if incremental {
            take_dot_field(&mut fields)
        } else {
            None
        };
        let end_field = if marker.is_some() {
            take_dot_field(&mut fields)
        } else {
            None
        };
        //= docs/parsing-requirements.md#parse-name-004
        //= type=implementation
        //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
        //= docs/parsing-requirements.md#parse-name-006
        //= type=implementation
        //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
        //= docs/parsing-requirements.md#parse-name-007
        //= type=implementation
        //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
        decode_long_interval(kind, start_field, separator, marker, end_field)
            .ok_or(NameError::Malformed)?
    };
    let volume = if matches!(kind, ObjectKind::FullVolume | ObjectKind::IncrementalVolume) {
        let field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        //= docs/parsing-requirements.md#parse-name-003
        //= type=implementation
        //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
        let number = if short {
            decode_short_volume_number(field).ok_or(NameError::Malformed)?
        } else {
            //= docs/parsing-requirements.md#parse-name-001
            //= type=implementation
            //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
            //= docs/parsing-requirements.md#parse-gen-003
            //= type=implementation
            //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
            decode_long_volume_number(field).ok_or(NameError::Malformed)?
        };
        Some(number)
    } else {
        None
    };
    let (envelope, partial) = if short {
        //= docs/parsing-requirements.md#parse-name-001
        //= type=implementation
        //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if take_dot_field(&mut fields).is_some() {
            return Err(NameError::Malformed);
        }
        decode_short_trailer(kind, trailer, first, second, third).ok_or(NameError::Malformed)?
    } else {
        //= docs/parsing-requirements.md#parse-name-001
        //= type=implementation
        //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
        let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
        let first = take_dot_field(&mut fields);
        let second = take_dot_field(&mut fields);
        let third = take_dot_field(&mut fields);
        if take_dot_field(&mut fields).is_some() {
            return Err(NameError::Malformed);
        }
        decode_long_trailer_suffix(kind, trailer, first, second, third)
            .ok_or(NameError::Malformed)?
    };
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

verus! {
pub open spec fn literal_prefix_model(input: Seq<u8>, prefix: Seq<u8>) -> bool {
    prefix.len() <= input.len()
        && forall|index: int| 0 <= index < prefix.len() ==> input[index] == prefix[index]
}

pub open spec fn configured_prefix_remainder_model(
    input: Seq<u8>,
    global_prefix: Seq<u8>,
    kind_prefix: Seq<u8>,
) -> Option<Seq<u8>> {
    let global_end = global_prefix.len() as int;
    let kind_end = global_end + kind_prefix.len() as int;
    if literal_prefix_model(input, global_prefix)
        && literal_prefix_model(input.subrange(global_end, input.len() as int), kind_prefix)
    {
        Some(input.subrange(kind_end, input.len() as int))
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-name-002
//= type=implementation
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
pub fn configured_prefix_remainder<'a>(
    input: &'a [u8],
    global_prefix: &[u8],
    kind_prefix: &[u8],
) -> (result: Option<&'a [u8]>)
    ensures match result {
        Some(rest) => configured_prefix_remainder_model(input@, global_prefix@, kind_prefix@)
            == Some(rest@),
        None => configured_prefix_remainder_model(input@, global_prefix@, kind_prefix@).is_none(),
    },
{
    if input.len() < global_prefix.len() {
        proof {
            reveal(configured_prefix_remainder_model);
            reveal(literal_prefix_model);
            assert(!literal_prefix_model(input@, global_prefix@));
        }
        return None;
    }
    let mut global_index = 0usize;
    while global_index < global_prefix.len()
        invariant global_index <= global_prefix.len(),
            global_prefix.len() <= input.len(),
            forall|index: int| 0 <= index < global_index as int
                ==> input@[index] == global_prefix@[index],
        decreases global_prefix.len() - global_index,
    {
        if input[global_index] != global_prefix[global_index] {
            proof {
                reveal(configured_prefix_remainder_model);
                reveal(literal_prefix_model);
                assert(!literal_prefix_model(input@, global_prefix@));
            }
            return None;
        }
        global_index += 1;
    }
    let after_global = &input[global_prefix.len()..];
    proof {
        reveal(literal_prefix_model);
        assert(literal_prefix_model(input@, global_prefix@));
        assert(after_global@ == input@.subrange(global_prefix.len() as int, input.len() as int));
    }
    if after_global.len() < kind_prefix.len() {
        proof {
            reveal(configured_prefix_remainder_model);
            reveal(literal_prefix_model);
            assert(!literal_prefix_model(after_global@, kind_prefix@));
        }
        return None;
    }
    let mut kind_index = 0usize;
    while kind_index < kind_prefix.len()
        invariant kind_index <= kind_prefix.len(),
            kind_prefix.len() <= after_global.len(),
            after_global@ == input@.subrange(global_prefix.len() as int, input.len() as int),
            forall|index: int| 0 <= index < kind_index as int
                ==> after_global@[index] == kind_prefix@[index],
        decreases kind_prefix.len() - kind_index,
    {
        if after_global[kind_index] != kind_prefix[kind_index] {
            proof {
                reveal(configured_prefix_remainder_model);
                reveal(literal_prefix_model);
                assert(!literal_prefix_model(after_global@, kind_prefix@));
            }
            return None;
        }
        kind_index += 1;
    }
    let rest = &after_global[kind_prefix.len()..];
    proof {
        reveal(configured_prefix_remainder_model);
        reveal(literal_prefix_model);
        assert(literal_prefix_model(input@, global_prefix@));
        assert(literal_prefix_model(after_global@, kind_prefix@));
        assert(rest@ == after_global@.subrange(kind_prefix.len() as int, after_global.len() as int));
        assert(input@.subrange(global_prefix.len() as int, input.len() as int)
            .subrange(kind_prefix.len() as int, after_global.len() as int)
            == input@.subrange((global_prefix.len() + kind_prefix.len()) as int, input.len() as int));
    }
    Some(rest)
}
}

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
    let mut recognized = false;
    let mut result = None;
    for kind in KINDS {
        let prefix = match kind {
            ObjectKind::FullVolume | ObjectKind::IncrementalVolume => config.archive_prefix,
            ObjectKind::FullManifest | ObjectKind::IncrementalManifest => config.manifest_prefix,
            _ => config.signature_prefix,
        };
        //= docs/parsing-requirements.md#parse-name-002
        //= type=implementation
        //# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
        let Some(rest) = configured_prefix_remainder(input, config.global_prefix, prefix) else {
            continue;
        };
        for short in [false, true] {
            if let Some(rest) = rest.strip_prefix(stem(kind, short)) {
                recognized = true;
                if let Ok(parsed) = body(rest, kind, short, config.time_separator) {
                    //= docs/parsing-requirements.md#parse-name-001
                    //= type=implementation
                    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
                    result = merge_name_candidate(result, Some(parsed))?;
                }
            }
        }
    }
    //= docs/parsing-requirements.md#parse-name-001
    //= type=implementation
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    finalize_name_classification(recognized, result)
}

#[cfg(test)]
mod tests {
    use super::{split_dot_field, take_dot_field};

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-gen-002
    //= type=test
    //# A successful parser MUST report a consumed input range wholly contained within the supplied input.
    fn dot_field_splitting_preserves_empty_fields_and_exact_ranges() {
        let cases: &[(&[u8], &[&[u8]])] = &[
            (b"", &[b""]),
            (b"start.end.volume", &[b"start", b"end", b"volume"]),
            (b".partial..gzip.", &[b"", b"partial", b"", b"gzip", b""]),
        ];
        for (input, expected) in cases {
            let mut remaining = Some(*input);
            let mut actual = Vec::new();
            while let Some(field) = take_dot_field(&mut remaining) {
                actual.push(field);
            }
            assert_eq!(actual, *expected);
            assert_eq!(remaining, None);
        }

        let (field, rest) = split_dot_field(b"first.second");
        assert_eq!(field, b"first");
        assert_eq!(rest, Some(&b"second"[..]));
        let (field, rest) = split_dot_field(b"last");
        assert_eq!(field, b"last");
        assert_eq!(rest, None);
    }

    use super::{
        calendar_date_valid, calendar_day_valid, calendar_fields_to_unix_seconds,
        clock_fields_valid, compose_timestamp_fields, configured_prefix_remainder, decode_base36,
        decode_decimal, decode_long_interval, decode_long_timestamp_numeric_field,
        decode_long_trailer_suffix, decode_long_volume_number, decode_short_interval,
        decode_short_timestamp, decode_short_trailer, decode_short_volume_number,
        decode_zone_offset_token, finalize_name_classification, long_timestamp_field_bounds,
        long_timestamp_framing_valid, merge_name_candidate, parse_long_timestamp, stem,
        time_separator_valid, Envelope, LongTimestampField, NameError, ObjectKind, ObjectName,
        Timestamp,
    };

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn name_candidate_accumulator_preserves_none_and_equal_results_and_rejects_conflicts() {
        let original = ObjectName {
            kind: ObjectKind::FullVolume,
            start: Timestamp(10),
            end: Timestamp(10),
            volume: Some(1),
            envelope: Envelope::Plain,
            partial: false,
            short: false,
        };
        assert_eq!(merge_name_candidate(None, None), Ok(None));
        assert_eq!(
            merge_name_candidate(Some(original), None),
            Ok(Some(original))
        );
        assert_eq!(
            merge_name_candidate(None, Some(original)),
            Ok(Some(original))
        );
        assert_eq!(
            merge_name_candidate(Some(original), Some(original)),
            Ok(Some(original))
        );
        for different in [
            ObjectName {
                kind: ObjectKind::IncrementalVolume,
                ..original
            },
            ObjectName {
                start: Timestamp(11),
                ..original
            },
            ObjectName {
                end: Timestamp(11),
                ..original
            },
            ObjectName {
                volume: Some(2),
                ..original
            },
            ObjectName {
                envelope: Envelope::Gzip,
                ..original
            },
            ObjectName {
                partial: true,
                ..original
            },
            ObjectName {
                short: true,
                ..original
            },
        ] {
            assert_eq!(
                merge_name_candidate(Some(original), Some(different)),
                Err(NameError::Ambiguous),
                "must distinguish {different:?} from {original:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn name_finalizer_covers_candidate_recognition_state_matrix() {
        let candidate = ObjectName {
            kind: ObjectKind::FullVolume,
            start: Timestamp(10),
            end: Timestamp(10),
            volume: Some(1),
            envelope: Envelope::Plain,
            partial: false,
            short: false,
        };
        assert_eq!(finalize_name_classification(false, None), Ok(None));
        assert_eq!(
            finalize_name_classification(true, None),
            Err(NameError::Malformed)
        );
        assert_eq!(
            finalize_name_classification(false, Some(candidate)),
            Ok(Some(candidate))
        );
        assert_eq!(
            finalize_name_classification(true, Some(candidate)),
            Ok(Some(candidate))
        );
    }

    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    #[test]
    fn object_kind_stems_match_the_exact_long_and_short_tables() {
        for (kind, short, expected) in [
            (ObjectKind::FullVolume, false, b"duplicity-full." as &[u8]),
            (ObjectKind::IncrementalVolume, false, b"duplicity-inc."),
            (ObjectKind::FullManifest, false, b"duplicity-full."),
            (ObjectKind::IncrementalManifest, false, b"duplicity-inc."),
            (
                ObjectKind::FullSignatures,
                false,
                b"duplicity-full-signatures.",
            ),
            (
                ObjectKind::IncrementalSignatures,
                false,
                b"duplicity-new-signatures.",
            ),
            (ObjectKind::FullVolume, true, b"df."),
            (ObjectKind::IncrementalVolume, true, b"di."),
            (ObjectKind::FullManifest, true, b"df."),
            (ObjectKind::IncrementalManifest, true, b"di."),
            (ObjectKind::FullSignatures, true, b"dfs."),
            (ObjectKind::IncrementalSignatures, true, b"dns."),
        ] {
            assert_eq!(stem(kind, short), expected, "{kind:?} short={short}");
        }
    }

    //= docs/parsing-requirements.md#parse-name-002
    //= type=test
    //# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
    #[test]
    fn configured_prefix_remainder_matches_ordered_literal_bytes() {
        let prefixes: [&[u8]; 7] = [b"", b"a", b"ab", b"a+", b"[g]", b"\0", b"/"];
        for global in prefixes {
            for kind in prefixes {
                let mut exact = global.to_vec();
                exact.extend_from_slice(kind);
                exact.extend_from_slice(b"stem");
                let expected = exact.strip_prefix(global).unwrap().strip_prefix(kind);
                assert_eq!(
                    configured_prefix_remainder(&exact, global, kind),
                    expected,
                    "global={global:?} kind={kind:?}"
                );

                if !global.is_empty() {
                    let mut global_mismatch = exact.clone();
                    global_mismatch[0] ^= 0xff;
                    assert_eq!(
                        configured_prefix_remainder(&global_mismatch, global, kind),
                        None
                    );
                }

                if !kind.is_empty() {
                    let mut kind_mismatch = exact.clone();
                    kind_mismatch[global.len()] ^= 0xff;
                    assert_eq!(
                        configured_prefix_remainder(&kind_mismatch, global, kind),
                        None
                    );
                }

                for input in [
                    global,
                    &exact[..global.len()],
                    &exact[..global.len().saturating_add(kind.len()).saturating_sub(1)],
                ] {
                    let expected = input
                        .strip_prefix(global)
                        .and_then(|rest| rest.strip_prefix(kind));
                    assert_eq!(configured_prefix_remainder(input, global, kind), expected);
                }
            }
        }

        for global in 0..=u8::MAX {
            for kind in 0..=u8::MAX {
                let input = [global, kind, 0, b'/'];
                assert_eq!(
                    configured_prefix_remainder(&input, &[global], &[kind]),
                    Some(&input[2..])
                );
                let global_wrong = [global.wrapping_add(1), kind, 0, b'/'];
                assert_eq!(
                    configured_prefix_remainder(&global_wrong, &[global], &[kind]),
                    None
                );
                let kind_wrong = [global, kind.wrapping_add(1), 0, b'/'];
                assert_eq!(
                    configured_prefix_remainder(&kind_wrong, &[global], &[kind]),
                    None
                );
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
    fn validated_timestamp_components_compose_exact_utc_seconds() {
        for (fields, expected) in [
            ((1970, 1, 1, 0, 0, 0, 0), 0),
            ((1969, 12, 31, 23, 59, 59, 0), -1),
            ((2000, 2, 29, 0, 0, 0, 0), 951_782_400),
            ((2038, 1, 19, 3, 14, 7, 0), 2_147_483_647),
            ((1970, 1, 1, 23, 59, 0, 86_340), 0),
            ((1969, 12, 31, 0, 1, 0, -86_340), 0),
        ] {
            let (year, month, day, hour, minute, second, offset) = fields;
            assert_eq!(
                compose_timestamp_fields(year, month, day, hour, minute, second, offset),
                Some(Timestamp(expected)),
                "{fields:?}"
            );
        }

        for fields in [
            (1900, 1, 1, 0, 0, 0, 0),
            (2100, 1, 1, 0, 0, 0, 0),
            (2000, 0, 1, 0, 0, 0, 0),
            (2001, 2, 29, 0, 0, 0, 0),
            (2000, 1, 1, 24, 0, 0, 0),
            (2000, 1, 1, 0, 60, 0, 0),
            (2000, 1, 1, 0, 0, 60, 0),
            (2000, 1, 1, 0, 0, 0, 86_341),
            (2000, 1, 1, 0, 0, 0, -86_341),
        ] {
            let (year, month, day, hour, minute, second, offset) = fields;
            assert_eq!(
                compose_timestamp_fields(year, month, day, hour, minute, second, offset),
                None,
                "{fields:?}"
            );
        }
    }

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
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_interval_decoder_couples_kind_timestamp_and_order_policy() {
        use super::ObjectKind;
        let full_kinds = [
            ObjectKind::FullVolume,
            ObjectKind::FullManifest,
            ObjectKind::FullSignatures,
        ];
        let incremental_kinds = [
            ObjectKind::IncrementalVolume,
            ObjectKind::IncrementalManifest,
            ObjectKind::IncrementalSignatures,
        ];

        for kind in full_kinds {
            assert_eq!(
                decode_short_interval(kind, b"3w5e11264sgsf", None),
                Some((Timestamp(u64::MAX as i128), Timestamp(u64::MAX as i128)))
            );
            assert_eq!(decode_short_interval(kind, b"1", Some(b"2")), None);
        }
        for kind in incremental_kinds {
            for (start, end, expected) in [
                (b"0" as &[u8], b"1" as &[u8], Some((0, 1))),
                (
                    b"3w5e11264sgse",
                    b"3w5e11264sgsf",
                    Some((u64::MAX - 1, u64::MAX)),
                ),
                (b"1", b"1", None),
                (b"2", b"1", None),
            ] {
                assert_eq!(
                    decode_short_interval(kind, start, Some(end)),
                    expected.map(|(start, end)| (Timestamp(start as i128), Timestamp(end as i128))),
                    "{kind:?} {start:?}..{end:?}"
                );
            }
            assert_eq!(decode_short_interval(kind, b"1", None), None);
            assert_eq!(decode_short_interval(kind, b"1", Some(b"A")), None);
            assert_eq!(
                decode_short_interval(kind, b"1", Some(b"3w5e11264sgsg")),
                None
            );
        }
        for kind in full_kinds.into_iter().chain(incremental_kinds) {
            assert_eq!(decode_short_interval(kind, b"", None), None);
            assert_eq!(decode_short_interval(kind, b"A", None), None);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_interval_decoder_couples_kind_marker_timestamp_and_order_policy() {
        let full_kinds = [
            ObjectKind::FullVolume,
            ObjectKind::FullManifest,
            ObjectKind::FullSignatures,
        ];
        let incremental_kinds = [
            ObjectKind::IncrementalVolume,
            ObjectKind::IncrementalManifest,
            ObjectKind::IncrementalSignatures,
        ];
        let canonical_start = b"20231114T221320Z";
        let canonical_end = b"20231115T221320Z";
        let legacy_start = b"2023-11-14T22:13:20Z";
        let legacy_end = b"2023-11-15T22:13:20Z";
        let expected_start = parse_long_timestamp(canonical_start, b':').unwrap();
        let expected_end = parse_long_timestamp(canonical_end, b':').unwrap();

        for kind in full_kinds {
            for (start, separator) in [
                (canonical_start as &[u8], b':'),
                (legacy_start as &[u8], b':'),
            ] {
                let timestamp = parse_long_timestamp(start, separator).unwrap();
                assert_eq!(
                    decode_long_interval(kind, start, separator, None, None),
                    Some((timestamp, timestamp)),
                    "{kind:?} {start:?}"
                );
            }
            assert_eq!(
                decode_long_interval(kind, canonical_start, b':', Some(b"to"), None),
                None
            );
        }

        for kind in incremental_kinds {
            for (start, end, separator, expected) in [
                (
                    canonical_start as &[u8],
                    canonical_end as &[u8],
                    b':',
                    Some((expected_start, expected_end)),
                ),
                (
                    legacy_start as &[u8],
                    legacy_end as &[u8],
                    b':',
                    Some((expected_start, expected_end)),
                ),
                (
                    b"19010101T000000Z" as &[u8],
                    b"19010102T000000Z" as &[u8],
                    b':',
                    Some((Timestamp(-2_177_452_800), Timestamp(-2_177_366_400))),
                ),
                (
                    b"20991230T000000Z" as &[u8],
                    b"20991231T000000Z" as &[u8],
                    b':',
                    Some((Timestamp(4_102_272_000), Timestamp(4_102_358_400))),
                ),
            ] {
                assert_eq!(
                    decode_long_interval(kind, start, separator, Some(b"to"), Some(end)),
                    expected,
                    "{kind:?} {start:?}..{end:?}"
                );
            }
            for (marker, end) in [
                (None, Some(canonical_end as &[u8])),
                (Some(b"TO" as &[u8]), Some(canonical_end as &[u8])),
                (Some(b"to" as &[u8]), None),
                (Some(b"to" as &[u8]), Some(b"bad" as &[u8])),
                (Some(b"to" as &[u8]), Some(canonical_start as &[u8])),
                (Some(b"to" as &[u8]), Some(b"20231113T221320Z" as &[u8])),
            ] {
                assert_eq!(
                    decode_long_interval(kind, canonical_start, b':', marker, end),
                    None,
                    "{kind:?} {marker:?} {end:?}"
                );
            }
        }
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
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn short_volume_number_exhaustively_decodes_one_and_two_digits() {
        for first in 0..=u8::MAX {
            assert_eq!(
                decode_short_volume_number(&[first]),
                decode_base36(&[first]).filter(|number| *number > 0),
                "byte {first:#04x}"
            );
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
                let encoded = [digit(high), digit(low)];
                assert_eq!(
                    decode_short_volume_number(&encoded),
                    Some(u64::from(high) * 36 + u64::from(low)).filter(|number| *number > 0),
                    "{encoded:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn short_volume_number_rejects_empty_zero_invalid_and_overflow() {
        assert_eq!(decode_short_volume_number(b""), None);
        for zero in [b"0" as &[u8], b"00", b"000"] {
            assert_eq!(decode_short_volume_number(zero), None);
        }
        for invalid in [b"A" as &[u8], b"-", b"_", b"0A", b"a_"] {
            assert_eq!(decode_short_volume_number(invalid), None, "{invalid:?}");
        }
        assert_eq!(
            decode_short_volume_number(&encode_base36(u64::MAX as u128 + 1)),
            None
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_volume_number_exhaustively_decodes_positive_decimal_tail() {
        for byte in 0..=u8::MAX {
            let mut field = b"vol".to_vec();
            field.push(byte);
            let expected = decode_decimal(&[byte]).filter(|number| *number > 0);
            assert_eq!(decode_long_volume_number(&field), expected, "{field:?}");
        }
        for high in 0..=9u8 {
            for low in 0..=9u8 {
                let field = [b'v', b'o', b'l', b'0' + high, b'0' + low];
                let expected =
                    Some(u64::from(high) * 10 + u64::from(low)).filter(|number| *number > 0);
                assert_eq!(decode_long_volume_number(&field), expected, "{field:?}");
            }
        }
        for value in [1, 9, 10, 99, u32::MAX as u64, u64::MAX] {
            let field = format!("vol{value}");
            assert_eq!(decode_long_volume_number(field.as_bytes()), Some(value));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_volume_number_rejects_bad_prefix_zero_invalid_and_overflow() {
        for malformed in [
            b"" as &[u8],
            b"v",
            b"vo",
            b"vol",
            b"vol0",
            b"vol00",
            b"Vol1",
            b"VOL1",
            b"voL1",
            b"vol-1",
            b"vol1a",
            b"vol_1",
        ] {
            assert_eq!(decode_long_volume_number(malformed), None, "{malformed:?}");
        }
        let overflow = format!("vol{}", u64::MAX as u128 + 1);
        assert_eq!(decode_long_volume_number(overflow.as_bytes()), None);
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_trailer_policy_systematically_checks_kinds_tokens_and_extra_fields() {
        let kinds = [
            ObjectKind::FullVolume,
            ObjectKind::IncrementalVolume,
            ObjectKind::FullManifest,
            ObjectKind::IncrementalManifest,
            ObjectKind::FullSignatures,
            ObjectKind::IncrementalSignatures,
        ];
        let token_choices: [Option<&[u8]>; 6] = [
            None,
            Some(b"p"),
            Some(b"z"),
            Some(b"g"),
            Some(b"x"),
            Some(b""),
        ];
        let trailer_choices: [&[u8]; 4] = [b"dt", b"m", b"st", b"x"];
        for kind in kinds {
            let expected_trailer: &[u8] = match kind {
                ObjectKind::FullVolume | ObjectKind::IncrementalVolume => b"dt",
                ObjectKind::FullManifest | ObjectKind::IncrementalManifest => b"m",
                ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => b"st",
            };
            for trailer in trailer_choices {
                for first in token_choices {
                    for second in token_choices {
                        for third in token_choices {
                            let mut expected = None;
                            if trailer == expected_trailer && third.is_none() {
                                expected = match (first, second) {
                                    (None, None) => Some((Envelope::Plain, false)),
                                    (Some(b"p"), None) => Some((Envelope::Plain, true)),
                                    (Some(b"z"), None) => Some((Envelope::Gzip, false)),
                                    (Some(b"g"), None) => Some((Envelope::OpenPgp, false)),
                                    (Some(b"p"), Some(b"z")) => Some((Envelope::Gzip, true)),
                                    (Some(b"p"), Some(b"g")) => Some((Envelope::OpenPgp, true)),
                                    _ => None,
                                };
                            }
                            assert_eq!(
                                decode_short_trailer(kind, trailer, first, second, third),
                                expected,
                                "kind={kind:?}, trailer={trailer:?}, fields={first:?}/{second:?}/{third:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_trailer_policy_systematically_checks_kinds_suffixes_and_extra_fields() {
        let kinds = [
            ObjectKind::FullVolume,
            ObjectKind::IncrementalVolume,
            ObjectKind::FullManifest,
            ObjectKind::IncrementalManifest,
            ObjectKind::FullSignatures,
            ObjectKind::IncrementalSignatures,
        ];
        let token_choices: [Option<&[u8]>; 7] = [
            None,
            Some(b"part"),
            Some(b"gz"),
            Some(b"gpg"),
            Some(b"p7m"),
            Some(b"bad"),
            Some(b""),
        ];
        let trailer_choices: [&[u8]; 4] = [b"difftar", b"manifest", b"sigtar", b"wrong"];
        for kind in kinds {
            let expected_trailer: &[u8] = match kind {
                ObjectKind::FullVolume | ObjectKind::IncrementalVolume => b"difftar",
                ObjectKind::FullManifest | ObjectKind::IncrementalManifest => b"manifest",
                ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => b"sigtar",
            };
            for trailer in trailer_choices {
                for first in token_choices {
                    for second in token_choices {
                        for third in token_choices {
                            let mut expected = None;
                            if trailer == expected_trailer && third.is_none() {
                                expected = match (first, second) {
                                    (None, None) => Some((Envelope::Plain, false)),
                                    (Some(b"part"), None) => Some((Envelope::Plain, true)),
                                    (Some(b"gz"), None) => Some((Envelope::Gzip, false)),
                                    (Some(b"gpg"), None) => Some((Envelope::OpenPgp, false)),
                                    (Some(b"p7m"), None) => Some((Envelope::Pkcs7, false)),
                                    (Some(b"part"), Some(b"gz")) => Some((Envelope::Gzip, true)),
                                    (Some(b"part"), Some(b"gpg")) => {
                                        Some((Envelope::OpenPgp, true))
                                    }
                                    (Some(b"part"), Some(b"p7m")) => Some((Envelope::Pkcs7, true)),
                                    _ => None,
                                };
                            }
                            assert_eq!(
                                decode_long_trailer_suffix(kind, trailer, first, second, third),
                                expected,
                                "kind={kind:?}, trailer={trailer:?}, fields={first:?}/{second:?}/{third:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    #[test]
    fn short_timestamp_decodes_base36_epoch_seconds_exactly() {
        for byte in 0..=u8::MAX {
            let expected = decode_base36(&[byte]).map(|seconds| Timestamp(seconds as i128));
            assert_eq!(
                decode_short_timestamp(&[byte]),
                expected,
                "byte {byte:#04x}"
            );
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
                let text = [digit(high), digit(low)];
                let seconds = u64::from(high) * 36 + u64::from(low);
                assert_eq!(
                    decode_short_timestamp(&text),
                    Some(Timestamp(seconds as i128))
                );
            }
        }
        for seconds in [0, 1, 35, 36, 1295, u64::MAX as u128] {
            let encoded = encode_base36(seconds);
            assert_eq!(
                decode_short_timestamp(&encoded),
                Some(Timestamp(seconds as i128)),
                "{seconds}"
            );
        }
    }

    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    #[test]
    fn short_timestamp_rejects_empty_invalid_and_overflowing_values() {
        assert_eq!(decode_short_timestamp(b""), None);
        for invalid in [b"A" as &[u8], b"-", b"_", b"0A", b"a_"] {
            assert_eq!(decode_short_timestamp(invalid), None, "{invalid:?}");
        }
        assert_eq!(
            decode_short_timestamp(&encode_base36(u64::MAX as u128 + 1)),
            None
        );
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
