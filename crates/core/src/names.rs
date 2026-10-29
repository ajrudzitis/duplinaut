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
    ensures body_field_split_model(
        optional_body_field_model(*old(remaining)),
        optional_body_field_model(field),
        optional_body_field_model(*final(remaining)),
    ) && match *old(remaining) {
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
    let _input = *remaining;
    let field = match remaining.take() {
        Some(input) => {
            let (field, rest) = split_dot_field(input);
            proof {
                reveal(optional_body_field_model);
                reveal(body_field_split_model);
                reveal(dot_field_boundary_model);
                assert(body_field_split_model(
                    Some(input@),
                    Some(field@),
                    match rest {
                        Some(bytes) => Some(bytes@),
                        None => None,
                    },
                ));
            }
            *remaining = rest;
            Some(field)
        }
        None => {
            proof {
                reveal(optional_body_field_model);
                reveal(body_field_split_model);
                assert(body_field_split_model(None, None, None));
            }
            None
        }
    };
    proof {
        reveal(optional_body_field_model);
        reveal(body_field_split_model);
        assert(body_field_split_model(
            optional_body_field_model(_input),
            optional_body_field_model(field),
            optional_body_field_model(*remaining),
        ));
    }
    field
}

closed spec fn optional_body_field_model(input: Option<&[u8]>) -> Option<Seq<u8>> {
    match input {
        Some(bytes) => Some(bytes@),
        None => None,
    }
}

closed spec fn body_field_split_model(
    input: Option<Seq<u8>>,
    field: Option<Seq<u8>>,
    remaining: Option<Seq<u8>>,
) -> bool {
    match (input, field) {
        (None, None) => remaining.is_none(),
        (Some(bytes), Some(value)) => {
            value.len() <= bytes.len()
                && value == bytes.subrange(0, value.len() as int)
                && forall|index: int| 0 <= index < value.len()
                    ==> value[index] != b'.'
                && match remaining {
                    None => value.len() == bytes.len(),
                    Some(rest) => {
                        value.len() < bytes.len()
                            && bytes[value.len() as int] == b'.'
                            && rest == bytes.subrange(value.len() as int + 1, bytes.len() as int)
                    }
                }
        }
        _ => false,
    }
}

closed spec fn body_suffix_fields_model(
    input: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    remaining: Option<Seq<u8>>,
) -> bool {
    exists|after_first: Option<Seq<u8>>|
        body_field_split_model(input, first, after_first)
            && exists|after_second: Option<Seq<u8>>|
                body_field_split_model(after_first, second, after_second)
                    && exists|after_third: Option<Seq<u8>>|
                        body_field_split_model(after_second, third, after_third)
                            && body_field_split_model(after_third, extra, remaining)
}

type BodySuffixFields<'a> = (
    Option<&'a [u8]>,
    Option<&'a [u8]>,
    Option<&'a [u8]>,
    Option<&'a [u8]>,
);

