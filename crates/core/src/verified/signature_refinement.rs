// Independent published signature wire model, refined by the DSL parser.
verus! {
pub open spec fn signature_word(input: Seq<u8>, offset: int) -> u32
    recommends 0 <= offset, offset + 4 <= input.len(),
{
    (input[offset] as u32) << 24 | (input[offset + 1] as u32) << 16
        | (input[offset + 2] as u32) << 8 | input[offset + 3] as u32
}

pub open spec fn signature_magic_supported(magic: u32) -> bool {
    magic == 0x72730136 || magic == 0x72730137
        || magic == 0x72730146 || magic == 0x72730147
}

pub open spec fn signature_wire_header(input: Seq<u8>) -> Option<(u32, u32, u32)> {
    if input.len() < 12 { None }
    else if !signature_magic_supported(signature_word(input, 0)) { None }
    else { Some((signature_word(input, 0), signature_word(input, 4), signature_word(input, 8))) }
}

pub proof fn lemma_signature_header_refinement(input: Seq<u8>)
    ensures
        match SignatureHeaderFmt.spec_parse(input) {
            Some((consumed, header)) => consumed == 12
                && signature_wire_header(input) == Some((header.magic, header.block_length, header.strong_length)),
            None => signature_wire_header(input) is None,
        },
{
    reveal(<SignatureHeaderFmt as SpecParser>::spec_parse);
    reveal(<SignatureMagicFmt as SpecParser>::spec_parse);
    reveal(SignatureHeaderSpec::from_structural);
    if input.len() >= 12 {
        assert(input.subrange(4, input.len() as int)[0] == input[4]);
        assert(input.subrange(4, input.len() as int).subrange(4, input.len() as int - 4)[0] == input[8]);
    }
}
}
