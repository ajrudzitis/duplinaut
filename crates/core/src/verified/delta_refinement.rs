// Authored semantic-refinement proofs; included in the generated module so
// closed generated constructor definitions are available to checked proofs.
verus! {
#[verifier::ext_equal]
pub enum CommandMeaning {
    End,
    Literal(Seq<u8>),
    Copy { offset: u64, length: u64 },
}

/// The unsigned big-endian value described by the wire grammar, independently
/// of Vest's parser implementation.
pub open spec fn wire_number(input: Seq<u8>, start: int, width: int) -> u64
    recommends
        0 <= start,
        width == 1 || width == 2 || width == 4 || width == 8,
        start + width <= input.len(),
{
    if width == 1 { input[start] as u64 }
    else if width == 2 { (input[start] as u64) << 8 | (input[start + 1] as u64) }
    else if width == 4 { (input[start] as u64) << 24 | (input[start + 1] as u64) << 16 | (input[start + 2] as u64) << 8 | (input[start + 3] as u64) }
    else if width == 8 { (input[start] as u64) << 56 | (input[start + 1] as u64) << 48 | (input[start + 2] as u64) << 40 | (input[start + 3] as u64) << 32 | (input[start + 4] as u64) << 24 | (input[start + 5] as u64) << 16 | (input[start + 6] as u64) << 8 | (input[start + 7] as u64) }
    else { 0 }
}

pub open spec fn literal_width(opcode: u8) -> int {
    match opcode { 0x41 => 1, 0x42 => 2, 0x43 => 4, 0x44 => 8, _ => 0 }
}

pub open spec fn copy_widths(opcode: u8) -> (int, int) {
    match opcode {
        0x45 => (1, 1),
        0x46 => (1, 2),
        0x47 => (1, 4),
        0x48 => (1, 8),
        0x49 => (2, 1),
        0x4a => (2, 2),
        0x4b => (2, 4),
        0x4c => (2, 8),
        0x4d => (4, 1),
        0x4e => (4, 2),
        0x4f => (4, 4),
        0x50 => (4, 8),
        0x51 => (8, 1),
        0x52 => (8, 2),
        0x53 => (8, 4),
        0x54 => (8, 8),
        _ => (0, 0),
    }
}

/// This specification is written from the reviewed opcode table. It does not
/// call the generated format's parser or serializer specifications.
pub open spec fn wire_command(input: Seq<u8>) -> Option<(int, CommandMeaning)> {
    if input.len() == 0 {
        None
    } else {
        let opcode = input[0];
        if opcode == 0 {
            Some((1, CommandMeaning::End))
        } else if opcode <= 0x40 {
            let end = 1 + opcode as int;
            if end <= input.len() {
                Some((end, CommandMeaning::Literal(input.subrange(1, end))))
            } else { None }
        } else if opcode <= 0x44 {
            let width = literal_width(opcode);
            let start = 1 + width;
            if start > input.len() { None } else {
                let length = wire_number(input, 1, width) as int;
                if start + length <= input.len() && length <= usize::MAX as int {
                    Some((start + length, CommandMeaning::Literal(input.subrange(start, start + length))))
                } else { None }
            }
        } else if opcode <= 0x54 {
            let widths = copy_widths(opcode);
            let end = 1 + widths.0 + widths.1;
            if end <= input.len() {
                Some((end, CommandMeaning::Copy {
                    offset: wire_number(input, 1, widths.0),
                    length: wire_number(input, 1 + widths.0, widths.1),
                }))
            } else { None }
        } else { None }
    }
}

pub open spec fn generated_meaning(command: CommandSpec) -> CommandMeaning {
    match command.body {
        CommandBodySpec::Variant1(_) => CommandMeaning::End,
        CommandBodySpec::Variant2(data) => CommandMeaning::Literal(data),
        CommandBodySpec::Variant3(value) => CommandMeaning::Literal(value.data),
        CommandBodySpec::Variant4(value) => CommandMeaning::Literal(value.data),
        CommandBodySpec::Variant5(value) => CommandMeaning::Literal(value.data),
        CommandBodySpec::Variant6(value) => CommandMeaning::Literal(value.data),
        CommandBodySpec::Variant7(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant8(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant9(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant10(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant11(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant12(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant13(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant14(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant15(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant16(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant17(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant18(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant19(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant20(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant21(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Variant22(value) => CommandMeaning::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBodySpec::Default(_) => CommandMeaning::End,
    }
}

pub open spec fn normalized_generated_parse(input: Seq<u8>) -> Option<(int, CommandMeaning)> {
    match CommandFmt.spec_parse(input) {
        Some((n, value)) => Some((n, generated_meaning(value))),
        None => None,
    }
}

//= docs/parsing-requirements.md#parse-delta-002
//# The delta parser MUST decode every nonreserved opcode according to the complete delta grammar above.
pub proof fn lemma_command_refinement(input: Seq<u8>)
    requires input.len() <= usize::MAX as int,
    ensures
        normalized_generated_parse(input) == wire_command(input),
        CommandFmt.spec_parse(input) is Some ==>
            !(CommandFmt.spec_parse(input).unwrap().1.body is Default),
{
    reveal(<CommandFmt as SpecParser>::spec_parse);
    reveal(<CommandBodyFmt as SpecParser>::spec_parse);
    reveal(CommandBodyFmt::spec);
    reveal(CommandBodyFmt::opcode_spec);
    reveal(CommandSpec::from_structural);
    reveal(CommandBodySpec::from_structural);
    reveal(<Literal1Fmt as SpecParser>::spec_parse);
    reveal(Literal1Spec::from_structural);
    reveal(<Literal2Fmt as SpecParser>::spec_parse);
    reveal(Literal2Spec::from_structural);
    reveal(<Literal4Fmt as SpecParser>::spec_parse);
    reveal(Literal4Spec::from_structural);
    reveal(<Literal8Fmt as SpecParser>::spec_parse);
    reveal(Literal8Spec::from_structural);
    reveal(<Copy11Fmt as SpecParser>::spec_parse);
    reveal(Copy11Spec::from_structural);
    reveal(<Copy12Fmt as SpecParser>::spec_parse);
    reveal(Copy12Spec::from_structural);
    reveal(<Copy14Fmt as SpecParser>::spec_parse);
    reveal(Copy14Spec::from_structural);
    reveal(<Copy18Fmt as SpecParser>::spec_parse);
    reveal(Copy18Spec::from_structural);
    reveal(<Copy21Fmt as SpecParser>::spec_parse);
    reveal(Copy21Spec::from_structural);
    reveal(<Copy22Fmt as SpecParser>::spec_parse);
    reveal(Copy22Spec::from_structural);
    reveal(<Copy24Fmt as SpecParser>::spec_parse);
    reveal(Copy24Spec::from_structural);
    reveal(<Copy28Fmt as SpecParser>::spec_parse);
    reveal(Copy28Spec::from_structural);
    reveal(<Copy41Fmt as SpecParser>::spec_parse);
    reveal(Copy41Spec::from_structural);
    reveal(<Copy42Fmt as SpecParser>::spec_parse);
    reveal(Copy42Spec::from_structural);
    reveal(<Copy44Fmt as SpecParser>::spec_parse);
    reveal(Copy44Spec::from_structural);
    reveal(<Copy48Fmt as SpecParser>::spec_parse);
    reveal(Copy48Spec::from_structural);
    reveal(<Copy81Fmt as SpecParser>::spec_parse);
    reveal(Copy81Spec::from_structural);
    reveal(<Copy82Fmt as SpecParser>::spec_parse);
    reveal(Copy82Spec::from_structural);
    reveal(<Copy84Fmt as SpecParser>::spec_parse);
    reveal(Copy84Spec::from_structural);
    reveal(<Copy88Fmt as SpecParser>::spec_parse);
    reveal(Copy88Spec::from_structural);
    if input.len() > 0 {
        match input[0] {
            0 => { assert(normalized_generated_parse(input) == wire_command(input)); },
            1 => { },
            2 => { },
            3 => { },
            4 => { },
            5 => { },
            6 => { },
            7 => { },
            8 => { },
            9 => { },
            10 => { },
            11 => { },
            12 => { },
            13 => { },
            14 => { },
            15 => { },
            16 => { },
            17 => { },
            18 => { },
            19 => { },
            20 => { },
            21 => { },
            22 => { },
            23 => { },
            24 => { },
            25 => { },
            26 => { },
            27 => { },
            28 => { },
            29 => { },
            30 => { },
            31 => { },
            32 => { },
            33 => { },
            34 => { },
            35 => { },
            36 => { },
            37 => { },
            38 => { },
            39 => { },
            40 => { },
            41 => { },
            42 => { },
            43 => { },
            44 => { },
            45 => { },
            46 => { },
            47 => { },
            48 => { },
            49 => { },
            50 => { },
            51 => { },
            52 => { },
            53 => { },
            54 => { },
            55 => { },
            56 => { },
            57 => { },
            58 => { },
            59 => { },
            60 => { },
            61 => { },
            62 => { },
            63 => { },
            64 => { },
            65 => { },
            66 => { },
            67 => { },
            68 => { },
            69 => { },
            70 => { },
            71 => { },
            72 => { },
            73 => { },
            74 => { },
            75 => { },
            76 => { },
            77 => { },
            78 => { },
            79 => { },
            80 => { },
            81 => { },
            82 => { },
            83 => { },
            84 => { },
            _ => { },
        }
    }
}

/// Complete stream interpretation against the immutable original basis.
/// Each recursive step consumes an opcode even when its payload length is zero.
pub open spec fn apply_from(
    bytes: Seq<u8>, basis: Seq<u8>, limit: int, cursor: int, output: Seq<u8>,
) -> Option<Seq<u8>>
    decreases bytes.len() - cursor,
{
    if cursor < 0 || cursor > bytes.len() || output.len() > limit { None }
    else {
        match wire_command(bytes.subrange(cursor, bytes.len() as int)) {
            None => None,
            Some((consumed, command)) => {
                if consumed <= 0 || consumed > bytes.len() - cursor { None }
                else {
                    let next = cursor + consumed;
                    match command {
                        CommandMeaning::End => {
                            if next == bytes.len() { Some(output) } else { None }
                        },
                        CommandMeaning::Literal(data) => {
                            if data.len() > limit - output.len() { None }
                            else { apply_from(bytes, basis, limit, next, output + data) }
                        },
                        CommandMeaning::Copy { offset, length } => {
                            if offset as int > basis.len()
                                || length as int > basis.len() - offset as int
                                || length as int > limit - output.len()
                            { None }
                            else {
                                apply_from(bytes, basis, limit, next,
                                    output + basis.subrange(offset as int, offset as int + length as int))
                            }
                        },
                    }
                }
            },
        }
    }
}

pub open spec fn delta_has_magic(bytes: Seq<u8>) -> bool {
    bytes.len() >= 4
        && bytes[0] == 0x72 && bytes[1] == 0x73 && bytes[2] == 0x02 && bytes[3] == 0x36
}

pub open spec fn apply_model(bytes: Seq<u8>, basis: Seq<u8>, limit: int) -> Option<Seq<u8>> {
    if delta_has_magic(bytes) { apply_from(bytes, basis, limit, 4, seq![]) } else { None }
}

pub proof fn lemma_magic_refinement(bytes: Seq<u8>)
    ensures
        (DeltaMagicFmt.spec_parse(bytes) is Some) == delta_has_magic(bytes),
        DeltaMagicFmt.spec_parse(bytes) is Some ==> DeltaMagicFmt.spec_parse(bytes).unwrap().0 == 4,
{
    reveal(<DeltaMagicFmt as SpecParser>::spec_parse);
}
}