/// Consume three optional suffix fields and one excess-field probe.
///
/// The caller decides whether the third suffix is admissible for its object
/// profile. `extra` identifies the first field beyond those three slots.
fn take_body_suffix_fields<'a>(
    remaining: &mut Option<&'a [u8]>,
) -> (suffixes: BodySuffixFields<'a>)
    ensures body_suffix_fields_model(
        optional_body_field_model(*old(remaining)),
        optional_body_field_model(suffixes.0),
        optional_body_field_model(suffixes.1),
        optional_body_field_model(suffixes.2),
        optional_body_field_model(suffixes.3),
        optional_body_field_model(*final(remaining)),
    ),
{
    let _input = *remaining;
    let first = take_dot_field(remaining);
    let _after_first = *remaining;
    let second = take_dot_field(remaining);
    let _after_second = *remaining;
    let third = take_dot_field(remaining);
    let _after_third = *remaining;
    let extra = take_dot_field(remaining);
    proof {
        reveal(optional_body_field_model);
        reveal(body_field_split_model);
        reveal(dot_field_boundary_model);
        assert(body_field_split_model(
            optional_body_field_model(_input),
            optional_body_field_model(first),
            optional_body_field_model(_after_first),
        ));
    }
    proof {
        reveal(optional_body_field_model);
        reveal(body_field_split_model);
        reveal(dot_field_boundary_model);
        assert(body_field_split_model(
            optional_body_field_model(_after_first),
            optional_body_field_model(second),
            optional_body_field_model(_after_second),
        ));
    }
    proof {
        reveal(optional_body_field_model);
        reveal(body_field_split_model);
        reveal(dot_field_boundary_model);
        assert(body_field_split_model(
            optional_body_field_model(_after_second),
            optional_body_field_model(third),
            optional_body_field_model(_after_third),
        ));
    }
    proof {
        reveal(optional_body_field_model);
        reveal(body_field_split_model);
        reveal(dot_field_boundary_model);
        assert(body_field_split_model(
            optional_body_field_model(_after_third),
            optional_body_field_model(extra),
            optional_body_field_model(*remaining),
        ));
    }
    proof {
        reveal(optional_body_field_model);
        reveal(body_suffix_fields_model);
        assert(body_suffix_fields_model(
            optional_body_field_model(_input),
            optional_body_field_model(first),
            optional_body_field_model(second),
            optional_body_field_model(third),
            optional_body_field_model(extra),
            optional_body_field_model(*remaining),
        ));
    }
    (first, second, third, extra)
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

/// Days from the calendar model's 1901 origin to the Unix epoch.
pub const UNIX_EPOCH_DAYS_FROM_1901: i128 = 25_202;

/// Derive the epoch offset by summing the supported Gregorian year recurrence.
pub proof fn unix_epoch_year_offset_is_exact()
    ensures days_before_year_model(1970) == UNIX_EPOCH_DAYS_FROM_1901 as int,
{
    assert(days_before_year_model(1970) == 25_202int) by (compute);
}

/// Check the Gregorian century exceptions used by the year recurrence.
pub proof fn gregorian_century_leap_policy()
    ensures !year_is_leap(1900) && year_is_leap(2000) && !year_is_leap(2100),
{
    assert(!year_is_leap(1900) && year_is_leap(2000) && !year_is_leap(2100)) by (compute);
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
        + day - 1 - UNIX_EPOCH_DAYS_FROM_1901;
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
    proof { unix_epoch_year_offset_is_exact(); }
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
    let days_since_epoch = year_days as i128 + month_days + day as i128 - 1
        - UNIX_EPOCH_DAYS_FROM_1901;
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

closed spec fn short_full_volume_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    volume: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, volume_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_volume: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(volume_field), after_volume),
            body_field_split_model(after_volume, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            short_full_volume_fields_model(start_field, volume_field, trailer, first, second)
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(volume_field), after_volume)
            && body_field_split_model(after_volume, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_full_volume_fields_model(start_field, volume_field, trailer, first, second)
                == Some((start, end, volume, envelope, partial))
}

closed spec fn long_full_volume_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    volume_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, int, Envelope, bool)> {
    match (
        long_timestamp_result_model(start_field, separator),
        long_volume_number_model(volume_field),
        long_trailer_suffix_model(
            ObjectKind::FullVolume,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some(timestamp), Some(volume), Some((envelope, partial))) => {
            Some((timestamp, timestamp, volume, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_incremental_volume_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    marker: Seq<u8>,
    end_field: Seq<u8>,
    volume_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, int, Envelope, bool)> {
    match (
        long_interval_model(
            ObjectKind::IncrementalVolume,
            start_field,
            separator,
            Some(marker),
            Some(end_field),
        ),
        long_volume_number_model(volume_field),
        long_trailer_suffix_model(
            ObjectKind::IncrementalVolume,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some((start, end)), Some(volume), Some((envelope, partial))) => {
            Some((start, end, volume, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_incremental_volume_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    volume: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, marker: Seq<u8>, end_field: Seq<u8>,
        volume_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_marker: Option<Seq<u8>>,
        after_end: Option<Seq<u8>>, after_volume: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(marker), after_marker),
            body_field_split_model(after_marker, Some(end_field), after_end),
            body_field_split_model(after_end, Some(volume_field), after_volume),
            body_field_split_model(after_volume, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_incremental_volume_fields_model(
                start_field, separator, marker, end_field, volume_field, trailer,
                first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(marker), after_marker)
            && body_field_split_model(after_marker, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(volume_field), after_volume)
            && body_field_split_model(after_volume, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_incremental_volume_fields_model(
                start_field, separator, marker, end_field, volume_field, trailer,
                first, second, third,
            ) == Some((start, end, volume, envelope, partial))
}

closed spec fn long_full_manifest_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        long_timestamp_result_model(start_field, separator),
        long_trailer_suffix_model(
            ObjectKind::FullManifest,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some(timestamp), Some((envelope, partial))) => {
            Some((timestamp, timestamp, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_full_manifest_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_trailer: Option<Seq<u8>>,
        extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_full_manifest_fields_model(
                start_field, separator, trailer, first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_full_manifest_fields_model(
                start_field, separator, trailer, first, second, third,
            ) == Some((start, end, envelope, partial))
}

closed spec fn long_full_signatures_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        long_timestamp_result_model(start_field, separator),
        long_trailer_suffix_model(
            ObjectKind::FullSignatures,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some(timestamp), Some((envelope, partial))) => {
            Some((timestamp, timestamp, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_full_signatures_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_trailer: Option<Seq<u8>>,
        extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_full_signatures_fields_model(
                start_field, separator, trailer, first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_full_signatures_fields_model(
                start_field, separator, trailer, first, second, third,
            ) == Some((start, end, envelope, partial))
}

closed spec fn long_full_volume_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    volume: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, volume_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_volume: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(volume_field), after_volume),
            body_field_split_model(after_volume, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_full_volume_fields_model(
                start_field, separator, volume_field, trailer, first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(volume_field), after_volume)
            && body_field_split_model(after_volume, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_full_volume_fields_model(
                start_field, separator, volume_field, trailer, first, second, third,
            ) == Some((start, end, volume, envelope, partial))
}

closed spec fn long_incremental_manifest_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    marker: Seq<u8>,
    end_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        long_interval_model(
            ObjectKind::IncrementalManifest,
            start_field,
            separator,
            Some(marker),
            Some(end_field),
        ),
        long_trailer_suffix_model(
            ObjectKind::IncrementalManifest,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some((start, end)), Some((envelope, partial))) => {
            Some((start, end, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_incremental_manifest_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, marker: Seq<u8>, end_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_marker: Option<Seq<u8>>,
        after_end: Option<Seq<u8>>, after_trailer: Option<Seq<u8>>,
        extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(marker), after_marker),
            body_field_split_model(after_marker, Some(end_field), after_end),
            body_field_split_model(after_end, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_incremental_manifest_fields_model(
                start_field, separator, marker, end_field, trailer, first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(marker), after_marker)
            && body_field_split_model(after_marker, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_incremental_manifest_fields_model(
                start_field, separator, marker, end_field, trailer, first, second, third,
            ) == Some((start, end, envelope, partial))
}

closed spec fn long_incremental_signatures_fields_model(
    start_field: Seq<u8>,
    separator: u8,
    marker: Seq<u8>,
    end_field: Seq<u8>,
    trailer: Seq<u8>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
) -> Option<(int, int, Envelope, bool)> {
    match (
        long_interval_model(
            ObjectKind::IncrementalSignatures,
            start_field,
            separator,
            Some(marker),
            Some(end_field),
        ),
        long_trailer_suffix_model(
            ObjectKind::IncrementalSignatures,
            trailer,
            first,
            second,
            third,
        ),
    ) {
        (Some((start, end)), Some((envelope, partial))) => {
            Some((start, end, envelope, partial))
        }
        _ => None,
    }
}

closed spec fn long_incremental_signatures_raw_body_model(
    input: Seq<u8>,
    separator: u8,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, marker: Seq<u8>, end_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>, third: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_marker: Option<Seq<u8>>,
        after_end: Option<Seq<u8>>, after_trailer: Option<Seq<u8>>,
        extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(marker), after_marker),
            body_field_split_model(after_marker, Some(end_field), after_end),
            body_field_split_model(after_end, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            long_incremental_signatures_fields_model(
                start_field, separator, marker, end_field, trailer, first, second, third,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(marker), after_marker)
            && body_field_split_model(after_marker, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && extra.is_none()
            && long_incremental_signatures_fields_model(
                start_field, separator, marker, end_field, trailer, first, second, third,
            ) == Some((start, end, envelope, partial))
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

closed spec fn short_full_manifest_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, trailer: Seq<u8>, first: Option<Seq<u8>>,
        second: Option<Seq<u8>>, after_start: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_full_manifest_fields_model(start_field, trailer, first, second)
                == Some((start, end, envelope, partial))
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

closed spec fn short_full_signatures_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, trailer: Seq<u8>, first: Option<Seq<u8>>,
        second: Option<Seq<u8>>, after_start: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_full_signatures_fields_model(start_field, trailer, first, second)
                == Some((start, end, envelope, partial))
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

closed spec fn short_incremental_signatures_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, end_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_end: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(end_field), after_end),
            body_field_split_model(after_end, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            short_incremental_signatures_fields_model(
                start_field, end_field, trailer, first, second,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_incremental_signatures_fields_model(
                start_field, end_field, trailer, first, second,
            ) == Some((start, end, envelope, partial))
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

closed spec fn short_incremental_manifest_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, end_field: Seq<u8>, trailer: Seq<u8>,
        first: Option<Seq<u8>>, second: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_end: Option<Seq<u8>>,
        after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(end_field), after_end),
            body_field_split_model(after_end, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            short_incremental_manifest_fields_model(
                start_field, end_field, trailer, first, second,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_incremental_manifest_fields_model(
                start_field, end_field, trailer, first, second,
            ) == Some((start, end, envelope, partial))
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

closed spec fn short_incremental_volume_raw_body_model(
    input: Seq<u8>,
    start: int,
    end: int,
    volume: int,
    envelope: Envelope,
    partial: bool,
) -> bool {
    exists|start_field: Seq<u8>, end_field: Seq<u8>, volume_field: Seq<u8>,
        trailer: Seq<u8>, first: Option<Seq<u8>>, second: Option<Seq<u8>>,
        after_start: Option<Seq<u8>>, after_end: Option<Seq<u8>>,
        after_volume: Option<Seq<u8>>, after_trailer: Option<Seq<u8>>,
        third: Option<Seq<u8>>, extra: Option<Seq<u8>>|
        #![trigger
            body_field_split_model(Some(input), Some(start_field), after_start),
            body_field_split_model(after_start, Some(end_field), after_end),
            body_field_split_model(after_end, Some(volume_field), after_volume),
            body_field_split_model(after_volume, Some(trailer), after_trailer),
            body_suffix_fields_model(after_trailer, first, second, third, extra, None),
            short_incremental_volume_fields_model(
                start_field, end_field, volume_field, trailer, first, second,
            )
        ]
        body_field_split_model(Some(input), Some(start_field), after_start)
            && body_field_split_model(after_start, Some(end_field), after_end)
            && body_field_split_model(after_end, Some(volume_field), after_volume)
            && body_field_split_model(after_volume, Some(trailer), after_trailer)
            && body_suffix_fields_model(
                after_trailer, first, second, third, extra, None,
            )
            && third.is_none()
            && extra.is_none()
            && short_incremental_volume_fields_model(
                start_field, end_field, volume_field, trailer, first, second,
            ) == Some((start, end, volume, envelope, partial))
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

verus! {
#[derive(Clone, Copy, Debug)]
pub struct NameConfig<'a> {
    pub global_prefix: &'a [u8],
    pub archive_prefix: &'a [u8],
    pub manifest_prefix: &'a [u8],
    pub signature_prefix: &'a [u8],
    pub time_separator: u8,
    pub max_name_bytes: usize,
}
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

//= docs/parsing-requirements.md#parse-name-002
//= type=implementation
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem. Volume kinds MUST select the archive prefix, manifest kinds MUST select the manifest prefix, and signature kinds MUST select the signature prefix.
verus! {
closed spec fn name_candidate_table_model() -> Seq<(ObjectKind, bool)> {
    seq![
        (ObjectKind::FullVolume, false),
        (ObjectKind::FullVolume, true),
        (ObjectKind::IncrementalVolume, false),
        (ObjectKind::IncrementalVolume, true),
        (ObjectKind::FullManifest, false),
        (ObjectKind::FullManifest, true),
        (ObjectKind::IncrementalManifest, false),
        (ObjectKind::IncrementalManifest, true),
        (ObjectKind::FullSignatures, false),
        (ObjectKind::FullSignatures, true),
        (ObjectKind::IncrementalSignatures, false),
        (ObjectKind::IncrementalSignatures, true),
    ]
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn name_candidate_table() -> (candidates: [(ObjectKind, bool); 12])
    ensures candidates@ == name_candidate_table_model(),
{
    [
        (ObjectKind::FullVolume, false),
        (ObjectKind::FullVolume, true),
        (ObjectKind::IncrementalVolume, false),
        (ObjectKind::IncrementalVolume, true),
        (ObjectKind::FullManifest, false),
        (ObjectKind::FullManifest, true),
        (ObjectKind::IncrementalManifest, false),
        (ObjectKind::IncrementalManifest, true),
        (ObjectKind::FullSignatures, false),
        (ObjectKind::FullSignatures, true),
        (ObjectKind::IncrementalSignatures, false),
        (ObjectKind::IncrementalSignatures, true),
    ]
}

closed spec fn kind_specific_prefix_model(
    kind: ObjectKind,
    archive: Seq<u8>,
    manifest: Seq<u8>,
    signatures: Seq<u8>,
) -> Seq<u8> {
    match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => archive,
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => manifest,
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => signatures,
    }
}

fn kind_specific_prefix<'a>(
    kind: ObjectKind,
    archive: &'a [u8],
    manifest: &'a [u8],
    signatures: &'a [u8],
) -> (prefix: &'a [u8])
    ensures prefix@ == kind_specific_prefix_model(kind, archive@, manifest@, signatures@),
{
    match kind {
        ObjectKind::FullVolume | ObjectKind::IncrementalVolume => archive,
        ObjectKind::FullManifest | ObjectKind::IncrementalManifest => manifest,
        ObjectKind::FullSignatures | ObjectKind::IncrementalSignatures => signatures,
    }
}

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

closed spec fn stem_remainder_model(input: Seq<u8>, prefix: Seq<u8>) -> Option<Seq<u8>> {
    if prefix.len() <= input.len()
        && input.subrange(0, prefix.len() as int) == prefix
    {
        Some(input.subrange(prefix.len() as int, input.len() as int))
    } else {
        None
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn stem_remainder<'a>(input: &'a [u8], prefix: &[u8]) -> (result: Option<&'a [u8]>)
    ensures match result {
        Some(rest) => prefix.len() <= input.len()
            && prefix@ == input@.subrange(0, prefix.len() as int)
            && rest@ == input@.subrange(prefix.len() as int, input@.len() as int)
            && Some(rest@) == stem_remainder_model(input@, prefix@),
        None => stem_remainder_model(input@, prefix@).is_none(),
    },
{
    if prefix.len() > input.len() {
        proof {
            reveal(stem_remainder_model);
            assert(prefix.len() > input.len());
        }
        return None;
    }

    let mut matched = 0usize;
    while matched < prefix.len()
        invariant matched <= prefix.len(),
            prefix.len() <= input.len(),
            forall|index: int| 0 <= index < matched as int
                ==> input@[index] == prefix@[index],
        decreases prefix.len() - matched,
    {
        if input[matched] != prefix[matched] {
            proof {
                reveal(stem_remainder_model);
                assert(input@[matched as int] != prefix@[matched as int]);
                assert(input@.subrange(0, prefix.len() as int) != prefix@);
            }
            return None;
        }
        matched += 1;
    }

    proof {
        reveal(stem_remainder_model);
        assert(matched == prefix.len());
        assert(input@.subrange(0, prefix.len() as int) == prefix@);
    }
    Some(&input[prefix.len()..])
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
        Err(error) => error == NameError::Malformed && short_full_volume_fields_model(
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
    } && match short_full_volume_fields_model(
        start_field@,
        volume_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
    ) {
        Some((start, end, volume, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::FullVolume,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: Some(volume as u64),
            envelope,
            partial,
            short: true,
        }),
        None => result == Err(NameError::Malformed),
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
        Err(error) => error == NameError::Malformed && short_full_manifest_fields_model(
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
    }
        && match short_full_manifest_fields_model(
            start_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
        ) {
            Some((start, end, envelope, partial)) => result == Ok(ObjectName {
                kind: ObjectKind::FullManifest,
                start: Timestamp(start as i128),
                end: Timestamp(end as i128),
                volume: None,
                envelope,
                partial,
                short: true,
            }),
            None => result == Err(NameError::Malformed),
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
        Err(error) => error == NameError::Malformed && short_full_signatures_fields_model(
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
    } && match short_full_signatures_fields_model(
        start_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::FullSignatures,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: true,
        }),
        None => result == Err(NameError::Malformed),
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
        Err(error) => error == NameError::Malformed && short_incremental_signatures_fields_model(
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
    } && match short_incremental_signatures_fields_model(
        start_field@,
        end_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalSignatures,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: true,
        }),
        None => result == Err(NameError::Malformed),
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
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
verus! {
fn long_full_manifest_body_fields(
    start_field: &[u8],
    separator: u8,
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => (error == NameError::Malformed
            || (error == NameError::Configuration
                && !time_separator_valid_model(separator)))
            && long_full_manifest_fields_model(
            start_field@,
            separator,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_full_manifest_fields_model(
            start_field@,
            separator,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::FullManifest
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_full_manifest_fields_model(
        start_field@,
        separator,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::FullManifest,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: false,
        }),
        None => true,
    },
{
    let start = parse_long_timestamp(start_field, separator)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::FullManifest,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullManifest,
        start,
        end: start,
        volume: None,
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_full_manifest_fields_model);
        assert(long_timestamp_result_model(start_field@, separator) == Some(start.0 as int));
        assert(long_trailer_suffix_model(
            ObjectKind::FullManifest,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
verus! {
fn long_full_signatures_body_fields(
    start_field: &[u8],
    separator: u8,
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == (if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        })
            && long_full_signatures_fields_model(
            start_field@,
            separator,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_full_signatures_fields_model(
            start_field@,
            separator,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::FullSignatures
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_full_signatures_fields_model(
        start_field@,
        separator,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::FullSignatures,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: false,
        }),
        None => result == Err(if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        }),
    },
{
    let start = parse_long_timestamp(start_field, separator)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::FullSignatures,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullSignatures,
        start,
        end: start,
        volume: None,
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_full_signatures_fields_model);
        assert(long_timestamp_result_model(start_field@, separator) == Some(start.0 as int));
        assert(long_trailer_suffix_model(
            ObjectKind::FullSignatures,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
verus! {
#[allow(clippy::too_many_arguments)] // Long-name fields map directly to the parsed body.
fn long_incremental_manifest_body_fields(
    start_field: &[u8],
    separator: u8,
    marker: &[u8],
    end_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == (if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        })
            && long_incremental_manifest_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_incremental_manifest_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalManifest
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_incremental_manifest_fields_model(
        start_field@,
        separator,
        marker@,
        end_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalManifest,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: false,
        }),
        None => result == Err(if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        }),
    },
{
    if !time_separator_valid(separator) {
        return Err(NameError::Configuration);
    }
    let (start, end) = decode_long_interval(
        ObjectKind::IncrementalManifest,
        start_field,
        separator,
        Some(marker),
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::IncrementalManifest,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalManifest,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_incremental_manifest_fields_model);
        assert(long_interval_model(
            ObjectKind::IncrementalManifest,
            start_field@,
            separator,
            Some(marker@),
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(long_trailer_suffix_model(
            ObjectKind::IncrementalManifest,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
verus! {
#[allow(clippy::too_many_arguments)] // Long-name fields map directly to the parsed body.
fn long_incremental_signatures_body_fields(
    start_field: &[u8],
    separator: u8,
    marker: &[u8],
    end_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == (if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        })
            && long_incremental_signatures_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_incremental_signatures_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalSignatures
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && name.volume.is_none()
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_incremental_signatures_fields_model(
        start_field@,
        separator,
        marker@,
        end_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalSignatures,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: false,
        }),
        None => result == Err(if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        }),
    },
{
    if !time_separator_valid(separator) {
        return Err(NameError::Configuration);
    }
    let (start, end) = decode_long_interval(
        ObjectKind::IncrementalSignatures,
        start_field,
        separator,
        Some(marker),
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::IncrementalSignatures,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalSignatures,
        start,
        end,
        volume: None,
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_incremental_signatures_fields_model);
        assert(long_interval_model(
            ObjectKind::IncrementalSignatures,
            start_field@,
            separator,
            Some(marker@),
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(long_trailer_suffix_model(
            ObjectKind::IncrementalSignatures,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
//= docs/parsing-requirements.md#parse-gen-003
//= type=implementation
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
verus! {
fn long_full_volume_body_fields(
    start_field: &[u8],
    separator: u8,
    volume_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == (if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        })
            && long_full_volume_fields_model(
            start_field@,
            separator,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_full_volume_fields_model(
            start_field@,
            separator,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, volume, envelope, partial)) =>
                name.kind == ObjectKind::FullVolume
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && (name.volume matches Some(actual) && actual as int == volume)
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_full_volume_fields_model(
        start_field@,
        separator,
        volume_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, volume, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::FullVolume,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: Some(volume as u64),
            envelope,
            partial,
            short: false,
        }),
        None => result == Err(if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        }),
    },
{
    let start = parse_long_timestamp(start_field, separator)?;
    let volume = decode_long_volume_number(volume_field).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::FullVolume,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::FullVolume,
        start,
        end: start,
        volume: Some(volume),
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_full_volume_fields_model);
        assert(long_timestamp_result_model(start_field@, separator) == Some(start.0 as int));
        assert(long_volume_number_model(volume_field@) == Some(volume as int));
        assert(long_trailer_suffix_model(
            ObjectKind::FullVolume,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) == Some((envelope, partial)));
    }
    Ok(result)
}
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-004
//= type=implementation
//# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-007
//= type=implementation
//# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
//= docs/parsing-requirements.md#parse-gen-003
//= type=implementation
//# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
verus! {
#[allow(clippy::too_many_arguments)] // Long-name fields map directly to the parsed body.
fn long_incremental_volume_body_fields(
    start_field: &[u8],
    separator: u8,
    marker: &[u8],
    end_field: &[u8],
    volume_field: &[u8],
    trailer: &[u8],
    first: Option<&[u8]>,
    second: Option<&[u8]>,
    third: Option<&[u8]>,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == (if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        })
            && long_incremental_volume_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ).is_none(),
        Ok(name) => match long_incremental_volume_fields_model(
            start_field@,
            separator,
            marker@,
            end_field@,
            volume_field@,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
        ) {
            Some((start, end, volume, envelope, partial)) =>
                name.kind == ObjectKind::IncrementalVolume
                    && name.start.0 as int == start
                    && name.end.0 as int == end
                    && (name.volume matches Some(actual) && actual as int == volume)
                    && name.envelope == envelope
                    && name.partial == partial
                    && !name.short,
            None => false,
        },
    } && match long_incremental_volume_fields_model(
        start_field@,
        separator,
        marker@,
        end_field@,
        volume_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
        optional_short_token(third),
    ) {
        Some((start, end, volume, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalVolume,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: Some(volume as u64),
            envelope,
            partial,
            short: false,
        }),
        None => result == Err(if time_separator_valid_model(separator) {
            NameError::Malformed
        } else {
            NameError::Configuration
        }),
    },
{
    if !time_separator_valid(separator) {
        return Err(NameError::Configuration);
    }
    let (start, end) = decode_long_interval(
        ObjectKind::IncrementalVolume,
        start_field,
        separator,
        Some(marker),
        Some(end_field),
    ).ok_or(NameError::Malformed)?;
    let volume = decode_long_volume_number(volume_field).ok_or(NameError::Malformed)?;
    let (envelope, partial) = decode_long_trailer_suffix(
        ObjectKind::IncrementalVolume,
        trailer,
        first,
        second,
        third,
    ).ok_or(NameError::Malformed)?;
    let result = ObjectName {
        kind: ObjectKind::IncrementalVolume,
        start,
        end,
        volume: Some(volume),
        envelope,
        partial,
        short: false,
    };
    proof {
        reveal(long_incremental_volume_fields_model);
        assert(long_interval_model(
            ObjectKind::IncrementalVolume,
            start_field@,
            separator,
            Some(marker@),
            Some(end_field@),
        ) == Some((start.0 as int, end.0 as int)));
        assert(long_volume_number_model(volume_field@) == Some(volume as int));
        assert(long_trailer_suffix_model(
            ObjectKind::IncrementalVolume,
            trailer@,
            optional_short_token(first),
            optional_short_token(second),
            optional_short_token(third),
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
        Err(error) => error == NameError::Malformed && short_incremental_manifest_fields_model(
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
    } && match short_incremental_manifest_fields_model(
        start_field@,
        end_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
    ) {
        Some((start, end, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalManifest,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: None,
            envelope,
            partial,
            short: true,
        }),
        None => result == Err(NameError::Malformed),
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
        Err(error) => error == NameError::Malformed && short_incremental_volume_fields_model(
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
    } && match short_incremental_volume_fields_model(
        start_field@,
        end_field@,
        volume_field@,
        trailer@,
        optional_short_token(first),
        optional_short_token(second),
    ) {
        Some((start, end, volume, envelope, partial)) => result == Ok(ObjectName {
            kind: ObjectKind::IncrementalVolume,
            start: Timestamp(start as i128),
            end: Timestamp(end as i128),
            volume: Some(volume as u64),
            envelope,
            partial,
            short: true,
        }),
        None => result == Err(NameError::Malformed),
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

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-006
//= type=implementation
//# An incremental set MUST have an end timestamp strictly later than its start timestamp.
//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
verus! {
closed spec fn body_success_result_model(
    input: Seq<u8>,
    kind: ObjectKind,
    short: bool,
    separator: u8,
    name: ObjectName,
) -> bool {
    name.kind == kind && name.short == short
        && if kind == ObjectKind::FullManifest && short {
            name.volume.is_none()
                && short_full_manifest_raw_body_model(input, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::FullSignatures && short {
            name.volume.is_none()
                && short_full_signatures_raw_body_model(input, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::FullVolume && short {
            name.volume matches Some(volume)
                && short_full_volume_raw_body_model(input, name.start.0 as int, name.end.0 as int, volume as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalManifest && short {
            name.volume.is_none()
                && short_incremental_manifest_raw_body_model(input, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalSignatures && short {
            name.volume.is_none()
                && short_incremental_signatures_raw_body_model(input, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalVolume && short {
            name.volume matches Some(volume)
                && short_incremental_volume_raw_body_model(input, name.start.0 as int, name.end.0 as int, volume as int, name.envelope, name.partial)
        } else if kind == ObjectKind::FullManifest && !short {
            name.volume.is_none()
                && long_full_manifest_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::FullSignatures && !short {
            name.volume.is_none()
                && long_full_signatures_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::FullVolume && !short {
            name.volume matches Some(volume)
                && long_full_volume_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, volume as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalManifest && !short {
            name.volume.is_none()
                && long_incremental_manifest_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalSignatures && !short {
            name.volume.is_none()
                && long_incremental_signatures_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, name.envelope, name.partial)
        } else if kind == ObjectKind::IncrementalVolume && !short {
            name.volume matches Some(volume)
                && long_incremental_volume_raw_body_model(input, separator, name.start.0 as int, name.end.0 as int, volume as int, name.envelope, name.partial)
        } else {
            true
        }
}

//= docs/parsing-requirements.md#parse-name-010
//= type=implementation
//# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
closed spec fn body_error_result_model(separator: u8, error: NameError) -> bool {
    match error {
        NameError::Malformed => true,
        NameError::Configuration => !time_separator_valid_model(separator),
        NameError::Limit | NameError::Ambiguous => false,
    }
}

closed spec fn body_outcome_result_model(
    input: Seq<u8>,
    kind: ObjectKind,
    short: bool,
    separator: u8,
    outcome: Result<ObjectName, NameError>,
) -> bool {
    match outcome {
        Ok(name) => body_success_result_model(input, kind, short, separator, name),
        Err(error) => body_error_result_model(separator, error),
    }
}

pub closed spec fn parse_name_body_outcomes_model(
    input: Seq<u8>,
    global_prefix: Seq<u8>,
    archive_prefix: Seq<u8>,
    manifest_prefix: Seq<u8>,
    signature_prefix: Seq<u8>,
    separator: u8,
    outcomes: Seq<Option<Result<ObjectName, NameError>>>,
) -> bool {
    outcomes.len() == 12
        && forall|index: int| #![trigger outcomes[index]]
            0 <= index < 12 ==>
            match (
                name_candidate_remainder_model(
                    input,
                    global_prefix,
                    archive_prefix,
                    manifest_prefix,
                    signature_prefix,
                    name_candidate_table_model()[index].0,
                    name_candidate_table_model()[index].1,
                ),
                outcomes[index],
            ) {
                (None, None) => true,
                (Some(rest), Some(outcome)) => body_outcome_result_model(
                    rest,
                    name_candidate_table_model()[index].0,
                    name_candidate_table_model()[index].1,
                    separator,
                    outcome,
                ),
                _ => false,
            }
}

pub closed spec fn name_success_candidates_match_model(
    outcomes: Seq<Option<Result<ObjectName, NameError>>>,
    candidates: Seq<Option<ObjectName>>,
) -> bool {
    outcomes.len() == 12
        && candidates.len() == 12
        && forall|index: int| #![trigger candidates[index]]
            0 <= index < 12 ==>
            candidates[index] == successful_name_body_candidate_model(outcomes[index])
}

//= docs/parsing-requirements.md#parse-name-010
//= type=implementation
//# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn short_incremental_volume_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::IncrementalVolume
            && name.short
            && (match name.volume {
                Some(volume) => short_incremental_volume_raw_body_model(
                    input@,
                    name.start.0 as int,
                    name.end.0 as int,
                    volume as int,
                    name.envelope,
                    name.partial,
                ),
                None => false,
            }),
    },
{
    let _ = separator; // Short incremental-volume bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_end = fields;
    let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_volume = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_incremental_volume_body_fields(
        start_field,
        end_field,
        volume_field,
        trailer,
        first,
        second,
    );
    proof {
        if let Ok(name) = result {
            if let Some(volume) = name.volume {
                short_incremental_volume_raw_body_from_fields(
                    input@,
                    start_field@,
                    optional_body_field_model(_after_start),
                    end_field@,
                    optional_body_field_model(_after_end),
                    volume_field@,
                    optional_body_field_model(_after_volume),
                    trailer@,
                    optional_body_field_model(_after_trailer),
                    optional_body_field_model(first),
                    optional_body_field_model(second),
                    optional_body_field_model(third),
                    optional_body_field_model(extra),
                    optional_body_field_model(fields),
                    volume,
                    name,
                );
            }
        }
    }
    result
}

proof fn short_incremental_volume_raw_body_from_fields(
    input: Seq<u8>,
    start_field: Seq<u8>,
    after_start: Option<Seq<u8>>,
    end_field: Seq<u8>,
    after_end: Option<Seq<u8>>,
    volume_field: Seq<u8>,
    after_volume: Option<Seq<u8>>,
    trailer: Seq<u8>,
    after_trailer: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    fields: Option<Seq<u8>>,
    volume: u64,
    name: ObjectName,
)
    requires
        body_field_split_model(Some(input), Some(start_field), after_start),
        body_field_split_model(after_start, Some(end_field), after_end),
        body_field_split_model(after_end, Some(volume_field), after_volume),
        body_field_split_model(after_volume, Some(trailer), after_trailer),
        body_suffix_fields_model(after_trailer, first, second, third, extra, fields),
        third.is_none(),
        extra.is_none(),
        fields.is_none(),
        name.volume == Some(volume),
        short_incremental_volume_fields_model(
            start_field, end_field, volume_field, trailer, first, second,
        ) == Some((
            name.start.0 as int,
            name.end.0 as int,
            volume as int,
            name.envelope,
            name.partial,
        )),
    ensures short_incremental_volume_raw_body_model(
        input,
        name.start.0 as int,
        name.end.0 as int,
        volume as int,
        name.envelope,
        name.partial,
    )
{
    reveal(short_incremental_volume_raw_body_model);
    reveal(short_incremental_volume_fields_model);
}

#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn short_incremental_signatures_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::IncrementalSignatures
            && name.short
            && name.volume.is_none()
            && short_incremental_signatures_raw_body_model(
                input@,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
            ),
    },
{
    let _ = separator; // Short incremental-signatures bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_end = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_incremental_signatures_body_fields(
        start_field,
        end_field,
        trailer,
        first,
        second,
    );
    proof {
        if let Ok(name) = result {
            short_incremental_signatures_raw_body_from_fields(
                input@,
                start_field@,
                optional_body_field_model(_after_start),
                end_field@,
                optional_body_field_model(_after_end),
                trailer@,
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
                name,
            );
        }
    }
    result
}

proof fn short_incremental_signatures_raw_body_from_fields(
    input: Seq<u8>,
    start_field: Seq<u8>,
    after_start: Option<Seq<u8>>,
    end_field: Seq<u8>,
    after_end: Option<Seq<u8>>,
    trailer: Seq<u8>,
    after_trailer: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    fields: Option<Seq<u8>>,
    name: ObjectName,
)
    requires
        body_field_split_model(Some(input), Some(start_field), after_start),
        body_field_split_model(after_start, Some(end_field), after_end),
        body_field_split_model(after_end, Some(trailer), after_trailer),
        body_suffix_fields_model(after_trailer, first, second, third, extra, fields),
        third.is_none(),
        extra.is_none(),
        fields.is_none(),
        short_incremental_signatures_fields_model(start_field, end_field, trailer, first, second)
            == Some((name.start.0 as int, name.end.0 as int, name.envelope, name.partial)),
    ensures short_incremental_signatures_raw_body_model(
        input,
        name.start.0 as int,
        name.end.0 as int,
        name.envelope,
        name.partial,
    )
{
    reveal(short_incremental_signatures_raw_body_model);
    reveal(short_incremental_signatures_fields_model);
}

#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn short_incremental_manifest_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::IncrementalManifest
            && name.short
            && name.volume.is_none()
            && short_incremental_manifest_raw_body_model(
                input@,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
            ),
    },
{
    let _ = separator; // Short incremental-manifest bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_end = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_incremental_manifest_body_fields(
        start_field,
        end_field,
        trailer,
        first,
        second,
    );
    proof {
        if let Ok(name) = result {
            short_incremental_manifest_raw_body_from_fields(
                input@,
                start_field@,
                optional_body_field_model(_after_start),
                end_field@,
                optional_body_field_model(_after_end),
                trailer@,
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
                name,
            );
        }
    }
    result
}

proof fn short_incremental_manifest_raw_body_from_fields(
    input: Seq<u8>,
    start_field: Seq<u8>,
    after_start: Option<Seq<u8>>,
    end_field: Seq<u8>,
    after_end: Option<Seq<u8>>,
    trailer: Seq<u8>,
    after_trailer: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    fields: Option<Seq<u8>>,
    name: ObjectName,
)
    requires
        body_field_split_model(Some(input), Some(start_field), after_start),
        body_field_split_model(after_start, Some(end_field), after_end),
        body_field_split_model(after_end, Some(trailer), after_trailer),
        body_suffix_fields_model(after_trailer, first, second, third, extra, fields),
        third.is_none(),
        extra.is_none(),
        fields.is_none(),
        short_incremental_manifest_fields_model(start_field, end_field, trailer, first, second)
            == Some((name.start.0 as int, name.end.0 as int, name.envelope, name.partial)),
    ensures short_incremental_manifest_raw_body_model(
        input,
        name.start.0 as int,
        name.end.0 as int,
        name.envelope,
        name.partial,
    )
{
    reveal(short_incremental_manifest_raw_body_model);
    reveal(short_incremental_manifest_fields_model);
}

#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn short_full_manifest_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::FullManifest
            && name.short
            && name.volume.is_none()
            && short_full_manifest_raw_body_model(
                input@,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
            ),
    },
{
    let _ = separator; // Short full-manifest bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_full_manifest_body_fields(start_field, trailer, first, second);
    proof {
        if let Ok(name) = result {
            short_full_manifest_raw_body_from_fields(
                input@,
                start_field@,
                optional_body_field_model(_after_start),
                trailer@,
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
                name,
            );
        }
    }
    result
}

proof fn short_full_manifest_raw_body_from_fields(
    input: Seq<u8>,
    start_field: Seq<u8>,
    after_start: Option<Seq<u8>>,
    trailer: Seq<u8>,
    after_trailer: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    fields: Option<Seq<u8>>,
    name: ObjectName,
)
    requires
        body_field_split_model(Some(input), Some(start_field), after_start),
        body_field_split_model(after_start, Some(trailer), after_trailer),
        body_suffix_fields_model(after_trailer, first, second, third, extra, fields),
        third.is_none(),
        extra.is_none(),
        fields.is_none(),
        short_full_manifest_fields_model(start_field, trailer, first, second)
            == Some((name.start.0 as int, name.end.0 as int, name.envelope, name.partial)),
    ensures short_full_manifest_raw_body_model(
        input,
        name.start.0 as int,
        name.end.0 as int,
        name.envelope,
        name.partial,
    )
{
    reveal(short_full_manifest_raw_body_model);
    reveal(short_full_manifest_fields_model);
}

#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn short_full_signatures_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::FullSignatures
            && name.short
            && name.volume.is_none()
            && short_full_signatures_raw_body_model(
                input@,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
            ),
    },
{
    let _ = separator; // Short full-signatures bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_full_signatures_body_fields(start_field, trailer, first, second);
    proof {
        if let Ok(name) = result {
            short_full_signatures_raw_body_from_fields(
                input@,
                start_field@,
                optional_body_field_model(_after_start),
                trailer@,
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
                name,
            );
        }
    }
    result
}

proof fn short_full_signatures_raw_body_from_fields(
    input: Seq<u8>,
    start_field: Seq<u8>,
    after_start: Option<Seq<u8>>,
    trailer: Seq<u8>,
    after_trailer: Option<Seq<u8>>,
    first: Option<Seq<u8>>,
    second: Option<Seq<u8>>,
    third: Option<Seq<u8>>,
    extra: Option<Seq<u8>>,
    fields: Option<Seq<u8>>,
    name: ObjectName,
)
    requires
        body_field_split_model(Some(input), Some(start_field), after_start),
        body_field_split_model(after_start, Some(trailer), after_trailer),
        body_suffix_fields_model(after_trailer, first, second, third, extra, fields),
        third.is_none(),
        extra.is_none(),
        fields.is_none(),
        short_full_signatures_fields_model(start_field, trailer, first, second)
            == Some((name.start.0 as int, name.end.0 as int, name.envelope, name.partial)),
    ensures short_full_signatures_raw_body_model(
        input,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
    )
{
    reveal(short_full_signatures_raw_body_model);
    reveal(short_full_signatures_fields_model);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-010
//= type=implementation
//# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
fn short_full_volume_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => body_success_result_model(
            input@, ObjectKind::FullVolume, true, separator, name,
        ),
    },
{
    let _ = separator; // Short full-volume bodies do not consume the time separator.
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_volume = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if third.is_some() || extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = short_full_volume_body_fields(start_field, volume_field, trailer, first, second);
    proof {
        if let Ok(name) = result {
            reveal(body_success_result_model);
            reveal(short_full_volume_raw_body_model);
            reveal(short_full_volume_fields_model);
            reveal(body_suffix_fields_model);
            reveal(body_field_split_model);
            assert(body_field_split_model(
                Some(input@), Some(start_field@),
                optional_body_field_model(_after_start),
            ));
            assert(body_field_split_model(
                optional_body_field_model(_after_start), Some(volume_field@),
                optional_body_field_model(_after_volume),
            ));
            assert(body_field_split_model(
                optional_body_field_model(_after_volume), Some(trailer@),
                optional_body_field_model(_after_trailer),
            ));
            assert(body_suffix_fields_model(
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
            ));
            assert(name.volume matches Some(_));
            assert(short_full_volume_raw_body_model(
                input@,
                name.start.0 as int,
                name.end.0 as int,
                match name.volume {
                    Some(volume) => volume as int,
                    None => 0,
                },
                name.envelope,
                name.partial,
            ));
        }
    }
    result
}

//= docs/parsing-requirements.md#parse-name-010
//= type=implementation
//# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
#[verifier::spinoff_prover]
#[verifier::rlimit(40)]
fn long_full_manifest_body(
    input: &[u8],
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Err(error) => error == NameError::Malformed,
        Ok(name) => name.kind == ObjectKind::FullManifest
            && !name.short
            && name.volume.is_none()
            && long_full_manifest_raw_body_model(
                input@,
                separator,
                name.start.0 as int,
                name.end.0 as int,
                name.envelope,
                name.partial,
            ),
    },
{
    let mut fields = Some(input);
    let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_start = fields;
    let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
    let _after_trailer = fields;
    let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
    if extra.is_some() || fields.is_some() {
        return Err(NameError::Malformed);
    }
    let result = match long_full_manifest_body_fields(
        start_field,
        separator,
        trailer,
        first,
        second,
        third,
    ) {
        Ok(name) => Ok(name),
        Err(_) => Err(NameError::Malformed),
    };
    proof {
        if let Ok(name) = result {
            reveal(long_full_manifest_raw_body_model);
            reveal(long_full_manifest_fields_model);
            reveal(body_suffix_fields_model);
            reveal(body_field_split_model);
            assert(body_field_split_model(
                Some(input@), Some(start_field@),
                optional_body_field_model(_after_start),
            ));
            assert(body_field_split_model(
                optional_body_field_model(_after_start), Some(trailer@),
                optional_body_field_model(_after_trailer),
            ));
            assert(body_suffix_fields_model(
                optional_body_field_model(_after_trailer),
                optional_body_field_model(first),
                optional_body_field_model(second),
                optional_body_field_model(third),
                optional_body_field_model(extra),
                optional_body_field_model(fields),
            ));
        }
    }
    result
}

#[verifier::spinoff_prover]
#[verifier::rlimit(80)]
//= docs/parsing-requirements.md#parse-name-010
//= type=implementation
//# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
fn body(
    input: &[u8],
    kind: ObjectKind,
    short: bool,
    separator: u8,
) -> (result: Result<ObjectName, NameError>)
    ensures match result {
        Ok(name) => body_success_result_model(input@, kind, short, separator, name),
        Err(error) => body_error_result_model(separator, error),
    },
{
    proof {
        reveal(body_success_result_model);
    }
    let selected_route = crate::name_dispatch::select_body_route(kind, short);
    match selected_route {
        crate::name_dispatch::BodyRoute::ShortIncrementalSignatures => {
            let result = short_incremental_signatures_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::IncrementalSignatures);
                    assert(short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::ShortIncrementalManifest => {
            let result = short_incremental_manifest_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::IncrementalManifest);
                    assert(short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::ShortFullManifest => {
            let result = short_full_manifest_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::FullManifest);
                    assert(short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::LongFullManifest => {
            let result = long_full_manifest_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::FullManifest);
                    assert(!short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::LongFullSignatures => {
            let mut fields = Some(input);
            let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_start = fields;
            let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_trailer = fields;
            let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
            if extra.is_some() || fields.is_some() {
                return Err(NameError::Malformed);
            }
            let result = long_full_signatures_body_fields(
                start_field,
                separator,
                trailer,
                first,
                second,
                third,
            );
            proof {
                if let Ok(name) = result {
                    reveal(long_full_signatures_raw_body_model);
                    reveal(long_full_signatures_fields_model);
                    reveal(body_suffix_fields_model);
                    reveal(body_field_split_model);
                    assert(body_field_split_model(
                        Some(input@), Some(start_field@),
                        optional_body_field_model(_after_start),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_start), Some(trailer@),
                        optional_body_field_model(_after_trailer),
                    ));
                    assert(body_suffix_fields_model(
                        optional_body_field_model(_after_trailer),
                        optional_body_field_model(first),
                        optional_body_field_model(second),
                        optional_body_field_model(third),
                        optional_body_field_model(extra),
                        optional_body_field_model(fields),
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::LongIncrementalManifest => {
            let mut fields = Some(input);
            let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_start = fields;
            let marker = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_marker = fields;
            let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_end = fields;
            let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_trailer = fields;
            let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
            if extra.is_some() || fields.is_some() {
                return Err(NameError::Malformed);
            }
            let result = long_incremental_manifest_body_fields(
                start_field,
                separator,
                marker,
                end_field,
                trailer,
                first,
                second,
                third,
            );
            proof {
                if let Ok(name) = result {
                    reveal(long_incremental_manifest_raw_body_model);
                    reveal(long_incremental_manifest_fields_model);
                    reveal(long_interval_model);
                    reveal(long_timestamp_result_model);
                    reveal(long_trailer_suffix_model);
                    reveal(body_suffix_fields_model);
                    reveal(body_field_split_model);
                    assert(body_field_split_model(
                        Some(input@), Some(start_field@),
                        optional_body_field_model(_after_start),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_start), Some(marker@),
                        optional_body_field_model(_after_marker),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_marker), Some(end_field@),
                        optional_body_field_model(_after_end),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_end), Some(trailer@),
                        optional_body_field_model(_after_trailer),
                    ));
                    assert(body_suffix_fields_model(
                        optional_body_field_model(_after_trailer),
                        optional_body_field_model(first),
                        optional_body_field_model(second),
                        optional_body_field_model(third),
                        optional_body_field_model(extra),
                        optional_body_field_model(fields),
                    ));
                    assert(long_incremental_manifest_fields_model(
                        start_field@,
                        separator,
                        marker@,
                        end_field@,
                        trailer@,
                        optional_short_token(first),
                        optional_short_token(second),
                        optional_short_token(third),
                    ) == Some((
                        name.start.0 as int,
                        name.end.0 as int,
                        name.envelope,
                        name.partial,
                    )));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::LongIncrementalSignatures => {
            let mut fields = Some(input);
            let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_start = fields;
            let marker = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_marker = fields;
            let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_end = fields;
            let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_trailer = fields;
            let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
            if extra.is_some() || fields.is_some() {
                return Err(NameError::Malformed);
            }
            let result = long_incremental_signatures_body_fields(
                start_field,
                separator,
                marker,
                end_field,
                trailer,
                first,
                second,
                third,
            );
            proof {
                if let Ok(name) = result {
                    reveal(long_incremental_signatures_raw_body_model);
                    reveal(long_incremental_signatures_fields_model);
                    reveal(long_interval_model);
                    reveal(long_timestamp_result_model);
                    reveal(long_trailer_suffix_model);
                    reveal(body_suffix_fields_model);
                    reveal(body_field_split_model);
                    assert(body_field_split_model(
                        Some(input@), Some(start_field@),
                        optional_body_field_model(_after_start),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_start), Some(marker@),
                        optional_body_field_model(_after_marker),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_marker), Some(end_field@),
                        optional_body_field_model(_after_end),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_end), Some(trailer@),
                        optional_body_field_model(_after_trailer),
                    ));
                    assert(body_suffix_fields_model(
                        optional_body_field_model(_after_trailer),
                        optional_body_field_model(first),
                        optional_body_field_model(second),
                        optional_body_field_model(third),
                        optional_body_field_model(extra),
                        optional_body_field_model(fields),
                    ));
                    assert(name.volume.is_none());
                    assert(long_incremental_signatures_fields_model(
                        start_field@,
                        separator,
                        marker@,
                        end_field@,
                        trailer@,
                        optional_short_token(first),
                        optional_short_token(second),
                        optional_short_token(third),
                    ) == Some((
                        name.start.0 as int,
                        name.end.0 as int,
                        name.envelope,
                        name.partial,
                    )));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::ShortFullSignatures => {
            let result = short_full_signatures_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::FullSignatures);
                    assert(short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::ShortIncrementalVolume => {
            let result = short_incremental_volume_body(input, separator);
            proof {
                if let Ok(name) = result {
                    reveal(body_success_result_model);
                    assert(kind == ObjectKind::IncrementalVolume);
                    assert(short);
                    assert(body_success_result_model(
                        input@, kind, short, separator, name,
                    ));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::ShortFullVolume => {
            short_full_volume_body(input, separator)
        }
        crate::name_dispatch::BodyRoute::LongFullVolume => {
            let mut fields = Some(input);
            let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_start = fields;
            let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_volume = fields;
            let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_trailer = fields;
            let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
            if extra.is_some() || fields.is_some() {
                return Err(NameError::Malformed);
            }
            let result = long_full_volume_body_fields(
                start_field,
                separator,
                volume_field,
                trailer,
                first,
                second,
                third,
            );
            proof {
                if let Ok(name) = result {
                    let volume = match name.volume {
                        Some(value) => value,
                        None => 0,
                    };
                    reveal(long_full_volume_raw_body_model);
                    reveal(long_full_volume_fields_model);
                    reveal(long_timestamp_result_model);
                    reveal(long_volume_number_model);
                    reveal(long_trailer_suffix_model);
                    reveal(body_suffix_fields_model);
                    reveal(body_field_split_model);
                    assert(body_field_split_model(
                        Some(input@), Some(start_field@),
                        optional_body_field_model(_after_start),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_start), Some(volume_field@),
                        optional_body_field_model(_after_volume),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_volume), Some(trailer@),
                        optional_body_field_model(_after_trailer),
                    ));
                    assert(body_suffix_fields_model(
                        optional_body_field_model(_after_trailer),
                        optional_body_field_model(first),
                        optional_body_field_model(second),
                        optional_body_field_model(third),
                        optional_body_field_model(extra),
                        optional_body_field_model(fields),
                    ));
                    assert(name.volume matches Some(_));
                    assert(long_full_volume_fields_model(
                        start_field@,
                        separator,
                        volume_field@,
                        trailer@,
                        optional_short_token(first),
                        optional_short_token(second),
                        optional_short_token(third),
                    ) == Some((
                        name.start.0 as int,
                        name.end.0 as int,
                        volume as int,
                        name.envelope,
                        name.partial,
                    )));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::LongIncrementalVolume => {
            let mut fields = Some(input);
            let start_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_start = fields;
            let marker = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_marker = fields;
            let end_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_end = fields;
            let volume_field = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_volume = fields;
            let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
            let _after_trailer = fields;
            let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
            if extra.is_some() || fields.is_some() {
                return Err(NameError::Malformed);
            }
            let result = long_incremental_volume_body_fields(
                start_field,
                separator,
                marker,
                end_field,
                volume_field,
                trailer,
                first,
                second,
                third,
            );
            proof {
                if let Ok(name) = result {
                    let volume = match name.volume {
                        Some(value) => value,
                        None => 0,
                    };
                    reveal(long_incremental_volume_raw_body_model);
                    reveal(long_incremental_volume_fields_model);
                    reveal(long_interval_model);
                    reveal(long_timestamp_result_model);
                    reveal(long_volume_number_model);
                    reveal(long_trailer_suffix_model);
                    reveal(body_suffix_fields_model);
                    reveal(body_field_split_model);
                    assert(body_field_split_model(
                        Some(input@), Some(start_field@),
                        optional_body_field_model(_after_start),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_start), Some(marker@),
                        optional_body_field_model(_after_marker),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_marker), Some(end_field@),
                        optional_body_field_model(_after_end),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_end), Some(volume_field@),
                        optional_body_field_model(_after_volume),
                    ));
                    assert(body_field_split_model(
                        optional_body_field_model(_after_volume), Some(trailer@),
                        optional_body_field_model(_after_trailer),
                    ));
                    assert(body_suffix_fields_model(
                        optional_body_field_model(_after_trailer),
                        optional_body_field_model(first),
                        optional_body_field_model(second),
                        optional_body_field_model(third),
                        optional_body_field_model(extra),
                        optional_body_field_model(fields),
                    ));
                    assert(name.volume matches Some(_));
                    assert(long_incremental_volume_fields_model(
                        start_field@,
                        separator,
                        marker@,
                        end_field@,
                        volume_field@,
                        trailer@,
                        optional_short_token(first),
                        optional_short_token(second),
                        optional_short_token(third),
                    ) == Some((
                        name.start.0 as int,
                        name.end.0 as int,
                        volume as int,
                        name.envelope,
                        name.partial,
                    )));
                }
            }
            result
        }
        crate::name_dispatch::BodyRoute::Unmatched => {
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
                let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
                if extra.is_some() {
                    return Err(NameError::Malformed);
                }
                decode_short_trailer(kind, trailer, first, second, third)
                    .ok_or(NameError::Malformed)?
            } else {
                //= docs/parsing-requirements.md#parse-name-001
                //= type=implementation
                //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
                let trailer = take_dot_field(&mut fields).ok_or(NameError::Malformed)?;
                let (first, second, third, extra) = take_body_suffix_fields(&mut fields);
                if extra.is_some() {
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
    }
}
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

verus! {
closed spec fn name_candidate_remainder_model(
    input: Seq<u8>,
    global_prefix: Seq<u8>,
    archive_prefix: Seq<u8>,
    manifest_prefix: Seq<u8>,
    signature_prefix: Seq<u8>,
    kind: ObjectKind,
    short: bool,
) -> Option<Seq<u8>> {
    match configured_prefix_remainder_model(
        input,
        global_prefix,
        kind_specific_prefix_model(kind, archive_prefix, manifest_prefix, signature_prefix),
    ) {
        Some(after_prefix) => stem_remainder_model(after_prefix, stem_model(kind, short)),
        None => None,
    }
}

//= docs/parsing-requirements.md#parse-name-002
//= type=implementation
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
fn name_candidate_remainder<'a>(
    input: &'a [u8],
    global_prefix: &[u8],
    archive_prefix: &[u8],
    manifest_prefix: &[u8],
    signature_prefix: &[u8],
    kind: ObjectKind,
    short: bool,
) -> (remainder: Option<&'a [u8]>)
    ensures match remainder {
        Some(rest) => name_candidate_remainder_model(
            input@,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
            kind,
            short,
        ) == Some(rest@),
        None => name_candidate_remainder_model(
            input@,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
            kind,
            short,
        ).is_none(),
    },
{
    let kind_prefix = kind_specific_prefix(kind, archive_prefix, manifest_prefix, signature_prefix);
    match configured_prefix_remainder(input, global_prefix, kind_prefix) {
        Some(after_prefix) => stem_remainder(after_prefix, stem(kind, short)),
        None => None,
    }
}

pub closed spec fn name_candidate_table_has_stem_model(
    input: Seq<u8>,
    global_prefix: Seq<u8>,
    archive_prefix: Seq<u8>,
    manifest_prefix: Seq<u8>,
    signature_prefix: Seq<u8>,
) -> bool {
    exists|index: int| 0 <= index < name_candidate_table_model().len()
        && name_candidate_remainder_model(
            input,
            global_prefix,
            archive_prefix,
            manifest_prefix,
            signature_prefix,
            name_candidate_table_model()[index].0,
            name_candidate_table_model()[index].1,
        ).is_some()
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-002
//= type=implementation
//# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
fn name_candidate_remainders<'a>(
    input: &'a [u8],
    global_prefix: &[u8],
    archive_prefix: &[u8],
    manifest_prefix: &[u8],
    signature_prefix: &[u8],
) -> (result: ([Option<&'a [u8]>; 12], bool))
    ensures result.1 == name_candidate_table_has_stem_model(
            input@,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ),
        forall|candidate: int| #![trigger result.0@[candidate]]
            0 <= candidate < 12 ==>
            match result.0@[candidate] {
                Some(rest) => name_candidate_remainder_model(
                    input@,
                    global_prefix@,
                    archive_prefix@,
                    manifest_prefix@,
                    signature_prefix@,
                    name_candidate_table_model()[candidate].0,
                    name_candidate_table_model()[candidate].1,
                ) == Some(rest@),
                None => name_candidate_remainder_model(
                    input@,
                    global_prefix@,
                    archive_prefix@,
                    manifest_prefix@,
                    signature_prefix@,
                    name_candidate_table_model()[candidate].0,
                    name_candidate_table_model()[candidate].1,
                ).is_none(),
            },
{
    let candidates = name_candidate_table();
    let mut index = 0usize;
    let mut recognized = false;
    let mut remainders: [Option<&'a [u8]>; 12] = [None; 12];
    while index < candidates.len()
        invariant index <= candidates.len(),
            recognized == (exists|prior: int| 0 <= prior < index as int
                && name_candidate_remainder_model(
                    input@,
                    global_prefix@,
                    archive_prefix@,
                    manifest_prefix@,
                    signature_prefix@,
                    candidates@[prior].0,
                    candidates@[prior].1,
                ).is_some()),
            forall|prior: int| #![trigger remainders@[prior]]
                0 <= prior < index as int ==>
                match remainders@[prior] {
                    Some(rest) => name_candidate_remainder_model(
                        input@,
                        global_prefix@,
                        archive_prefix@,
                        manifest_prefix@,
                        signature_prefix@,
                        candidates@[prior].0,
                        candidates@[prior].1,
                    ) == Some(rest@),
                    None => name_candidate_remainder_model(
                        input@,
                        global_prefix@,
                        archive_prefix@,
                        manifest_prefix@,
                        signature_prefix@,
                        candidates@[prior].0,
                        candidates@[prior].1,
                    ).is_none(),
                },
        decreases candidates.len() - index,
    {
        let (kind, short) = candidates[index];
        let remainder = name_candidate_remainder(
            input,
            global_prefix,
            archive_prefix,
            manifest_prefix,
            signature_prefix,
            kind,
            short,
        );
        remainders[index] = remainder;
        if remainder.is_some() {
            recognized = true;
        }
        proof {
            reveal(name_candidate_remainder_model);
            reveal(name_candidate_table_has_stem_model);
            assert(candidates@[index as int] == (kind, short));
            if remainder.is_some() {
                assert(name_candidate_remainder_model(
                    input@,
                    global_prefix@,
                    archive_prefix@,
                    manifest_prefix@,
                    signature_prefix@,
                    kind,
                    short,
                ).is_some());
                assert(exists|prior: int| 0 <= prior < (index + 1) as int
                    && name_candidate_remainder_model(
                        input@,
                        global_prefix@,
                        archive_prefix@,
                        manifest_prefix@,
                        signature_prefix@,
                        candidates@[prior].0,
                        candidates@[prior].1,
                    ).is_some());
            } else {
                assert(name_candidate_remainder_model(
                    input@,
                    global_prefix@,
                    archive_prefix@,
                    manifest_prefix@,
                    signature_prefix@,
                    kind,
                    short,
                ).is_none());
                assert(recognized == (exists|prior: int| 0 <= prior < (index + 1) as int
                    && name_candidate_remainder_model(
                        input@,
                        global_prefix@,
                        archive_prefix@,
                        manifest_prefix@,
                        signature_prefix@,
                        candidates@[prior].0,
                        candidates@[prior].1,
                    ).is_some()));
            }
        }
        index += 1;
    }
    proof {
        reveal(name_candidate_table_has_stem_model);
        assert(candidates@ == name_candidate_table_model());
        assert(recognized == name_candidate_table_has_stem_model(
            input@,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ));
    }
    (remainders, recognized)
}
}

verus! {
/// Input-byte predicate for the basename boundary.
closed spec fn name_basename_bytes_valid_model(input: Seq<u8>) -> bool {
    forall|index: int| 0 <= index < input.len()
        ==> input[index] != 0 && input[index] != b'/'
}

/// Enforce the input-size limit before rejecting path separators or NUL bytes.
/// This keeps the public parser's preflight error precedence explicit.
//= docs/parsing-requirements.md#parse-name-008
//= type=implementation
//# For a valid parser configuration, the reader MUST return the name-size limit error when a basename exceeds its configured byte limit, before checking basename contents. At or below that limit, the reader MUST reject a basename containing NUL or `/` with the malformed-name error.
fn validate_name_basename(input: &[u8], max_name_bytes: usize) -> (result: Result<(), NameError>)
    ensures match result {
        Ok(()) => input.len() <= max_name_bytes && name_basename_bytes_valid_model(input@),
        Err(NameError::Limit) => input.len() > max_name_bytes,
        Err(NameError::Malformed) => input.len() <= max_name_bytes
            && !name_basename_bytes_valid_model(input@),
        _ => false,
    },
{
    if input.len() > max_name_bytes {
        return Err(NameError::Limit);
    }
    let mut index = 0usize;
    while index < input.len()
        invariant index <= input.len(),
            input.len() <= max_name_bytes,
            forall|prior: int| 0 <= prior < index as int
                ==> input@[prior] != 0 && input@[prior] != b'/',
        decreases input.len() - index,
    {
        if input[index] == 0 || input[index] == b'/' {
            proof {
                assert(exists|bad: int| 0 <= bad < input@.len()
                    && (input@[bad] == 0 || input@[bad] == b'/')) by {
                    assert(0 <= index as int);
                    assert((index as int) < input@.len() as int);
                    assert(input@[index as int] == 0 || input@[index as int] == b'/');
                }
                reveal(name_basename_bytes_valid_model);
            }
            return Err(NameError::Malformed);
        }
        index += 1;
    }
    proof {
        reveal(name_basename_bytes_valid_model);
        assert(name_basename_bytes_valid_model(input@));
    }
    Ok(())
}

closed spec fn configured_name_prefix_valid_model(prefix: Seq<u8>) -> bool {
    forall|index: int| 0 <= index < prefix.len()
        ==> prefix[index] != 0 && prefix[index] != b'/'
}

//= docs/parsing-requirements.md#parse-name-009
//= type=implementation
//# Configured global, archive, manifest, and signature prefixes MAY be empty and otherwise contain arbitrary bytes, but MUST NOT contain NUL or `/`.
fn configured_name_prefix_valid(prefix: &[u8]) -> (result: bool)
    ensures result == configured_name_prefix_valid_model(prefix@),
{
    let mut index = 0usize;
    while index < prefix.len()
        invariant index <= prefix.len(),
            forall|prior: int| 0 <= prior < index as int
                ==> prefix@[prior] != 0 && prefix@[prior] != b'/',
        decreases prefix.len() - index,
    {
        if prefix[index] == 0 || prefix[index] == b'/' {
            proof {
                reveal(configured_name_prefix_valid_model);
                assert(!configured_name_prefix_valid_model(prefix@));
            }
            return false;
        }
        index += 1;
    }
    proof {
        reveal(configured_name_prefix_valid_model);
        assert(configured_name_prefix_valid_model(prefix@));
    }
    true
}

closed spec fn name_configuration_valid_model(
    separator: u8,
    global_prefix: Seq<u8>,
    archive_prefix: Seq<u8>,
    manifest_prefix: Seq<u8>,
    signature_prefix: Seq<u8>,
) -> bool {
    time_separator_valid_model(separator)
        && configured_name_prefix_valid_model(global_prefix)
        && configured_name_prefix_valid_model(archive_prefix)
        && configured_name_prefix_valid_model(manifest_prefix)
        && configured_name_prefix_valid_model(signature_prefix)
}

pub closed spec fn name_preflight_contract_model(
    input: Seq<u8>,
    separator: u8,
    global_prefix: Seq<u8>,
    archive_prefix: Seq<u8>,
    manifest_prefix: Seq<u8>,
    signature_prefix: Seq<u8>,
    max_name_bytes: usize,
) -> bool {
    time_separator_valid_model(separator)
        && forall|index: int| 0 <= index < global_prefix.len()
            ==> global_prefix[index] != 0 && global_prefix[index] != b'/'
        && forall|index: int| 0 <= index < archive_prefix.len()
            ==> archive_prefix[index] != 0 && archive_prefix[index] != b'/'
        && forall|index: int| 0 <= index < manifest_prefix.len()
            ==> manifest_prefix[index] != 0 && manifest_prefix[index] != b'/'
        && forall|index: int| 0 <= index < signature_prefix.len()
            ==> signature_prefix[index] != 0 && signature_prefix[index] != b'/'
        && input.len() <= max_name_bytes
        && forall|index: int| 0 <= index < input.len()
            ==> input[index] != 0 && input[index] != b'/'
}

//= docs/parsing-requirements.md#parse-name-009
//= type=implementation
//# The time separator MUST be in the valid set in PARSE-NAME-007.
fn name_configuration_valid(
    separator: u8,
    global_prefix: &[u8],
    archive_prefix: &[u8],
    manifest_prefix: &[u8],
    signature_prefix: &[u8],
) -> (result: bool)
    ensures result == name_configuration_valid_model(
        separator,
        global_prefix@,
        archive_prefix@,
        manifest_prefix@,
        signature_prefix@,
    ),
{
    let result = time_separator_valid(separator)
        && configured_name_prefix_valid(global_prefix)
        && configured_name_prefix_valid(archive_prefix)
        && configured_name_prefix_valid(manifest_prefix)
        && configured_name_prefix_valid(signature_prefix);
    proof {
        reveal(name_configuration_valid_model);
        assert(result == name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ));
    }
    result
}

//= docs/parsing-requirements.md#parse-name-009
//= type=implementation
//# Before checking basename size or bytes, the reader MUST validate the parser configuration.
//= docs/parsing-requirements.md#parse-name-009
//= type=implementation
//# Invalid configuration MUST return the configuration error before basename limit or malformed-name errors; valid configuration MUST proceed to the checks in PARSE-NAME-008.
fn validate_name_preflight(
    input: &[u8],
    separator: u8,
    global_prefix: &[u8],
    archive_prefix: &[u8],
    manifest_prefix: &[u8],
    signature_prefix: &[u8],
    max_name_bytes: usize,
) -> (result: Result<(), NameError>)
    ensures match result {
        Err(NameError::Configuration) => !name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ),
        Err(NameError::Limit) => name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ) && input.len() > max_name_bytes,
        Err(NameError::Malformed) => name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ) && input.len() <= max_name_bytes && !name_basename_bytes_valid_model(input@),
        Ok(()) => name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ) && input.len() <= max_name_bytes && name_basename_bytes_valid_model(input@),
        _ => false,
    },
{
    if !name_configuration_valid(
        separator,
        global_prefix,
        archive_prefix,
        manifest_prefix,
        signature_prefix,
    ) {
        proof {
            reveal(name_configuration_valid_model);
            assert(!name_configuration_valid_model(
                separator,
                global_prefix@,
                archive_prefix@,
                manifest_prefix@,
                signature_prefix@,
            ));
        }
        return Err(NameError::Configuration);
    }
    proof {
        reveal(name_configuration_valid_model);
        assert(name_configuration_valid_model(
            separator,
            global_prefix@,
            archive_prefix@,
            manifest_prefix@,
            signature_prefix@,
        ));
    }
    validate_name_basename(input, max_name_bytes)
}
}

//= docs/parsing-requirements.md#parse-name-009
//= type=implementation
//# Invalid configuration MUST return the configuration error before basename limit or malformed-name errors; valid configuration MUST proceed to the checks in PARSE-NAME-008.
verus! {
/// Parse a basename, not an S3 key. Unrelated names return `Ok(None)`.
/// Conflicting prefix configurations producing distinct successful classifications
/// return `Ambiguous`; malformed recognized stems are errors rather than ignored.
pub fn parse_name(input: &[u8], config: NameConfig<'_>) -> (result: Result<Option<ObjectName>, NameError>)
{
    let preflight = validate_name_preflight(
        input,
        config.time_separator,
        config.global_prefix,
        config.archive_prefix,
        config.manifest_prefix,
        config.signature_prefix,
        config.max_name_bytes,
    );
    preflight?;
    proof {
        reveal(name_preflight_contract_model);
        assert(name_preflight_contract_model(
            input@, config.time_separator, config.global_prefix@,
            config.archive_prefix@, config.manifest_prefix@,
            config.signature_prefix@, config.max_name_bytes));
    }
    parse_name_after_preflight(input, config)
}

fn parse_name_after_preflight(
    input: &[u8],
    config: NameConfig<'_>,
) -> (result: Result<Option<ObjectName>, NameError>)
    requires name_preflight_contract_model(
        input@,
        config.time_separator,
        config.global_prefix@,
        config.archive_prefix@,
        config.manifest_prefix@,
        config.signature_prefix@,
        config.max_name_bytes,
    ),
    ensures exists|outcomes: Seq<Option<Result<ObjectName, NameError>>>,
            candidates: Seq<Option<ObjectName>>|
            #![trigger parse_name_body_outcomes_model(
                input@,
                config.global_prefix@,
                config.archive_prefix@,
                config.manifest_prefix@,
                config.signature_prefix@,
                config.time_separator,
                outcomes,
            ), name_success_candidates_match_model(outcomes, candidates)]
            parse_name_body_outcomes_model(
                input@,
                config.global_prefix@,
                config.archive_prefix@,
                config.manifest_prefix@,
                config.signature_prefix@,
                config.time_separator,
                outcomes,
            )
                && name_success_candidates_match_model(outcomes, candidates)
                && result == name_candidate_controller_result_model(
                    name_candidate_table_has_stem_model(
                        input@,
                        config.global_prefix@,
                        config.archive_prefix@,
                        config.manifest_prefix@,
                        config.signature_prefix@,
                    ),
                    candidates,
                ),
{
    let (candidate_remainders, recognized) = name_candidate_remainders(
        input,
        config.global_prefix,
        config.archive_prefix,
        config.manifest_prefix,
        config.signature_prefix,
    );
    let body_outcomes = collect_name_body_outcomes(candidate_remainders, config.time_separator);
    let successful_candidates = successful_name_body_candidates(body_outcomes);
    let result = merge_name_candidate_successes(successful_candidates);
    //= docs/parsing-requirements.md#parse-name-001
    //= type=implementation
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    let result = finalize_name_candidate_controller(recognized, successful_candidates, result);
    proof {
        reveal(parse_name_body_outcomes_model);
        reveal(name_success_candidates_match_model);
        assert(parse_name_body_outcomes_model(
            input@,
            config.global_prefix@,
            config.archive_prefix@,
            config.manifest_prefix@,
            config.signature_prefix@,
            config.time_separator,
            body_outcomes@,
        ));
        assert(name_success_candidates_match_model(body_outcomes@, successful_candidates@));
        assert(result == name_candidate_controller_result_model(
            name_candidate_table_has_stem_model(
                input@,
                config.global_prefix@,
                config.archive_prefix@,
                config.manifest_prefix@,
                config.signature_prefix@,
            ),
            successful_candidates@,
        ));
        assert(exists|outcomes: Seq<Option<Result<ObjectName, NameError>>>,
            candidates: Seq<Option<ObjectName>>|
            parse_name_body_outcomes_model(
                input@,
                config.global_prefix@,
                config.archive_prefix@,
                config.manifest_prefix@,
                config.signature_prefix@,
                config.time_separator,
                outcomes,
            )
                && name_success_candidates_match_model(outcomes, candidates)
                && result == name_candidate_controller_result_model(
                    name_candidate_table_has_stem_model(
                        input@,
                        config.global_prefix@,
                        config.archive_prefix@,
                        config.manifest_prefix@,
                        config.signature_prefix@,
                    ),
                    candidates,
                ));
    }
    result
}
}

verus! {
closed spec fn name_candidate_merge_success_model(
    current: Option<ObjectName>,
    candidate: Option<ObjectName>,
) -> Result<Option<ObjectName>, NameError> {
    match candidate {
        None => Ok(current),
        Some(parsed) => name_candidate_merge_model(current, Some(parsed)),
    }
}

closed spec fn name_candidate_accumulation_model(
    candidates: Seq<Option<ObjectName>>,
    count: int,
) -> Result<Option<ObjectName>, NameError>
    decreases count,
{
    if count <= 0 || count > candidates.len() {
        Ok(None)
    } else {
        match name_candidate_accumulation_model(candidates, count - 1) {
            Err(error) => Err(error),
            Ok(current) => name_candidate_merge_success_model(current, candidates[count - 1]),
        }
    }
}

proof fn lemma_name_candidate_accumulation_step(
    candidates: Seq<Option<ObjectName>>,
    count: int,
)
    requires 0 <= count < candidates.len(),
    ensures name_candidate_accumulation_model(candidates, count + 1)
        == match name_candidate_accumulation_model(candidates, count) {
            Err(error) => Err(error),
            Ok(current) => name_candidate_merge_success_model(current, candidates[count]),
        },
{
    reveal(name_candidate_accumulation_model);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn merge_name_candidate_successes(
    candidates: [Option<ObjectName>; 12],
) -> (result: Result<Option<ObjectName>, NameError>)
    ensures result == name_candidate_accumulation_model(candidates@, 12),
{
    let mut index = 0usize;
    let mut accumulated: Result<Option<ObjectName>, NameError> = Ok(None);
    while index < candidates.len()
        invariant index <= candidates.len(),
            accumulated == name_candidate_accumulation_model(candidates@, index as int),
        decreases candidates.len() - index,
    {
        let candidate = candidates[index];
        accumulated = match accumulated {
            Err(error) => Err(error),
            Ok(current) => merge_name_candidate(current, candidate),
        };
        proof {
            lemma_name_candidate_accumulation_step(candidates@, index as int);
            reveal(name_candidate_merge_success_model);
            reveal(name_candidate_merge_model);
            assert(accumulated == name_candidate_accumulation_model(
                candidates@,
                (index + 1) as int,
            ));
        }
        index += 1;
    }
    accumulated
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
pub closed spec fn name_candidate_controller_result_model(
    recognized: bool,
    candidates: Seq<Option<ObjectName>>,
) -> Result<Option<ObjectName>, NameError> {
    match name_candidate_accumulation_model(candidates, 12) {
        Err(error) => Err(error),
        Ok(candidate) => name_finalization_model(recognized, candidate),
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn finalize_name_candidate_controller(
    recognized: bool,
    candidates: [Option<ObjectName>; 12],
    accumulated: Result<Option<ObjectName>, NameError>,
) -> (result: Result<Option<ObjectName>, NameError>)
    requires accumulated == name_candidate_accumulation_model(candidates@, 12),
    ensures result == name_candidate_controller_result_model(recognized, candidates@),
{
    let _ = candidates;
    match accumulated {
        Err(error) => Err(error),
        Ok(candidate) => finalize_name_classification(recognized, candidate),
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
// Keep the explicit match so Verus can carry the body's success contract into
// the loop invariant; Option::map hides that branch relation from the proof.
#[allow(clippy::manual_map)]
fn collect_name_body_outcomes(
    remainders: [Option<&[u8]>; 12],
    separator: u8,
) -> (outcomes: [Option<Result<ObjectName, NameError>>; 12])
    ensures forall|index: int| #![trigger outcomes@[index]]
        0 <= index < 12 ==>
        match (remainders@[index], outcomes@[index]) {
            (None, None) => true,
            (Some(rest), Some(outcome)) => {
                let (kind, short) = name_candidate_table_model()[index];
                body_outcome_result_model(rest@, kind, short, separator, outcome)
            },
            _ => false,
        },
{
    let candidates = name_candidate_table();
    let mut index = 0usize;
    let mut outcomes: [Option<Result<ObjectName, NameError>>; 12] = [None; 12];
    while index < candidates.len()
        invariant index <= candidates.len(),
            candidates@ == name_candidate_table_model(),
            forall|prior: int| #![trigger remainders@[prior], outcomes@[prior]]
                0 <= prior < index as int ==>
                match (remainders@[prior], outcomes@[prior]) {
                    (None, None) => true,
                    (Some(rest), Some(outcome)) => {
                        let (kind, short) = candidates@[prior];
                        body_outcome_result_model(rest@, kind, short, separator, outcome)
                    },
                    _ => false,
                },
        decreases candidates.len() - index,
    {
        let (kind, short) = candidates[index];
        let outcome = match remainders[index] {
            Some(rest) => Some(body(rest, kind, short, separator)),
            None => None,
        };
        outcomes[index] = outcome;
        proof {
            reveal(body_success_result_model);
            assert(candidates@[index as int] == (kind, short));
        }
        index += 1;
    }
    outcomes
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
closed spec fn successful_name_body_candidate_model(
    outcome: Option<Result<ObjectName, NameError>>,
) -> Option<ObjectName> {
    match outcome {
        Some(Ok(name)) => Some(name),
        None | Some(Err(_)) => None,
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn successful_name_body_candidate(
    outcome: Option<Result<ObjectName, NameError>>,
) -> (candidate: Option<ObjectName>)
    ensures candidate == successful_name_body_candidate_model(outcome),
{
    match outcome {
        Some(Ok(name)) => Some(name),
        None | Some(Err(_)) => None,
    }
}

//= docs/parsing-requirements.md#parse-name-001
//= type=implementation
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
fn successful_name_body_candidates(
    outcomes: [Option<Result<ObjectName, NameError>>; 12],
) -> (candidates: [Option<ObjectName>; 12])
    ensures forall|index: int| #![trigger candidates@[index]]
        0 <= index < 12 ==>
        candidates@[index] == successful_name_body_candidate_model(outcomes@[index]),
{
    let mut candidates = [None; 12];
    let mut index = 0usize;
    while index < outcomes.len()
        invariant index <= outcomes.len(),
            forall|prior: int| #![trigger candidates@[prior], outcomes@[prior]]
                0 <= prior < index as int ==>
                candidates@[prior] == successful_name_body_candidate_model(outcomes@[prior]),
        decreases outcomes.len() - index,
    {
        candidates[index] = successful_name_body_candidate(outcomes[index]);
        proof {
            reveal(successful_name_body_candidate_model);
            assert(candidates@[index as int] == successful_name_body_candidate_model(
                outcomes@[index as int],
            ));
        }
        index += 1;
    }
    candidates
}
}

#[cfg(test)]
mod tests {
    use super::{split_dot_field, take_body_suffix_fields, take_dot_field};

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn name_candidate_table_covers_each_kind_profile_once_in_order() {
        let candidates = super::name_candidate_table();
        let expected = [
            (super::ObjectKind::FullVolume, false),
            (super::ObjectKind::FullVolume, true),
            (super::ObjectKind::IncrementalVolume, false),
            (super::ObjectKind::IncrementalVolume, true),
            (super::ObjectKind::FullManifest, false),
            (super::ObjectKind::FullManifest, true),
            (super::ObjectKind::IncrementalManifest, false),
            (super::ObjectKind::IncrementalManifest, true),
            (super::ObjectKind::FullSignatures, false),
            (super::ObjectKind::FullSignatures, true),
            (super::ObjectKind::IncrementalSignatures, false),
            (super::ObjectKind::IncrementalSignatures, true),
        ];
        assert_eq!(candidates, expected);
        for i in 0..candidates.len() {
            for j in i + 1..candidates.len() {
                assert_ne!(candidates[i], candidates[j]);
            }
        }
        for i in (0..candidates.len()).step_by(2) {
            let pair = &candidates[i..i + 2];
            assert_eq!(pair[0].0, pair[1].0);
            assert!(!pair[0].1);
            assert!(pair[1].1);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-002
    //= type=test
    //# Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.
    fn candidate_controller_recognizes_each_exact_kind_profile_route() {
        let global = b"root-";
        let archive = b"arc/";
        let manifest = b"man/";
        let signatures = b"sig/";
        for (candidate_index, (kind, short)) in
            super::name_candidate_table().into_iter().enumerate()
        {
            let kind_prefix = super::kind_specific_prefix(kind, archive, manifest, signatures);
            let mut input = global.to_vec();
            input.extend_from_slice(kind_prefix);
            input.extend_from_slice(super::stem(kind, short));
            input.extend_from_slice(b"malformed-body");
            assert!(super::name_candidate_remainder(
                &input, global, archive, manifest, signatures, kind, short,
            )
            .is_some());
            let (remainders, recognized) =
                super::name_candidate_remainders(&input, global, archive, manifest, signatures);
            assert!(recognized);
            assert_eq!(
                remainders[candidate_index],
                Some(&input[input.len() - b"malformed-body".len()..])
            );

            let mut near_miss = input.clone();
            near_miss[global.len()] ^= 1;
            let (_, recognized) =
                super::name_candidate_remainders(&near_miss, global, archive, manifest, signatures);
            assert!(!recognized);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn body_outcome_collection_refines_all_kind_profile_routes() {
        let examples = [
            (
                super::ObjectKind::FullVolume,
                false,
                &b"20231114T221320Z.vol1.difftar"[..],
            ),
            (super::ObjectKind::FullVolume, true, &b"1.1.dt"[..]),
            (
                super::ObjectKind::IncrementalVolume,
                false,
                &b"20231114T221320Z.to.20231115T221320Z.vol1.difftar"[..],
            ),
            (super::ObjectKind::IncrementalVolume, true, &b"1.2.1.dt"[..]),
            (
                super::ObjectKind::FullManifest,
                false,
                &b"20231114T221320Z.manifest"[..],
            ),
            (super::ObjectKind::FullManifest, true, &b"1.m"[..]),
            (
                super::ObjectKind::IncrementalManifest,
                false,
                &b"20231114T221320Z.to.20231115T221320Z.manifest"[..],
            ),
            (super::ObjectKind::IncrementalManifest, true, &b"1.2.m"[..]),
            (
                super::ObjectKind::FullSignatures,
                false,
                &b"20231114T221320Z.sigtar"[..],
            ),
            (super::ObjectKind::FullSignatures, true, &b"1.st"[..]),
            (
                super::ObjectKind::IncrementalSignatures,
                false,
                &b"20231114T221320Z.to.20231115T221320Z.sigtar"[..],
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                true,
                &b"1.2.st"[..],
            ),
        ];
        for (expected_index, (kind, short, suffix)) in examples.into_iter().enumerate() {
            assert_eq!(super::name_candidate_table()[expected_index], (kind, short));
            let input = [super::stem(kind, short), suffix].concat();
            let (remainders, recognized) =
                super::name_candidate_remainders(&input, b"", b"", b"", b"");
            assert!(recognized, "{kind:?}, short={short}");
            let outcomes = super::collect_name_body_outcomes(remainders, b':');
            for (index, outcome) in outcomes.into_iter().enumerate() {
                if index == expected_index {
                    let name = match outcome {
                        Some(Ok(name)) => name,
                        other => panic!("expected successful outcome, got {other:?}"),
                    };
                    assert_eq!((name.kind, name.short), (kind, short));
                } else {
                    assert!(
                        !matches!(outcome, Some(Ok(_))),
                        "unexpected successful route {index} for {kind:?}"
                    );
                }
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn body_outcome_collection_refines_twelve_error_slots() {
        let malformed = [
            (
                super::ObjectKind::FullVolume,
                false,
                &b"bad.vol1.difftar"[..],
            ),
            (super::ObjectKind::FullVolume, true, &b"!.1.dt"[..]),
            (
                super::ObjectKind::IncrementalVolume,
                false,
                &b"bad.to.20231115T221320Z.vol1.difftar"[..],
            ),
            (super::ObjectKind::IncrementalVolume, true, &b"!.2.1.dt"[..]),
            (super::ObjectKind::FullManifest, false, &b"bad.manifest"[..]),
            (super::ObjectKind::FullManifest, true, &b"!.m"[..]),
            (
                super::ObjectKind::IncrementalManifest,
                false,
                &b"bad.to.20231115T221320Z.manifest"[..],
            ),
            (super::ObjectKind::IncrementalManifest, true, &b"!.2.m"[..]),
            (super::ObjectKind::FullSignatures, false, &b"bad.sigtar"[..]),
            (super::ObjectKind::FullSignatures, true, &b"!.st"[..]),
            (
                super::ObjectKind::IncrementalSignatures,
                false,
                &b"bad.to.20231115T221320Z.sigtar"[..],
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                true,
                &b"!.2.st"[..],
            ),
        ];
        for (kind, short, input) in malformed {
            let index = super::name_candidate_table()
                .iter()
                .position(|candidate| *candidate == (kind, short))
                .unwrap();
            let mut remainders = [None; 12];
            remainders[index] = Some(input);
            let outcomes = super::collect_name_body_outcomes(remainders, b':');
            assert_eq!(outcomes[index], Some(Err(super::NameError::Malformed)));
            for (other, outcome) in outcomes.iter().enumerate() {
                if other != index {
                    assert!(
                        outcome.is_none(),
                        "unexpected outcome in candidate slot {other}"
                    );
                }
            }
        }

        let mut remainders = [None; 12];
        remainders[0] = Some(&b"20231114T221320Z.vol1.difftar"[..]);
        let outcomes = super::collect_name_body_outcomes(remainders, b'.');
        assert_eq!(outcomes[0], Some(Err(super::NameError::Configuration)));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn body_error_matrix_classifies_all_kind_profile_routes() {
        let malformed = [
            (
                super::ObjectKind::FullVolume,
                false,
                &b"bad.vol1.difftar"[..],
            ),
            (super::ObjectKind::FullVolume, true, &b"!.1.dt"[..]),
            (
                super::ObjectKind::IncrementalVolume,
                false,
                &b"bad.to.20231115T221320Z.vol1.difftar"[..],
            ),
            (super::ObjectKind::IncrementalVolume, true, &b"!.2.1.dt"[..]),
            (super::ObjectKind::FullManifest, false, &b"bad.manifest"[..]),
            (super::ObjectKind::FullManifest, true, &b"!.m"[..]),
            (
                super::ObjectKind::IncrementalManifest,
                false,
                &b"bad.to.20231115T221320Z.manifest"[..],
            ),
            (super::ObjectKind::IncrementalManifest, true, &b"!.2.m"[..]),
            (super::ObjectKind::FullSignatures, false, &b"bad.sigtar"[..]),
            (super::ObjectKind::FullSignatures, true, &b"!.st"[..]),
            (
                super::ObjectKind::IncrementalSignatures,
                false,
                &b"bad.to.20231115T221320Z.sigtar"[..],
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                true,
                &b"!.2.st"[..],
            ),
        ];
        for (kind, short, input) in malformed {
            assert_eq!(
                super::body(input, kind, short, b':'),
                Err(super::NameError::Malformed),
                "{kind:?}, short={short}"
            );
        }

        for (kind, input) in [
            (
                super::ObjectKind::FullVolume,
                &b"20231114T221320Z.vol1.difftar"[..],
            ),
            (
                super::ObjectKind::IncrementalVolume,
                &b"20231114T221320Z.to.20231115T221320Z.vol1.difftar"[..],
            ),
            (
                super::ObjectKind::FullManifest,
                &b"20231114T221320Z.manifest"[..],
            ),
            (
                super::ObjectKind::IncrementalManifest,
                &b"20231114T221320Z.to.20231115T221320Z.manifest"[..],
            ),
            (
                super::ObjectKind::FullSignatures,
                &b"20231114T221320Z.sigtar"[..],
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                &b"20231114T221320Z.to.20231115T221320Z.sigtar"[..],
            ),
        ] {
            assert!(super::body(input, kind, false, b':').is_ok());
            let result = super::body(input, kind, false, b'.');
            assert!(
                matches!(
                    result,
                    Err(super::NameError::Configuration | super::NameError::Malformed)
                ),
                "unexpected error variant for {kind:?}: {result:?}"
            );
            if kind == super::ObjectKind::FullVolume {
                assert_eq!(result, Err(super::NameError::Configuration));
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_full_manifest_body_error_and_success_edge_matrix() {
        for (input, separator) in [
            (b"20231114T221320Z.manifest" as &[u8], b':'),
            (b"1970-01-01T00:00:00Z.manifest", b':'),
        ] {
            let name = super::long_full_manifest_body(input, separator).expect("valid body");
            assert_eq!(name.kind, super::ObjectKind::FullManifest, "{input:?}");
            assert_eq!(name.start, name.end, "{input:?}");
            assert!(name.volume.is_none(), "{input:?}");
            assert!(!name.short, "{input:?}");
        }

        let malformed: [(&[u8], u8); 8] = [
            (b"", b':'),
            (b"bad.manifest", b':'),
            (b"20231114T221320Z", b':'),
            (b"20231114T221320Z.bad", b':'),
            (b"20231114T221320Z.manifest.extra", b':'),
            (b"20231114T221320Z.manifest", b'\0'),
            (b"1970-01-01T00:00:00Z.manifest", b'\0'),
            (b"1970-01-01T00_00_00Z.manifest", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::long_full_manifest_body(input, separator),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_full_manifest_fields_refine_timestamp_and_suffix_profiles() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type ManifestSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [ManifestSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (timestamp, separator) in timestamps {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::FullManifest,
                    start: super::Timestamp(0),
                    end: super::Timestamp(0),
                    volume: None,
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_full_manifest_body_fields(
                        timestamp,
                        separator,
                        b"manifest",
                        first,
                        second,
                        None,
                    ),
                    Ok(expected),
                    "{timestamp:?} {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_full_manifest_body_raw_result_matrix() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type RawLongManifestProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawLongManifestProfile<'_>; 8] = [
            (b"", super::Envelope::Plain, false),
            (b".part", super::Envelope::Plain, true),
            (b".gz", super::Envelope::Gzip, false),
            (b".gpg", super::Envelope::OpenPgp, false),
            (b".p7m", super::Envelope::Pkcs7, false),
            (b".part.gz", super::Envelope::Gzip, true),
            (b".part.gpg", super::Envelope::OpenPgp, true),
            (b".part.p7m", super::Envelope::Pkcs7, true),
        ];

        for (timestamp_field, separator) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".manifest");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::body(&raw, super::ObjectKind::FullManifest, false, separator),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullManifest,
                        start: super::Timestamp(0),
                        end: super::Timestamp(0),
                        volume: None,
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_full_volume_fields_refine_timestamp_volume_and_suffix_profiles() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type VolumeSuffixProfile<'a> = (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [VolumeSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (timestamp, separator) in timestamps {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::FullVolume,
                    start: super::Timestamp(0),
                    end: super::Timestamp(0),
                    volume: Some(42),
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_full_volume_body_fields(
                        timestamp, separator, b"vol42", b"difftar", first, second, None,
                    ),
                    Ok(expected),
                    "{timestamp:?} {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_full_volume_body_raw_result_matrix() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type RawLongVolumeProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawLongVolumeProfile<'_>; 8] = [
            (b"", super::Envelope::Plain, false),
            (b".part", super::Envelope::Plain, true),
            (b".gz", super::Envelope::Gzip, false),
            (b".gpg", super::Envelope::OpenPgp, false),
            (b".p7m", super::Envelope::Pkcs7, false),
            (b".part.gz", super::Envelope::Gzip, true),
            (b".part.gpg", super::Envelope::OpenPgp, true),
            (b".part.p7m", super::Envelope::Pkcs7, true),
        ];

        for (timestamp_field, separator) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".vol42.difftar");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::body(&raw, super::ObjectKind::FullVolume, false, separator),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullVolume,
                        start: super::Timestamp(0),
                        end: super::Timestamp(0),
                        volume: Some(42),
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn long_full_volume_body_exact_error_edge_matrix() {
        let kind = super::ObjectKind::FullVolume;

        // The scanner has a complete three-field shape, so an invalid
        // separator reaches the field parser and takes timestamp-policy
        // precedence even when the timestamp or later fields are malformed.
        for raw in [
            &b"19700101T000000Z.vol42.difftar"[..],
            &b"bad.vol0.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b'.'),
                Err(super::NameError::Configuration),
                "{raw:?} with invalid separator"
            );
        }

        for raw in [
            &b"bad.vol42.difftar"[..],
            &b"19700101T000000Z.volx.difftar"[..],
            &b"19700101T000000Z.vol0.difftar"[..],
            &b"19700101T000000Z.vol42.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b':'),
                Err(super::NameError::Malformed),
                "{raw:?} with valid separator"
            );
        }

        assert_eq!(
            super::body(b"19700101T000000Z.vol42.difftar", kind, false, b':',),
            Ok(super::ObjectName {
                kind,
                start: super::Timestamp(0),
                end: super::Timestamp(0),
                volume: Some(42),
                envelope: super::Envelope::Plain,
                partial: false,
                short: false,
            }),
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_incremental_volume_fields_refine_interval_volume_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type VolumeSuffixProfile<'a> = (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [VolumeSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(0),
                    end: super::Timestamp(1),
                    volume: Some(42),
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_incremental_volume_body_fields(
                        start_field,
                        separator,
                        b"to",
                        end_field,
                        b"vol42",
                        b"difftar",
                        first,
                        second,
                        None,
                    ),
                    Ok(expected),
                    "{start_field:?} to {end_field:?} vol42 {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    //= docs/parsing-requirements.md#parse-gen-003
    //= type=test
    //# A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.
    fn long_incremental_volume_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type VolumeSuffixProfile<'a> = (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [VolumeSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.extend_from_slice(b".to.");
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".vol42.difftar");
                if let Some(first) = first {
                    raw.push(b'.');
                    raw.extend_from_slice(first);
                }
                if let Some(second) = second {
                    raw.push(b'.');
                    raw.extend_from_slice(second);
                }

                assert_eq!(
                    super::body(&raw, super::ObjectKind::IncrementalVolume, false, separator,),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalVolume,
                        start: super::Timestamp(0),
                        end: super::Timestamp(1),
                        volume: Some(42),
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn long_incremental_volume_body_exact_error_edge_matrix() {
        let kind = super::ObjectKind::IncrementalVolume;

        // Complete dot-field names reach timestamp policy with an invalid
        // separator, which takes precedence over interval and volume errors.
        for raw in [
            &b"19700101T000000Z.to.19700101T000001Z.vol42.difftar"[..],
            &b"bad.to.19700101T000001Z.volx.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b'.'),
                Err(super::NameError::Configuration),
                "{raw:?} with invalid separator"
            );
        }

        for raw in [
            &b"bad.to.19700101T000001Z.vol42.difftar"[..],
            &b"19700101T000000Z.to.19700101T000000Z.vol42.difftar"[..],
            &b"19700101T000001Z.to.19700101T000000Z.vol42.difftar"[..],
            &b"19700101T000000Z.to.19700101T000001Z.volx.difftar"[..],
            &b"19700101T000000Z.to.19700101T000001Z.vol42.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b':'),
                Err(super::NameError::Malformed),
                "{raw:?} with valid separator"
            );
        }

        assert_eq!(
            super::body(
                b"19700101T000000Z.to.19700101T000001Z.vol42.difftar",
                kind,
                false,
                b':',
            ),
            Ok(super::ObjectName {
                kind,
                start: super::Timestamp(0),
                end: super::Timestamp(1),
                volume: Some(42),
                envelope: super::Envelope::Plain,
                partial: false,
                short: false,
            }),
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_full_signatures_fields_refine_timestamp_and_suffix_profiles() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type SignatureSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [SignatureSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (timestamp, separator) in timestamps {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::FullSignatures,
                    start: super::Timestamp(0),
                    end: super::Timestamp(0),
                    volume: None,
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_full_signatures_body_fields(
                        timestamp, separator, b"sigtar", first, second, None,
                    ),
                    Ok(expected),
                    "{timestamp:?} {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_full_signatures_body_raw_result_matrix() {
        let timestamps: [(&[u8], u8); 3] = [
            (b"19700101T000000Z", b':'),
            (b"1970-01-01T00:00:00Z", b':'),
            (b"1970-01-01T01_00_00+01_00", b'_'),
        ];
        type RawLongSignaturesProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawLongSignaturesProfile<'_>; 8] = [
            (b"", super::Envelope::Plain, false),
            (b".part", super::Envelope::Plain, true),
            (b".gz", super::Envelope::Gzip, false),
            (b".gpg", super::Envelope::OpenPgp, false),
            (b".p7m", super::Envelope::Pkcs7, false),
            (b".part.gz", super::Envelope::Gzip, true),
            (b".part.gpg", super::Envelope::OpenPgp, true),
            (b".part.p7m", super::Envelope::Pkcs7, true),
        ];

        for (timestamp_field, separator) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".sigtar");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::body(&raw, super::ObjectKind::FullSignatures, false, separator),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullSignatures,
                        start: super::Timestamp(0),
                        end: super::Timestamp(0),
                        volume: None,
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn long_full_signatures_body_exact_error_edge_matrix() {
        let kind = super::ObjectKind::FullSignatures;

        // Both inputs have complete timestamp/trailer field structure, so the
        // invalid separator reaches timestamp parsing before other validation.
        for raw in [&b"19700101T000000Z.sigtar"[..], &b"bad.wrong"[..]] {
            assert_eq!(
                super::body(raw, kind, false, b'.'),
                Err(super::NameError::Configuration),
                "{raw:?} with invalid separator"
            );
        }

        for raw in [&b"bad.sigtar"[..], &b"19700101T000000Z.wrong"[..]] {
            assert_eq!(
                super::body(raw, kind, false, b':'),
                Err(super::NameError::Malformed),
                "{raw:?} with valid separator"
            );
        }

        assert_eq!(
            super::body(b"19700101T000000Z.sigtar", kind, false, b':'),
            Ok(super::ObjectName {
                kind,
                start: super::Timestamp(0),
                end: super::Timestamp(0),
                volume: None,
                envelope: super::Envelope::Plain,
                partial: false,
                short: false,
            }),
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_incremental_manifest_fields_refine_interval_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type ManifestSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [ManifestSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(0),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_incremental_manifest_body_fields(
                        start_field,
                        separator,
                        b"to",
                        end_field,
                        b"manifest",
                        first,
                        second,
                        None,
                    ),
                    Ok(expected),
                    "{start_field:?} to {end_field:?} {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_incremental_manifest_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type ManifestSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [ManifestSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.extend_from_slice(b".to.");
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".manifest");
                if let Some(first) = first {
                    raw.push(b'.');
                    raw.extend_from_slice(first);
                }
                if let Some(second) = second {
                    raw.push(b'.');
                    raw.extend_from_slice(second);
                }

                assert_eq!(
                    super::body(
                        &raw,
                        super::ObjectKind::IncrementalManifest,
                        false,
                        separator,
                    ),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalManifest,
                        start: super::Timestamp(0),
                        end: super::Timestamp(1),
                        volume: None,
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn long_incremental_manifest_body_exact_error_edge_matrix() {
        let kind = super::ObjectKind::IncrementalManifest;

        // Complete four-field names reach timestamp policy with an invalid
        // separator, which takes precedence over interval and trailer errors.
        for raw in [
            &b"19700101T000000Z.to.19700101T000001Z.manifest"[..],
            &b"bad.to.19700101T000001Z.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b'.'),
                Err(super::NameError::Configuration),
                "{raw:?} with invalid separator"
            );
        }

        for raw in [
            &b"bad.to.19700101T000001Z.manifest"[..],
            &b"19700101T000000Z.to.19700101T000000Z.manifest"[..],
            &b"19700101T000001Z.to.19700101T000000Z.manifest"[..],
            &b"19700101T000000Z.to.19700101T000001Z.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b':'),
                Err(super::NameError::Malformed),
                "{raw:?} with valid separator"
            );
        }

        assert_eq!(
            super::body(
                b"19700101T000000Z.to.19700101T000001Z.manifest",
                kind,
                false,
                b':',
            ),
            Ok(super::ObjectName {
                kind,
                start: super::Timestamp(0),
                end: super::Timestamp(1),
                volume: None,
                envelope: super::Envelope::Plain,
                partial: false,
                short: false,
            }),
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_incremental_signatures_fields_refine_interval_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type SignatureSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [SignatureSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(0),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope,
                    partial,
                    short: false,
                };
                assert_eq!(
                    super::long_incremental_signatures_body_fields(
                        start_field,
                        separator,
                        b"to",
                        end_field,
                        b"sigtar",
                        first,
                        second,
                        None,
                    ),
                    Ok(expected),
                    "{start_field:?} to {end_field:?} {first:?} {second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-004
    //= type=test
    //# The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    //= docs/parsing-requirements.md#parse-name-007
    //= type=test
    //# The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.
    fn long_incremental_signatures_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], u8); 3] = [
            (b"19700101T000000Z", b"19700101T000001Z", b':'),
            (b"1970-01-01T00:00:00Z", b"1970-01-01T00:00:01Z", b':'),
            (
                b"1970-01-01T01_00_00+01_00",
                b"1970-01-01T01_00_01+01_00",
                b'_',
            ),
        ];
        type SignatureSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [SignatureSuffixProfile<'_>; 8] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"part"), None, super::Envelope::Plain, true),
            (Some(b"gz"), None, super::Envelope::Gzip, false),
            (Some(b"gpg"), None, super::Envelope::OpenPgp, false),
            (Some(b"p7m"), None, super::Envelope::Pkcs7, false),
            (Some(b"part"), Some(b"gz"), super::Envelope::Gzip, true),
            (Some(b"part"), Some(b"gpg"), super::Envelope::OpenPgp, true),
            (Some(b"part"), Some(b"p7m"), super::Envelope::Pkcs7, true),
        ];

        for (start_field, end_field, separator) in intervals {
            for (first, second, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.extend_from_slice(b".to.");
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".sigtar");
                if let Some(first) = first {
                    raw.push(b'.');
                    raw.extend_from_slice(first);
                }
                if let Some(second) = second {
                    raw.push(b'.');
                    raw.extend_from_slice(second);
                }

                assert_eq!(
                    super::body(
                        &raw,
                        super::ObjectKind::IncrementalSignatures,
                        false,
                        separator,
                    ),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalSignatures,
                        start: super::Timestamp(0),
                        end: super::Timestamp(1),
                        volume: None,
                        envelope,
                        partial,
                        short: false,
                    }),
                    "{raw:?} with separator {separator:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn long_incremental_signatures_body_exact_error_edge_matrix() {
        let kind = super::ObjectKind::IncrementalSignatures;

        // Complete four-field names reach timestamp policy with an invalid
        // separator, which takes precedence over interval and trailer errors.
        for raw in [
            &b"19700101T000000Z.to.19700101T000001Z.sigtar"[..],
            &b"bad.to.19700101T000001Z.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b'.'),
                Err(super::NameError::Configuration),
                "{raw:?} with invalid separator"
            );
        }

        for raw in [
            &b"bad.to.19700101T000001Z.sigtar"[..],
            &b"19700101T000000Z.to.19700101T000000Z.sigtar"[..],
            &b"19700101T000001Z.to.19700101T000000Z.sigtar"[..],
            &b"19700101T000000Z.to.19700101T000001Z.wrong"[..],
        ] {
            assert_eq!(
                super::body(raw, kind, false, b':'),
                Err(super::NameError::Malformed),
                "{raw:?} with valid separator"
            );
        }

        assert_eq!(
            super::body(
                b"19700101T000000Z.to.19700101T000001Z.sigtar",
                kind,
                false,
                b':',
            ),
            Ok(super::ObjectName {
                kind,
                start: super::Timestamp(0),
                end: super::Timestamp(1),
                volume: None,
                envelope: super::Envelope::Plain,
                partial: false,
                short: false,
            }),
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn body_candidate_accumulation_ignores_errors_and_merges_successes_in_order() {
        let mut remainders: [Option<&[u8]>; 12] = [None; 12];
        remainders[0] = Some(b"malformed");
        let outcomes = super::collect_name_body_outcomes(remainders, b':');
        assert!(matches!(outcomes[0], Some(Err(_))));
        let accumulated =
            super::merge_name_candidate_successes(super::successful_name_body_candidates(outcomes));
        assert_eq!(accumulated, Ok(None));

        remainders[0] = Some(b"20231114T221320Z.vol1.difftar");
        let outcomes = super::collect_name_body_outcomes(remainders, b':');
        let candidates = super::successful_name_body_candidates(outcomes);
        let accumulated = super::merge_name_candidate_successes(candidates);
        let parsed = match candidates[0] {
            Some(name) => name,
            _ => panic!("valid long full-volume body must parse"),
        };
        assert_eq!(accumulated, Ok(Some(parsed)));

        remainders[1] = Some(b"1.1.dt");
        let outcomes = super::collect_name_body_outcomes(remainders, b':');
        let candidates = super::successful_name_body_candidates(outcomes);
        let accumulated = super::merge_name_candidate_successes(candidates);
        assert!(candidates[1].is_some());
        assert_eq!(accumulated, Err(super::NameError::Ambiguous));
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-009
    //= type=test
    //# Invalid configuration MUST return the configuration error before basename limit or malformed-name errors; valid configuration MUST proceed to the checks in PARSE-NAME-008.
    fn parse_name_final_result_composes_candidate_model() {
        let config = super::NameConfig::default();
        for input in [
            &b"unrelated"[..],
            &b"duplicity-full.bad.vol1.difftar"[..],
            &b"duplicity-full.20231114T221320Z.vol1.difftar"[..],
            &b"df.1.1.dt"[..],
            &b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar"[..],
            &b"di.1.2.1.dt"[..],
            &b"duplicity-full.20231114T221320Z.manifest"[..],
            &b"df.1.m"[..],
            &b"duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest"[..],
            &b"di.1.2.m"[..],
            &b"duplicity-full-signatures.20231114T221320Z.sigtar"[..],
            &b"dfs.1.st"[..],
            &b"duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar"[..],
            &b"dns.1.2.st"[..],
        ] {
            let direct = super::parse_name(input, config);
            let (remainders, recognized) = super::name_candidate_remainders(
                input,
                config.global_prefix,
                config.archive_prefix,
                config.manifest_prefix,
                config.signature_prefix,
            );
            let outcomes = super::collect_name_body_outcomes(remainders, config.time_separator);
            let candidates = super::successful_name_body_candidates(outcomes);
            let accumulated = super::merge_name_candidate_successes(candidates);
            let composed =
                super::finalize_name_candidate_controller(recognized, candidates, accumulated);
            assert_eq!(direct, composed, "input: {:?}", input);
        }

        let invalid_separator = super::NameConfig {
            time_separator: b'.',
            ..config
        };
        assert_eq!(
            super::parse_name(
                b"duplicity-full.20231114T221320Z.vol1.difftar",
                invalid_separator,
            ),
            Err(super::NameError::Configuration)
        );
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn stem_remainder_matches_exact_prefix_and_returns_borrowed_tail() {
        assert_eq!(
            super::stem_remainder(b"duplicity-full.manifest", b"duplicity-full."),
            Some(&b"manifest"[..])
        );
        assert_eq!(super::stem_remainder(b"df.", b"df."), Some(&b""[..]));
        assert_eq!(
            super::stem_remainder(b"anything", b""),
            Some(&b"anything"[..])
        );
        assert_eq!(super::stem_remainder(b"", b""), Some(&b""[..]));

        for (kind, short) in super::name_candidate_table() {
            let prefix = super::stem(kind, short);
            let input = [prefix, b"matched-suffix"].concat();
            assert_eq!(
                super::stem_remainder(&input, prefix),
                Some(&input[prefix.len()..]),
                "{kind:?}, short={short}"
            );
        }

        for (input, prefix) in [
            (&b"dg.rest"[..], &b"df."[..]),
            (&b"df"[..], &b"df."[..]),
            (&b"duplicity-fullXrest"[..], &b"duplicity-full."[..]),
            (&b""[..], &b"df."[..]),
        ] {
            assert_eq!(super::stem_remainder(input, prefix), None);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_signatures_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_full_signatures_route(kind, true),
                kind == super::ObjectKind::FullSignatures
            );
            assert!(!crate::name_dispatch::short_full_signatures_route(
                kind, false
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_full_signatures_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_full_signatures_route(kind, false),
                kind == super::ObjectKind::FullSignatures
            );
            assert!(!crate::name_dispatch::long_full_signatures_route(
                kind, true
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_manifest_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_full_manifest_route(kind, true),
                kind == super::ObjectKind::FullManifest
            );
            assert!(!crate::name_dispatch::short_full_manifest_route(
                kind, false
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_full_manifest_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_full_manifest_route(kind, false),
                kind == super::ObjectKind::FullManifest
            );
            assert!(!crate::name_dispatch::long_full_manifest_route(kind, true));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_incremental_manifest_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_incremental_manifest_route(kind, true),
                kind == super::ObjectKind::IncrementalManifest
            );
            assert!(!crate::name_dispatch::short_incremental_manifest_route(
                kind, false
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_incremental_manifest_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_incremental_manifest_route(kind, false),
                kind == super::ObjectKind::IncrementalManifest
            );
            assert!(!crate::name_dispatch::long_incremental_manifest_route(
                kind, true
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_incremental_signatures_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_incremental_signatures_route(kind, true),
                kind == super::ObjectKind::IncrementalSignatures
            );
            assert!(!crate::name_dispatch::short_incremental_signatures_route(
                kind, false
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_incremental_signatures_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_incremental_signatures_route(kind, false),
                kind == super::ObjectKind::IncrementalSignatures
            );
            assert!(!crate::name_dispatch::long_incremental_signatures_route(
                kind, true
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_volume_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_full_volume_route(kind, true),
                kind == super::ObjectKind::FullVolume
            );
            assert!(!crate::name_dispatch::short_full_volume_route(kind, false));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_full_volume_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_full_volume_route(kind, false),
                kind == super::ObjectKind::FullVolume
            );
            assert!(!crate::name_dispatch::long_full_volume_route(kind, true));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_incremental_volume_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::short_incremental_volume_route(kind, true),
                kind == super::ObjectKind::IncrementalVolume
            );
            assert!(!crate::name_dispatch::short_incremental_volume_route(
                kind, false
            ));
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn body_route_partition_covers_every_kind_and_profile_exactly_once() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            for short in [false, true] {
                assert_eq!(crate::name_dispatch::body_route_partition(kind, short), 1);
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn selected_body_route_matches_all_kind_and_profile_pairs() {
        use crate::name_dispatch::BodyRoute as R;
        let cases = [
            (super::ObjectKind::FullVolume, false, R::LongFullVolume),
            (super::ObjectKind::FullVolume, true, R::ShortFullVolume),
            (
                super::ObjectKind::IncrementalVolume,
                false,
                R::LongIncrementalVolume,
            ),
            (
                super::ObjectKind::IncrementalVolume,
                true,
                R::ShortIncrementalVolume,
            ),
            (super::ObjectKind::FullManifest, false, R::LongFullManifest),
            (super::ObjectKind::FullManifest, true, R::ShortFullManifest),
            (
                super::ObjectKind::IncrementalManifest,
                false,
                R::LongIncrementalManifest,
            ),
            (
                super::ObjectKind::IncrementalManifest,
                true,
                R::ShortIncrementalManifest,
            ),
            (
                super::ObjectKind::FullSignatures,
                false,
                R::LongFullSignatures,
            ),
            (
                super::ObjectKind::FullSignatures,
                true,
                R::ShortFullSignatures,
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                false,
                R::LongIncrementalSignatures,
            ),
            (
                super::ObjectKind::IncrementalSignatures,
                true,
                R::ShortIncrementalSignatures,
            ),
        ];
        for (kind, short, expected) in cases {
            assert_eq!(
                crate::name_dispatch::select_body_route(kind, short),
                expected
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn successful_body_results_preserve_selected_kind_and_profile() {
        let cases: [(super::ObjectKind, bool, &[u8]); 12] = [
            (
                super::ObjectKind::FullVolume,
                false,
                b"19700101T000000Z.vol1.difftar",
            ),
            (super::ObjectKind::FullVolume, true, b"1.1.dt"),
            (
                super::ObjectKind::IncrementalVolume,
                false,
                b"19700101T000000Z.to.19700101T000001Z.vol1.difftar",
            ),
            (super::ObjectKind::IncrementalVolume, true, b"1.2.1.dt"),
            (
                super::ObjectKind::FullManifest,
                false,
                b"19700101T000000Z.manifest",
            ),
            (super::ObjectKind::FullManifest, true, b"1.m"),
            (
                super::ObjectKind::IncrementalManifest,
                false,
                b"19700101T000000Z.to.19700101T000001Z.manifest",
            ),
            (super::ObjectKind::IncrementalManifest, true, b"1.2.m"),
            (
                super::ObjectKind::FullSignatures,
                false,
                b"19700101T000000Z.sigtar",
            ),
            (super::ObjectKind::FullSignatures, true, b"1.st"),
            (
                super::ObjectKind::IncrementalSignatures,
                false,
                b"19700101T000000Z.to.19700101T000001Z.sigtar",
            ),
            (super::ObjectKind::IncrementalSignatures, true, b"1.2.st"),
        ];

        for (kind, short, input) in cases {
            let name = super::body(input, kind, short, b':').expect("valid body");
            assert_eq!(name.kind, kind);
            assert_eq!(name.short, short);
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_volume_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8, super::ObjectName); 4] = [
            (
                b"1.1.dt",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: Some(1),
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            // The short FullVolume grammar does not consume the time separator.
            (
                b"1.1.dt",
                b'\0',
                super::ObjectName {
                    kind: super::ObjectKind::FullVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: Some(1),
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"z.z.dt.z",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullVolume,
                    start: super::Timestamp(35),
                    end: super::Timestamp(35),
                    volume: Some(35),
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"2.2.dt.p.g",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullVolume,
                    start: super::Timestamp(2),
                    end: super::Timestamp(2),
                    volume: Some(2),
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, separator, expected) in valid {
            let result = super::body(input, super::ObjectKind::FullVolume, true, separator);
            assert_eq!(
                result,
                Ok(expected),
                "{input:?} with separator {separator:?}"
            );
        }

        let malformed: [(&[u8], u8); 6] = [
            (b"", b':'),
            (b"!.1.dt", b':'),
            (b"1.!.dt", b':'),
            (b"1.1.bad", b':'),
            (b"1.1.dt.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.1.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(input, super::ObjectKind::FullVolume, true, separator),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn short_full_volume_body_raw_result_matrix() {
        let timestamps: [(&[u8], i128); 3] = [(b"1", 1), (b"z", 35), (b"10", 36)];
        type RawFullVolumeProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawFullVolumeProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (timestamp_field, timestamp) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".10.dt");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_full_volume_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullVolume,
                        start: super::Timestamp(timestamp),
                        end: super::Timestamp(timestamp),
                        volume: Some(36),
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_signatures_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8, super::ObjectName); 4] = [
            (
                b"1.st",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullSignatures,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            // The short FullSignatures grammar does not consume the time separator.
            (
                b"1.st",
                b'\0',
                super::ObjectName {
                    kind: super::ObjectKind::FullSignatures,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"z.st.z",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullSignatures,
                    start: super::Timestamp(35),
                    end: super::Timestamp(35),
                    volume: None,
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"2.st.p.g",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullSignatures,
                    start: super::Timestamp(2),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, separator, expected) in valid {
            let result = super::body(input, super::ObjectKind::FullSignatures, true, separator);
            assert_eq!(
                result,
                Ok(expected),
                "{input:?} with separator {separator:?}"
            );
        }

        let malformed: [(&[u8], u8); 5] = [
            (b"", b':'),
            (b"!.st", b':'),
            (b"1.bad", b':'),
            (b"1.st.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(input, super::ObjectKind::FullSignatures, true, separator),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn short_full_signatures_body_raw_result_matrix() {
        let timestamps: [(&[u8], i128); 3] = [(b"1", 1), (b"z", 35), (b"10", 36)];
        type RawSignaturesProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawSignaturesProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (timestamp_field, timestamp) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".st");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_full_signatures_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullSignatures,
                        start: super::Timestamp(timestamp),
                        end: super::Timestamp(timestamp),
                        volume: None,
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_full_manifest_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8, super::ObjectName); 4] = [
            (
                b"1.m",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullManifest,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            // The short FullManifest grammar does not consume the time separator.
            (
                b"1.m",
                b'\0',
                super::ObjectName {
                    kind: super::ObjectKind::FullManifest,
                    start: super::Timestamp(1),
                    end: super::Timestamp(1),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"z.m.z",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullManifest,
                    start: super::Timestamp(35),
                    end: super::Timestamp(35),
                    volume: None,
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"2.m.p.g",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::FullManifest,
                    start: super::Timestamp(2),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, separator, expected) in valid {
            let result = super::body(input, super::ObjectKind::FullManifest, true, separator);
            assert_eq!(
                result,
                Ok(expected),
                "{input:?} with separator {separator:?}"
            );
        }

        let malformed: [(&[u8], u8); 5] = [
            (b"", b':'),
            (b"!.m", b':'),
            (b"1.bad", b':'),
            (b"1.m.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(input, super::ObjectKind::FullManifest, true, separator),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    fn short_full_manifest_body_raw_result_matrix() {
        let timestamps: [(&[u8], i128); 3] = [(b"1", 1), (b"z", 35), (b"10", 36)];
        type RawManifestProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawManifestProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (timestamp_field, timestamp) in timestamps {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(timestamp_field);
                raw.extend_from_slice(b".m");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_full_manifest_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::FullManifest,
                        start: super::Timestamp(timestamp),
                        end: super::Timestamp(timestamp),
                        volume: None,
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_incremental_manifest_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8, super::ObjectName); 4] = [
            (
                b"1.2.m",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            // The short IncrementalManifest grammar does not consume the time separator.
            (
                b"1.2.m",
                b'\0',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"z.10.m.z",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(35),
                    end: super::Timestamp(36),
                    volume: None,
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"2.3.m.p.g",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(2),
                    end: super::Timestamp(3),
                    volume: None,
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, separator, expected) in valid {
            let result = super::body(
                input,
                super::ObjectKind::IncrementalManifest,
                true,
                separator,
            );
            assert_eq!(
                result,
                Ok(expected),
                "{input:?} with separator {separator:?}"
            );
        }

        let malformed: [(&[u8], u8); 7] = [
            (b"", b':'),
            (b"!.2.m", b':'),
            (b"1.!.m", b':'),
            (b"2.1.m", b':'),
            (b"1.2.bad", b':'),
            (b"1.2.m.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.2.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(
                    input,
                    super::ObjectKind::IncrementalManifest,
                    true,
                    separator,
                ),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_manifest_fields_refine_interval_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type ManifestSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [ManifestSuffixProfile<'_>; 6] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"p"), None, super::Envelope::Plain, true),
            (Some(b"z"), None, super::Envelope::Gzip, false),
            (Some(b"g"), None, super::Envelope::OpenPgp, false),
            (Some(b"p"), Some(b"z"), super::Envelope::Gzip, true),
            (Some(b"p"), Some(b"g"), super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalManifest,
                    start: super::Timestamp(start),
                    end: super::Timestamp(end),
                    volume: None,
                    envelope,
                    partial,
                    short: true,
                };
                assert_eq!(
                    super::short_incremental_manifest_body_fields(
                        start_field,
                        end_field,
                        b"m",
                        first,
                        second,
                    ),
                    Ok(expected),
                    "{start_field:?}.{end_field:?}.m.{first:?}.{second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_manifest_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type RawManifestProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawManifestProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.push(b'.');
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".m");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_incremental_manifest_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalManifest,
                        start: super::Timestamp(start),
                        end: super::Timestamp(end),
                        volume: None,
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn short_incremental_signatures_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8, super::ObjectName); 4] = [
            (
                b"1.2.st",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            // The short IncrementalSignatures grammar does not consume the time separator.
            (
                b"1.2.st",
                b'\0',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: None,
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"z.10.st.z",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(35),
                    end: super::Timestamp(36),
                    volume: None,
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"2.3.st.p.g",
                b':',
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(2),
                    end: super::Timestamp(3),
                    volume: None,
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, separator, expected) in valid {
            let result = super::body(
                input,
                super::ObjectKind::IncrementalSignatures,
                true,
                separator,
            );
            assert_eq!(
                result,
                Ok(expected),
                "{input:?} with separator {separator:?}"
            );
        }

        let malformed: [(&[u8], u8); 7] = [
            (b"", b':'),
            (b"!.2.st", b':'),
            (b"1.!.st", b':'),
            (b"2.1.st", b':'),
            (b"1.2.bad", b':'),
            (b"1.2.st.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.2.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(
                    input,
                    super::ObjectKind::IncrementalSignatures,
                    true,
                    separator,
                ),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_signatures_fields_refine_interval_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type SignatureSuffixProfile<'a> =
            (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [SignatureSuffixProfile<'_>; 6] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"p"), None, super::Envelope::Plain, true),
            (Some(b"z"), None, super::Envelope::Gzip, false),
            (Some(b"g"), None, super::Envelope::OpenPgp, false),
            (Some(b"p"), Some(b"z"), super::Envelope::Gzip, true),
            (Some(b"p"), Some(b"g"), super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalSignatures,
                    start: super::Timestamp(start),
                    end: super::Timestamp(end),
                    volume: None,
                    envelope,
                    partial,
                    short: true,
                };
                assert_eq!(
                    super::short_incremental_signatures_body_fields(
                        start_field,
                        end_field,
                        b"st",
                        first,
                        second,
                    ),
                    Ok(expected),
                    "{start_field:?}.{end_field:?}.st.{first:?}.{second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_signatures_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type RawSignatureProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawSignatureProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.push(b'.');
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".st");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_incremental_signatures_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalSignatures,
                        start: super::Timestamp(start),
                        end: super::Timestamp(end),
                        volume: None,
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-010
    //= type=test
    //# The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.
    fn short_incremental_volume_body_error_and_success_edge_matrix() {
        let valid: [(&[u8], u8); 2] = [
            (b"1.2.1.dt", b':'),
            // The short IncrementalVolume grammar does not consume the time separator.
            (b"1.2.1.dt", b'\0'),
        ];
        for (input, separator) in valid {
            let result = super::body(input, super::ObjectKind::IncrementalVolume, true, separator);
            assert!(result.is_ok(), "{input:?} with separator {separator:?}");
        }

        let malformed: [(&[u8], u8); 8] = [
            (b"", b':'),
            (b"!.2.1.dt", b':'),
            (b"1.!.1.dt", b':'),
            (b"1.2.0.dt", b':'),
            (b"2.1.1.dt", b':'),
            (b"1.2.1.bad", b':'),
            (b"1.2.1.dt.extra", b':'),
            // Invalid separator configuration does not affect the short route.
            (b"1.2.1.bad", b'\0'),
        ];
        for (input, separator) in malformed {
            assert_eq!(
                super::body(input, super::ObjectKind::IncrementalVolume, true, separator,),
                Err(super::NameError::Malformed),
                "{input:?} with separator {separator:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_volume_success_route_composes_interval_volume_and_trailer() {
        let profiles: [(&[u8], super::ObjectName); 7] = [
            (
                b"1.2.1.dt",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::Plain,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"1.2.1.dt.p",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::Plain,
                    partial: true,
                    short: true,
                },
            ),
            (
                b"1.2.1.dt.z",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::Gzip,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"1.2.1.dt.g",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::OpenPgp,
                    partial: false,
                    short: true,
                },
            ),
            (
                b"1.2.1.dt.p.z",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::Gzip,
                    partial: true,
                    short: true,
                },
            ),
            (
                b"1.2.1.dt.p.g",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(1),
                    end: super::Timestamp(2),
                    volume: Some(1),
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
            (
                b"2.z.3.dt.p.g",
                super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(2),
                    end: super::Timestamp(35),
                    volume: Some(3),
                    envelope: super::Envelope::OpenPgp,
                    partial: true,
                    short: true,
                },
            ),
        ];
        for (input, expected) in profiles {
            assert_eq!(
                super::short_incremental_volume_body(input, b':'),
                Ok(expected),
                "{input:?}"
            );
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_volume_fields_refine_interval_volume_and_suffix_profiles() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type VolumeSuffixProfile<'a> = (Option<&'a [u8]>, Option<&'a [u8]>, super::Envelope, bool);
        let profiles: [VolumeSuffixProfile<'_>; 6] = [
            (None, None, super::Envelope::Plain, false),
            (Some(b"p"), None, super::Envelope::Plain, true),
            (Some(b"z"), None, super::Envelope::Gzip, false),
            (Some(b"g"), None, super::Envelope::OpenPgp, false),
            (Some(b"p"), Some(b"z"), super::Envelope::Gzip, true),
            (Some(b"p"), Some(b"g"), super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (first, second, envelope, partial) in profiles {
                let expected = super::ObjectName {
                    kind: super::ObjectKind::IncrementalVolume,
                    start: super::Timestamp(start),
                    end: super::Timestamp(end),
                    volume: Some(36),
                    envelope,
                    partial,
                    short: true,
                };
                assert_eq!(
                    super::short_incremental_volume_body_fields(
                        start_field,
                        end_field,
                        b"10",
                        b"dt",
                        first,
                        second,
                    ),
                    Ok(expected),
                    "{start_field:?}.{end_field:?}.10.dt.{first:?}.{second:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-name-003
    //= type=test
    //# Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.
    //= docs/parsing-requirements.md#parse-name-006
    //= type=test
    //# An incremental set MUST have an end timestamp strictly later than its start timestamp.
    fn short_incremental_volume_body_raw_result_matrix() {
        let intervals: [(&[u8], &[u8], i128, i128); 3] = [
            (b"1", b"2", 1, 2),
            (b"z", b"10", 35, 36),
            (b"10", b"11", 36, 37),
        ];
        type RawVolumeProfile<'a> = (&'a [u8], super::Envelope, bool);
        let profiles: [RawVolumeProfile<'_>; 6] = [
            (b"", super::Envelope::Plain, false),
            (b".p", super::Envelope::Plain, true),
            (b".z", super::Envelope::Gzip, false),
            (b".g", super::Envelope::OpenPgp, false),
            (b".p.z", super::Envelope::Gzip, true),
            (b".p.g", super::Envelope::OpenPgp, true),
        ];

        for (start_field, end_field, start, end) in intervals {
            for (suffix, envelope, partial) in profiles {
                let mut raw = Vec::new();
                raw.extend_from_slice(start_field);
                raw.push(b'.');
                raw.extend_from_slice(end_field);
                raw.extend_from_slice(b".10.dt");
                raw.extend_from_slice(suffix);

                assert_eq!(
                    super::short_incremental_volume_body(&raw, b':'),
                    Ok(super::ObjectName {
                        kind: super::ObjectKind::IncrementalVolume,
                        start: super::Timestamp(start),
                        end: super::Timestamp(end),
                        volume: Some(36),
                        envelope,
                        partial,
                        short: true,
                    }),
                    "{raw:?}"
                );
            }
        }
    }

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    fn long_incremental_volume_dispatch_selects_only_its_profile() {
        let kinds = [
            super::ObjectKind::FullVolume,
            super::ObjectKind::IncrementalVolume,
            super::ObjectKind::FullManifest,
            super::ObjectKind::IncrementalManifest,
            super::ObjectKind::FullSignatures,
            super::ObjectKind::IncrementalSignatures,
        ];
        for kind in kinds {
            assert_eq!(
                crate::name_dispatch::long_incremental_volume_route(kind, false),
                kind == super::ObjectKind::IncrementalVolume
            );
            assert!(!crate::name_dispatch::long_incremental_volume_route(
                kind, true
            ));
        }
    }

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

    #[test]
    //= docs/parsing-requirements.md#parse-name-001
    //= type=test
    //# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
    //= docs/parsing-requirements.md#parse-gen-002
    //= type=test
    //# A successful parser MUST report a consumed input range wholly contained within the supplied input.
    fn body_suffix_framing_preserves_optional_slots_and_first_excess_field() {
        let mut remaining = None;
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (None, None, None, None)
        );
        assert_eq!(remaining, None);

        // An empty first suffix is still a field, and does not collapse the
        // framing of later suffix positions.
        let mut remaining = Some(&b""[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (Some(&b""[..]), None, None, None)
        );
        assert_eq!(remaining, None);

        // Exercise empty values independently in every optional suffix slot.
        let mut remaining = Some(&b"base."[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (Some(&b"base"[..]), Some(&b""[..]), None, None)
        );
        assert_eq!(remaining, None);

        let mut remaining = Some(&b"base.gz."[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (Some(&b"base"[..]), Some(&b"gz"[..]), Some(&b""[..]), None)
        );
        assert_eq!(remaining, None);

        let mut remaining = Some(&b"part.gz"[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (Some(&b"part"[..]), Some(&b"gz"[..]), None, None)
        );
        assert_eq!(remaining, None);

        let mut remaining = Some(&b"part.gz.extra."[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (
                Some(&b"part"[..]),
                Some(&b"gz"[..]),
                Some(&b"extra"[..]),
                Some(&b""[..]),
            )
        );
        assert_eq!(remaining, None);

        let mut remaining = Some(&b"part.gz.extra.trailing"[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (
                Some(&b"part"[..]),
                Some(&b"gz"[..]),
                Some(&b"extra"[..]),
                Some(&b"trailing"[..]),
            )
        );
        assert_eq!(remaining, None);

        let mut remaining = Some(&b"part.gz.extra.trailing.unconsumed"[..]);
        assert_eq!(
            take_body_suffix_fields(&mut remaining),
            (
                Some(&b"part"[..]),
                Some(&b"gz"[..]),
                Some(&b"extra"[..]),
                Some(&b"trailing"[..]),
            )
        );
        assert_eq!(remaining, Some(&b"unconsumed"[..]));
    }

    use super::{
        calendar_date_valid, calendar_day_valid, calendar_fields_to_unix_seconds,
        clock_fields_valid, compose_timestamp_fields, configured_prefix_remainder,
        days_before_year, decode_base36, decode_decimal, decode_long_interval,
        decode_long_timestamp_numeric_field, decode_long_trailer_suffix, decode_long_volume_number,
        decode_short_interval, decode_short_timestamp, decode_short_trailer,
        decode_short_volume_number, decode_zone_offset_token, finalize_name_classification,
        long_timestamp_field_bounds, long_timestamp_framing_valid, merge_name_candidate,
        parse_long_timestamp, stem, time_separator_valid, Envelope, LongTimestampField, NameError,
        ObjectKind, ObjectName, Timestamp, UNIX_EPOCH_DAYS_FROM_1901,
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
    fn unix_epoch_year_offset_matches_gregorian_boundaries() {
        assert_eq!(UNIX_EPOCH_DAYS_FROM_1901, 25_202);
        assert_eq!(days_before_year(1970), 25_202);
        assert!(calendar_date_valid(1904, 2, 29));
        // The date API starts in 1901, so this also verifies lower-bound rejection.
        assert!(!calendar_date_valid(1900, 2, 29));
        assert!(calendar_date_valid(2000, 2, 29));
        let day_before_leap_day = calendar_fields_to_unix_seconds(1904, 2, 28, 0, 0, 0, 0);
        let leap_day = calendar_fields_to_unix_seconds(1904, 2, 29, 0, 0, 0, 0);
        let day_after_leap_day = calendar_fields_to_unix_seconds(1904, 3, 1, 0, 0, 0, 0);
        assert_eq!(leap_day.0 - day_before_leap_day.0, 86_400);
        assert_eq!(day_after_leap_day.0 - leap_day.0, 86_400);
        let day_before_2000_leap_day = calendar_fields_to_unix_seconds(2000, 2, 28, 0, 0, 0, 0);
        let leap_day_2000 = calendar_fields_to_unix_seconds(2000, 2, 29, 0, 0, 0, 0);
        let day_after_2000_leap_day = calendar_fields_to_unix_seconds(2000, 3, 1, 0, 0, 0, 0);
        assert_eq!(leap_day_2000.0 - day_before_2000_leap_day.0, 86_400);
        assert_eq!(day_after_2000_leap_day.0 - leap_day_2000.0, 86_400);
        assert_eq!(
            calendar_fields_to_unix_seconds(1970, 1, 1, 0, 0, 0, 0),
            Timestamp(0)
        );
        assert_eq!(
            calendar_fields_to_unix_seconds(1969, 12, 31, 23, 59, 59, 0),
            Timestamp(-1)
        );
        assert_eq!(
            calendar_fields_to_unix_seconds(1970, 1, 2, 0, 0, 0, 0),
            Timestamp(86_400)
        );
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
