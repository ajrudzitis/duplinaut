//! Independent adversarial review of the production command decoder.
use duplicity_core::delta::{apply_delta, decode_command, DecodedCommand, DeltaError};

// Widths are transcribed from the reviewed upstream table, rather than inferred
// by calling the implementation under test.
const COPY_WIDTHS: [(u8, usize, usize); 16] = [
    (0x45, 1, 1), (0x46, 1, 2), (0x47, 1, 4), (0x48, 1, 8),
    (0x49, 2, 1), (0x4a, 2, 2), (0x4b, 2, 4), (0x4c, 2, 8),
    (0x4d, 4, 1), (0x4e, 4, 2), (0x4f, 4, 4), (0x50, 4, 8),
    (0x51, 8, 1), (0x52, 8, 2), (0x53, 8, 4), (0x54, 8, 8),
];

fn append_number(bytes: &mut Vec<u8>, value: u64, width: usize) {
    bytes.extend_from_slice(&value.to_be_bytes()[8 - width..]);
}

fn delta(commands: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x72, 0x73, 0x02, 0x36];
    bytes.extend_from_slice(commands);
    bytes.push(0);
    bytes
}

fn asymmetric_value(width: usize) -> u64 {
    match width {
        1 => 0x91,
        2 => 0x1234,
        4 => 0x1234_5678,
        8 => 0x1234_5678_90ab_cdef,
        _ => unreachable!(),
    }
}

//= docs/parsing-requirements.md#parse-delta-002
//= type=test
//# The delta parser MUST decode every nonreserved opcode according to the complete delta grammar above.
#[test]
fn every_copy_width_preserves_high_bits_and_byte_order() {
    for (opcode, offset_width, length_width) in COPY_WIDTHS {
        let offset = asymmetric_value(offset_width);
        let length = asymmetric_value(length_width) ^ 0x55;
        let mut command = vec![opcode];
        append_number(&mut command, offset, offset_width);
        append_number(&mut command, length, length_width);
        assert_eq!(
            decode_command(&command),
            Ok((command.len(), DecodedCommand::Copy { offset, length })),
            "opcode {opcode:#x}",
        );
    }
}

//= docs/parsing-requirements.md#parse-gen-001
//= type=test
//# A parser MUST return a structured error for malformed or truncated required input.
#[test]
fn every_command_rejects_every_proper_truncated_prefix() {
    let mut commands = Vec::new();
    for length in 1u8..=64 {
        let mut bytes = vec![length];
        bytes.extend((0..length).map(|n| n ^ 0xa5));
        commands.push(bytes);
    }
    for (opcode, width) in [(0x41, 1), (0x42, 2), (0x43, 4), (0x44, 8)] {
        let mut bytes = vec![opcode];
        append_number(&mut bytes, 3, width);
        bytes.extend_from_slice(b"abc");
        commands.push(bytes);
    }
    for (opcode, offset_width, length_width) in COPY_WIDTHS {
        let mut bytes = vec![opcode];
        append_number(&mut bytes, 2, offset_width);
        append_number(&mut bytes, 3, length_width);
        commands.push(bytes);
    }
    for command in commands {
        for end in 0..command.len() {
            assert_eq!(
                decode_command(&command[..end]),
                Err(DeltaError::Truncated),
                "opcode {:#x}, prefix {end}", command[0],
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-delta-005
//= type=test
//# A copy command MUST be rejected unless its offset and length describe a range wholly within the basis file.
#[test]
fn oversized_u64_fields_cannot_wrap_into_small_valid_ranges() {
    let mut command = vec![0x54];
    append_number(&mut command, 0x1_0000_0002, 8);
    append_number(&mut command, 1, 8);
    assert_eq!(apply_delta(&delta(&command), b"abcd", 4), Err(DeltaError::CopyOutOfBounds));

    let mut command = vec![0x54];
    append_number(&mut command, 0, 8);
    append_number(&mut command, 0x1_0000_0002, 8);
    assert_eq!(apply_delta(&delta(&command), b"abcd", 4), Err(DeltaError::CopyOutOfBounds));

    let mut command = vec![0x44];
    append_number(&mut command, 0x1_0000_0002, 8);
    command.extend_from_slice(b"ab");
    assert_eq!(decode_command(&command), Err(DeltaError::Truncated));
}

//= docs/parsing-requirements.md#parse-delta-004
//= type=test
//# A copy command MUST append exactly the indicated byte range from the fixed basis file.
#[test]
fn literals_do_not_change_the_basis_for_later_copies() {
    assert_eq!(
        apply_delta(&delta(&[2, b'z', b'z', 0x45, 0, 3]), b"abcdef", 5),
        Ok(b"zzabc".to_vec()),
    );
}

//= docs/parsing-requirements.md#parse-delta-006
//= type=test
//# Zero-length explicit literal and valid zero-length copy commands MUST consume their encoded arguments and leave reconstructed output unchanged.
#[test]
fn all_copy_widths_accept_zero_length_at_the_end_of_the_basis() {
    for (opcode, offset_width, length_width) in COPY_WIDTHS {
        let mut command = vec![opcode];
        append_number(&mut command, 4, offset_width);
        append_number(&mut command, 0, length_width);
        assert_eq!(apply_delta(&delta(&command), b"abcd", 0), Ok(vec![]));
    }
    // Reaching the budget does not prevent parsing subsequent no-op commands.
    assert_eq!(apply_delta(&delta(&[1, b'x', 0x41, 0, 0x45, 0, 0]), b"", 1), Ok(vec![b'x']));
    assert_eq!(apply_delta(&delta(&[1, b'x', 1, b'y']), b"", 1), Err(DeltaError::OutputLimit));
}

//= docs/parsing-requirements.md#parse-gen-002
//= type=test
//# A successful parser MUST report a consumed input range wholly contained within the supplied input.
#[test]
fn command_consumption_leaves_the_following_command_untouched() {
    assert_eq!(decode_command(&[1, b'x', 0x55]), Ok((2, DecodedCommand::Literal(b"x"))));
    assert_eq!(decode_command(&[0, 0x55]), Ok((1, DecodedCommand::End)));
}
