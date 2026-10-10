//! Independent byte grammar and refinement facts for manifest tokens.
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use crate::generated::manifest_token::*;
#[cfg(verus_keep_ghost)]
use vest_lib::{combinators::{Star, Const, U8}, core::{spec::*, proof::*}};

verus! {
pub open spec fn is_hex(b: u8) -> bool {
    48 <= b <= 57 || 65 <= b <= 70 || 97 <= b <= 102
}
pub open spec fn hex_spec(b: u8) -> u8 {
    if 48 <= b <= 57 { (b as int - 48) as u8 }
    else if 65 <= b <= 70 { (b as int - 65 + 10) as u8 }
    else { (b as int - 97 + 10) as u8 }
}
pub open spec fn quoted_plain(b: u8) -> bool {
    b != 0 && b != 10 && b != 13 && b != 34 && b != 92
}
pub open spec fn unquoted_plain(b: u8) -> bool {
    b != 0 && !(9 <= b <= 13) && b != 32 && b != 34 && b != 39 && b != 92
}
pub open spec fn atom_value(atom: QuotedAtomSpec) -> u8 {
    match atom {
        QuotedAtomSpec::Plain(b) => b,
        QuotedAtomSpec::Escaped(pair) =>
            (16 * hex_spec(pair.high) as int + hex_spec(pair.low) as int) as u8,
    }
}
pub open spec fn atom_values(atoms: Seq<QuotedAtomSpec>) -> Seq<u8> {
    atoms.map_values(|atom: QuotedAtomSpec| atom_value(atom))
}
pub open spec fn valid_atom(atom: QuotedAtomSpec) -> bool {
    match atom {
        QuotedAtomSpec::Plain(b) => quoted_plain(b),
        QuotedAtomSpec::Escaped(pair) => is_hex(pair.high) && is_hex(pair.low),
    }
}

/// One quoted atom, expressed only through the reviewed byte grammar.
pub open spec fn wire_atom(input: Seq<u8>) -> Option<(int, u8)> {
    if input.len() == 0 { None }
    else if quoted_plain(input[0]) { Some((1, input[0])) }
    else if input.len() >= 4 && input[0] == 92 && input[1] == 120
        && is_hex(input[2]) && is_hex(input[3]) {
        Some((4, (16 * hex_spec(input[2]) as int + hex_spec(input[3]) as int) as u8))
    } else { None }
}
pub open spec fn wire_atoms(input: Seq<u8>) -> (int, Seq<u8>)
    decreases input.len(),
{
    match wire_atom(input) {
        Some((n, b)) => {
            let rest = wire_atoms(input.skip(n));
            (n + rest.0, seq![b] + rest.1)
        },
        None => (0, seq![]),
    }
}
pub open spec fn wire_unquoted(input: Seq<u8>) -> (int, Seq<u8>)
    decreases input.len(),
{
    if input.len() > 0 && unquoted_plain(input[0]) {
        let rest = wire_unquoted(input.skip(1));
        (1 + rest.0, seq![input[0]] + rest.1)
    } else { (0, seq![]) }
}
pub open spec fn wire_quoted(input: Seq<u8>) -> Option<(int, Seq<u8>)> {
    if input.len() == 0 || input[0] != 34 { None }
    else {
        let body = wire_atoms(input.skip(1));
        if 0 <= body.0 && 1 + body.0 < input.len() && input[1 + body.0] == 34 {
            Some((2 + body.0, body.1))
        } else { None }
    }
}
pub open spec fn wire_token(input: Seq<u8>) -> Option<(int, Seq<u8>)> {
    if input.len() == 0 { None }
    else if input[0] == 34 { wire_quoted(input) }
    else {
        let token = wire_unquoted(input);
        if token.0 == 0 { None } else { Some(token) }
    }
}

pub proof fn lemma_atom_refinement(input: Seq<u8>)
    ensures
        match QuotedAtomFmt.spec_parse(input) {
            Some((n, atom)) => wire_atom(input) == Some((n, atom_value(atom)))
                && valid_atom(atom) && 1 <= n <= input.len(),
            None => wire_atom(input) is None,
        },
{
    reveal(<QuotedAtomFmt as SpecParser>::spec_parse);
    reveal(<QuotedByteFmt as SpecParser>::spec_parse);
    reveal(<EscapeFmt as SpecParser>::spec_parse);
    reveal(<EscapeAnonFmt as SpecParser>::spec_parse);
    reveal(<HexDigitFmt as SpecParser>::spec_parse);
    reveal(QuotedAtomSpec::from_structural);
    reveal(EscapeAnonSpec::from_structural);
}

pub proof fn lemma_atoms_refinement(input: Seq<u8>)
    ensures
        wire_atoms(input).0 == Star(QuotedAtomFmt).parse_rec(input).0,
        wire_atoms(input).1 == atom_values(Star(QuotedAtomFmt).parse_rec(input).1),
        0 <= wire_atoms(input).0 <= input.len(),
        wire_atoms(input).1.len() <= wire_atoms(input).0,
        forall|i: int| 0 <= i < Star(QuotedAtomFmt).parse_rec(input).1.len()
            ==> valid_atom(#[trigger] Star(QuotedAtomFmt).parse_rec(input).1[i]),
    decreases input.len(),
{
    lemma_atom_refinement(input);
    reveal_with_fuel(wire_atoms, 1);
    reveal_with_fuel(Star::<_>::parse_rec, 1);
    if let Some((n, atom)) = QuotedAtomFmt.spec_parse(input) {
        lemma_atoms_refinement(input.skip(n));
        let rest = Star(QuotedAtomFmt).parse_rec(input.skip(n)).1;
        assert(atom_values(seq![atom] + rest) =~= seq![atom_value(atom)] + atom_values(rest));
    }
}

pub proof fn lemma_unquoted_refinement(input: Seq<u8>)
    ensures
        wire_unquoted(input) == Star(UnquotedByteFmt).parse_rec(input),
        0 <= wire_unquoted(input).0 <= input.len(),
        wire_unquoted(input).1.len() == wire_unquoted(input).0,
    decreases input.len(),
{
    reveal(<UnquotedByteFmt as SpecParser>::spec_parse);
    reveal_with_fuel(wire_unquoted, 1);
    reveal_with_fuel(Star::<_>::parse_rec, 1);
    if input.len() > 0 && unquoted_plain(input[0]) {
        lemma_unquoted_refinement(input.skip(1));
    }
}

pub proof fn lemma_quoted_refinement(input: Seq<u8>)
    ensures
        match ManifestTokenFmt.spec_parse(input) {
            Some((n, token)) => wire_quoted(input) == Some((n, atom_values(token.atoms)))
                && 2 <= n <= input.len() && token.atoms.len() <= n
                && forall|i: int| 0 <= i < token.atoms.len() ==> valid_atom(#[trigger] token.atoms[i]),
            None => wire_quoted(input) is None,
        },
{
    reveal(<ManifestTokenFmt as SpecParser>::spec_parse);
    reveal(<QuotedTokenFmt as SpecParser>::spec_parse);
    reveal(QuotedTokenSpec::from_structural);
    reveal(<Star<_> as SpecParser>::spec_parse);
    if input.len() > 0 && input[0] == 34 { lemma_atoms_refinement(input.skip(1)); }
}

/// Generated ordered-choice code repeats this predicate as a guard. This fact
/// proves that a successful refined-byte parse already satisfies that guard.
pub broadcast proof fn lemma_quoted_byte_constraint(input: Seq<u8>)
    ensures
        (#[trigger] QuotedByteFmt.spec_parse(input)) is Some ==>
            quoted_plain(QuotedByteFmt.spec_parse(input).unwrap().1),
{
    reveal(<QuotedByteFmt as SpecParser>::spec_parse);
}

pub broadcast proof fn lemma_byte_escape_disjoint(fmt: QuotedByteFmt)
    ensures #[trigger] disjoint_domains(fmt, EscapeFmt),
{
    reveal(disjoint_domains);
    reveal(<QuotedByteFmt as SpecParser>::spec_parse);
    reveal(<EscapeFmt as SpecParser>::spec_parse);
    assert forall|input: Seq<u8>| #![auto]
        fmt.spec_parse(input) is Some && EscapeFmt.spec_parse(input) is Some implies false by { }
}

pub broadcast proof fn lemma_atom_closing_disjoint(fmt: QuotedAtomFmt)
    ensures #[trigger] disjoint_domains(fmt, Const(U8, 34u8)),
{
    reveal(disjoint_domains);
    assert forall|input: Seq<u8>| #![auto]
        fmt.spec_parse(input) is Some && Const(U8, 34u8).spec_parse(input) is Some implies false by {
        lemma_atom_refinement(input);
    }
}

pub broadcast group group_token_facts {
    lemma_quoted_byte_constraint,
    lemma_byte_escape_disjoint,
    lemma_atom_closing_disjoint,
}
}
