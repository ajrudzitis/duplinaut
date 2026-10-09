//! Byte-preserving token decoding for the reviewed manifest grammar.
use crate::generated::manifest_token::{ManifestTokenFmt, QuotedAtom, UnquotedByteFmt};
use vest_lib::combinators::Star;
use vest_lib::core::exec::parser::Parser;
use vstd::prelude::*;

verus! {
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenError { Invalid, Limit }

fn hex_value(byte: u8) -> u8 {
    if byte >= b'0' && byte <= b'9' { byte - b'0' }
    else if byte >= b'A' && byte <= b'F' { byte - b'A' + 10 }
    else { byte - b'a' + 10 }
}

/// Decode one token. Callers validate the following delimiter and path semantics.
/// `max_encoded` bounds input before the generated parser allocates atom storage.
pub fn decode_token(bytes: &[u8], max_encoded: usize) -> Result<(usize, Vec<u8>), TokenError> {
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
        while index < atoms.len() {
            output.push(atoms[index]);
            index += 1;
        }
        return Ok((consumed, output));
    }
    let (consumed, token) = match ManifestTokenFmt.parse(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => return Err(TokenError::Invalid),
    };
    let mut index = 0;
    while index < token.atoms.len() {
        match &token.atoms[index] {
            QuotedAtom::Plain(byte) => output.push(*byte),
            QuotedAtom::Escaped(pair) => output.push(hex_value(pair.high) * 16 + hex_value(pair.low)),
        }
        index += 1;
    }
    Ok((consumed, output))
}
}
