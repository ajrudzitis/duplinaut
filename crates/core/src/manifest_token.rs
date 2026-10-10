//! Byte-preserving token decoding for the reviewed manifest grammar.
// Explicit length and byte-range predicates match the checked arithmetic model.
#![allow(clippy::len_zero, clippy::manual_range_contains)]
use crate::generated::manifest_token::{ManifestTokenFmt, QuotedAtom, UnquotedByteFmt};
use vest_lib::combinators::Star;
use vest_lib::core::exec::parser::Parser;
#[cfg(verus_keep_ghost)]
use vest_lib::core::{spec::*, proof::*};
#[cfg(verus_keep_ghost)]
use crate::verified::token_refinement::*;
use vstd::prelude::*;

verus! {
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenError { Invalid, Limit }

fn hex_value(byte: u8) -> (result: u8)
    requires is_hex(byte),
    ensures result == hex_spec(byte), result <= 15,
{
    if byte >= b'0' && byte <= b'9' { byte - b'0' }
    else if byte >= b'A' && byte <= b'F' { byte - b'A' + 10 }
    else { byte - b'a' + 10 }
}

/// Decode one token. Callers validate the following delimiter and path semantics.
/// `max_encoded` bounds input before the generated parser allocates atom storage.
//= docs/parsing-requirements.md#parse-man-002
//# The manifest token parser MUST decode a valid double-quoted token's `\xHH` escape to exactly the represented byte.
//= docs/parsing-requirements.md#parse-man-003
//# The manifest token parser MUST reject incomplete escapes, non-hexadecimal escape digits, unsupported escape forms, and unterminated quoted tokens.
pub fn decode_token(bytes: &[u8], max_encoded: usize) -> (result: Result<(usize, Vec<u8>), TokenError>)
    ensures
        match result {
            Ok((consumed, decoded)) => bytes.len() <= max_encoded
                && wire_token(bytes@) == Some((consumed as int, decoded@))
                && 1 <= consumed <= bytes.len() && decoded.len() <= consumed,
            Err(TokenError::Limit) => bytes.len() > max_encoded,
            Err(TokenError::Invalid) => bytes.len() <= max_encoded && wire_token(bytes@) is None,
        },
{
    proof {
        lemma_quoted_refinement(bytes@);
        lemma_unquoted_refinement(bytes@);
        reveal(<Star<_> as SpecParser>::spec_parse);
    }
    if bytes.len() > max_encoded { return Err(TokenError::Limit); }
    if bytes.len() == 0 { return Err(TokenError::Invalid); }
    let mut output: Vec<u8> = Vec::new();
    if bytes[0] != b'"' {
        //= docs/parsing-requirements.md#assure-vest-002
        //# Every direct-combinator fallback MUST document the DSL limitation or suitability reason and its affected requirement IDs.
        // Vest 0.2.1 emits RepeatTillEnd in the spec but Star in executable
        // parsing for Vec followed by Nothing. Use explicit Star for prefix
        // repetition; see the core README and PARSE-MAN-002/003.
        let (consumed, atoms) = match Star(UnquotedByteFmt).parse(&bytes) {
            Ok(parsed) => parsed,
            Err(_) => return Err(TokenError::Invalid),
        };
        if consumed == 0 { return Err(TokenError::Invalid); }
        let mut index = 0;
        while index < atoms.len()
            invariant index <= atoms.len(), output@ == atoms@.take(index as int),
            decreases atoms.len() - index,
        {
            output.push(atoms[index]);
            assert(output@ =~= atoms@.take(index as int + 1));
            index += 1;
        }
        return Ok((consumed, output));
    }
    let (consumed, token) = match ManifestTokenFmt.parse(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => return Err(TokenError::Invalid),
    };
    let ghost original_token = token.deep_view();
    proof {
        reveal(<crate::generated::manifest_token::QuotedToken as DeepView>::deep_view);
        reveal(<QuotedAtom as DeepView>::deep_view);
        reveal(<crate::generated::manifest_token::EscapeAnon as DeepView>::deep_view);
    }
    let mut index = 0;
    while index < token.atoms.len()
        invariant
            index <= token.atoms.len(),
            original_token.atoms == token.atoms.deep_view(),
            forall|i: int| 0 <= i < original_token.atoms.len() ==> valid_atom(#[trigger] original_token.atoms[i]),
            output@ == atom_values(original_token.atoms.take(index as int)),
        decreases token.atoms.len() - index,
    {
        proof {
            reveal(<QuotedAtom as DeepView>::deep_view);
            reveal(<crate::generated::manifest_token::EscapeAnon as DeepView>::deep_view);
            assert(original_token.atoms[index as int] == token.atoms@[index as int].deep_view());
            assert(valid_atom(token.atoms@[index as int].deep_view()));
        }
        match &token.atoms[index] {
            QuotedAtom::Plain(byte) => output.push(*byte),
            QuotedAtom::Escaped(pair) => output.push(hex_value(pair.high) * 16 + hex_value(pair.low)),
        }
        assert(output@ =~= atom_values(original_token.atoms.take(index as int + 1)));
        index += 1;
    }
    Ok((consumed, output))
}
}
