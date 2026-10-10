//! Native librsync delta decoding using the Vest DSL-generated parser.
// Retain explicit arithmetic and error branches in the checked kernel rather
// than introducing trait-dispatched helpers into the verification surface.
#![allow(clippy::len_zero, clippy::question_mark, clippy::assign_op_pattern)]
use crate::generated::delta::{CommandBody, CommandFmt, DeltaMagicFmt};
#[cfg(verus_keep_ghost)]
use crate::generated::delta::{CommandMeaning, generated_meaning, wire_command, lemma_command_refinement,
    apply_from, apply_model, delta_has_magic, lemma_magic_refinement};
use vest_lib::core::exec::parser::Parser;
#[cfg(verus_keep_ghost)]
use vest_lib::core::{proof::*, spec::*};
use vstd::prelude::*;

verus! {
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaError {
    BadMagic,
    Truncated,
    UnsupportedOpcode,
    TrailingData,
    CopyOutOfBounds,
    OutputLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedCommand<'a> {
    End,
    Literal(&'a [u8]),
    Copy { offset: u64, length: u64 },
}

pub open spec fn decoded_meaning(command: DecodedCommand) -> CommandMeaning {
    match command {
        DecodedCommand::End => CommandMeaning::End,
        DecodedCommand::Literal(data) => CommandMeaning::Literal(data@),
        DecodedCommand::Copy { offset, length } => CommandMeaning::Copy { offset, length },
    }
}

/// Parse one command, returning its exact consumed byte count.
pub fn decode_command<'a>(bytes: &'a [u8]) -> (result: Result<(usize, DecodedCommand<'a>), DeltaError>)
    ensures
        match result {
            Ok((consumed, command)) => {
                &&& 1 <= consumed <= bytes.len()
                &&& wire_command(bytes@) == Some((consumed as int, decoded_meaning(command)))
            },
            Err(error) => {
                &&& wire_command(bytes@) is None
                &&& if bytes.len() > 0 && bytes[0] > 0x54 {
                    error == DeltaError::UnsupportedOpcode
                } else { error == DeltaError::Truncated }
            },
        },
{
    proof {
        lemma_command_refinement(bytes@);
        CommandFmt.lemma_parse_safe(bytes@);
        CommandFmt.lemma_productive(bytes@);
    }
    if bytes.len() == 0 { return Err(DeltaError::Truncated); }
    if bytes[0] > 0x54 { return Err(DeltaError::UnsupportedOpcode); }
    let (consumed, command) = match CommandFmt.parse(&bytes) {
        Ok(value) => value,
        Err(_) => return Err(DeltaError::Truncated),
    };
    let ghost original_command = command.deep_view();
    proof {
        reveal(<crate::generated::delta::Command as DeepView>::deep_view);
        reveal(<CommandBody as DeepView>::deep_view);
        reveal(<crate::generated::delta::Literal1 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Literal2 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Literal4 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Literal8 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy11 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy12 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy14 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy18 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy21 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy22 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy24 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy28 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy41 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy42 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy44 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy48 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy81 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy82 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy84 as DeepView>::deep_view);
        reveal(<crate::generated::delta::Copy88 as DeepView>::deep_view);
    }
    let body = match command.body {
        CommandBody::Variant1(_) => DecodedCommand::End,
        CommandBody::Variant2(data) => DecodedCommand::Literal(data),
        CommandBody::Variant3(value) => DecodedCommand::Literal(value.data),
        CommandBody::Variant4(value) => DecodedCommand::Literal(value.data),
        CommandBody::Variant5(value) => DecodedCommand::Literal(value.data),
        CommandBody::Variant6(value) => DecodedCommand::Literal(value.data),
        CommandBody::Variant7(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant8(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant9(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant10(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length },
        CommandBody::Variant11(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant12(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant13(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant14(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length },
        CommandBody::Variant15(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant16(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant17(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length as u64 },
        CommandBody::Variant18(value) => DecodedCommand::Copy { offset: value.offset as u64, length: value.length },
        CommandBody::Variant19(value) => DecodedCommand::Copy { offset: value.offset, length: value.length as u64 },
        CommandBody::Variant20(value) => DecodedCommand::Copy { offset: value.offset, length: value.length as u64 },
        CommandBody::Variant21(value) => DecodedCommand::Copy { offset: value.offset, length: value.length as u64 },
        CommandBody::Variant22(value) => DecodedCommand::Copy { offset: value.offset, length: value.length },
        CommandBody::Default(_) => return Err(DeltaError::UnsupportedOpcode),
    };
    assert(decoded_meaning(body) == generated_meaning(original_command));
    Ok((consumed, body))
}

/// Decode and apply a complete delta against an immutable basis.
/// Output is bounded independently of the encoded delta length.
//= docs/parsing-requirements.md#parse-delta-003
//# A literal command MUST append exactly its indicated payload bytes to the reconstructed output.
//= docs/parsing-requirements.md#parse-delta-004
//# A copy command MUST append exactly the indicated byte range from the fixed basis file.
//= docs/parsing-requirements.md#parse-delta-005
//# A copy command MUST be rejected unless its offset and length describe a range wholly within the basis file.
//= docs/parsing-requirements.md#parse-delta-006
//# Zero-length explicit literal and valid zero-length copy commands MUST consume their encoded arguments and leave reconstructed output unchanged.
//= docs/parsing-requirements.md#parse-delta-008
//# A complete delta stream MUST contain END immediately after its last complete command and contain no bytes after END.
pub fn apply_delta(bytes: &[u8], basis: &[u8], max_output: usize) -> (result: Result<Vec<u8>, DeltaError>)
    ensures
        match result {
            Ok(output) => apply_model(bytes@, basis@, max_output as int) == Some(output@)
                && output.len() <= max_output,
            Err(_) => apply_model(bytes@, basis@, max_output as int) is None,
        },
{
    proof {
        lemma_magic_refinement(bytes@);
        reveal_with_fuel(apply_model, 1);
    }
    if bytes.len() < 4 { return Err(DeltaError::Truncated); }
    let (header, _) = match DeltaMagicFmt.parse(&bytes) {
        Ok(value) => value,
        Err(_) => return Err(DeltaError::BadMagic),
    };
    let mut cursor = header;
    let mut output: Vec<u8> = Vec::new();
    loop
        invariant
            4 <= cursor <= bytes.len(),
            output.len() <= max_output,
            delta_has_magic(bytes@),
            apply_model(bytes@, basis@, max_output as int)
                == apply_from(bytes@, basis@, max_output as int, cursor as int, output@),
        decreases bytes.len() - cursor,
    {
        let ghost prior_cursor = cursor as int;
        let ghost prior_output = output@;
        proof { reveal_with_fuel(apply_from, 1); }
        let rest = vstd::slice::slice_subrange(bytes, cursor, bytes.len());
        let (consumed, command) = match decode_command(rest) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        cursor = cursor + consumed;
        match command {
            DecodedCommand::End => {
                if cursor != bytes.len() { return Err(DeltaError::TrailingData); }
                return Ok(output);
            },
            DecodedCommand::Literal(data) => {
                if data.len() > max_output - output.len() { return Err(DeltaError::OutputLimit); }
                proof {
                    assert(apply_from(bytes@, basis@, max_output as int, prior_cursor, prior_output)
                        == apply_from(bytes@, basis@, max_output as int, cursor as int, prior_output + data@));
                }
                let mut index = 0;
                while index < data.len()
                    invariant
                        index <= data.len(),
                        prior_output.len() + data.len() <= max_output,
                        output@ == prior_output + data@.take(index as int),
                    decreases data.len() - index,
                {
                    output.push(data[index]);
                    proof {
                        assert(output@ =~= prior_output + data@.take(index as int + 1));
                    }
                    index = index + 1;
                }
                assert(output@ =~= prior_output + data@);
            },
            DecodedCommand::Copy { offset, length } => {
                if offset > basis.len() as u64 { return Err(DeltaError::CopyOutOfBounds); }
                if length > basis.len() as u64 - offset { return Err(DeltaError::CopyOutOfBounds); }
                if length > (max_output - output.len()) as u64 { return Err(DeltaError::OutputLimit); }
                let start = offset as usize;
                let count = length as usize;
                proof {
                    assert(start as int == offset as int);
                    assert(count as int == length as int);
                    assert(apply_from(bytes@, basis@, max_output as int, prior_cursor, prior_output)
                        == apply_from(bytes@, basis@, max_output as int, cursor as int,
                            prior_output + basis@.subrange(start as int, start as int + count as int)));
                }
                let mut index = 0;
                while index < count
                    invariant
                        index <= count,
                        start + count <= basis.len(),
                        prior_output.len() + count <= max_output,
                        output@ == prior_output + basis@.subrange(start as int, start as int + index as int),
                    decreases count - index,
                {
                    output.push(basis[start + index]);
                    proof {
                        assert(output@ =~= prior_output + basis@.subrange(start as int, start as int + index as int + 1));
                    }
                    index = index + 1;
                }
            },
        }
    }
}
}

impl std::fmt::Display for DeltaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "librsync delta: {self:?}")
    }
}
impl std::error::Error for DeltaError {}
