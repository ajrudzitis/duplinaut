use duplicity_core::delta::{apply_delta, decode_command, DecodedCommand, DeltaError};

fn stream(command: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x72, 0x73, 0x02, 0x36];
    bytes.extend_from_slice(command);
    bytes.push(0);
    bytes
}

// Independent wire encoding for test vectors; widths come from the reviewed
// upstream opcode table, not from the generated implementation.
fn number(value: u64, width: usize) -> Vec<u8> {
    value.to_be_bytes()[8 - width..].to_vec()
}

//= docs/parsing-requirements.md#parse-delta-002
//= type=test
//# The delta parser MUST decode every nonreserved opcode according to the complete delta grammar above.
//= docs/parsing-requirements.md#parse-delta-003
//= type=test
//# A literal command MUST append exactly its indicated payload bytes to the reconstructed output.
#[test]
fn every_literal_opcode_decodes_exact_payload() {
    for length in 1..=64 {
        let data: Vec<u8> = (0..length).map(|index| index ^ 0xa5).collect();
        let mut command = vec![length];
        command.extend_from_slice(&data);
        assert_eq!(
            decode_command(&command),
            Ok((command.len(), DecodedCommand::Literal(&data)))
        );
        assert_eq!(
            apply_delta(&stream(&command), b"unused basis", 64),
            Ok(data)
        );
    }
    for (opcode, width) in [(0x41, 1), (0x42, 2), (0x43, 4), (0x44, 8)] {
        let mut command = vec![opcode];
        command.extend(number(3, width));
        command.extend_from_slice(b"new");
        assert_eq!(
            decode_command(&command),
            Ok((command.len(), DecodedCommand::Literal(b"new")))
        );
        assert_eq!(
            apply_delta(&stream(&command), b"old", 3),
            Ok(b"new".to_vec())
        );
    }
    assert_eq!(decode_command(&[0]), Ok((1, DecodedCommand::End)));
}

//= docs/parsing-requirements.md#parse-delta-004
//= type=test
//# A copy command MUST append exactly the indicated byte range from the fixed basis file.
#[test]
fn all_copy_widths_use_the_fixed_basis() {
    let basis = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for offset_width in 0..4 {
        for length_width in 0..4 {
            let mut command = vec![0x45 + offset_width * 4 + length_width];
            command.extend(number(5, [1, 2, 4, 8][offset_width as usize]));
            command.extend(number(7, [1, 2, 4, 8][length_width as usize]));
            assert_eq!(
                decode_command(&command),
                Ok((
                    command.len(),
                    DecodedCommand::Copy {
                        offset: 5,
                        length: 7
                    }
                ))
            );
            let mut repeated = command.clone();
            repeated.extend_from_slice(&command);
            assert_eq!(
                apply_delta(&stream(&repeated), basis, 14),
                Ok(b"56789AB56789AB".to_vec())
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-delta-001
//= type=test
//# The delta parser MUST require the reviewed delta magic before decoding commands.
//= docs/parsing-requirements.md#parse-delta-008
//= type=test
//# A complete delta stream MUST contain END immediately after its last complete command and contain no bytes after END.
#[test]
fn magic_and_exact_stream_termination_are_required() {
    assert_eq!(
        apply_delta(&[0x72, 0x73, 0x02, 0x36, 0], b"", 0),
        Ok(vec![])
    );
    for bytes in [vec![], vec![0x72, 0x73, 0x02]] {
        assert_eq!(apply_delta(&bytes, b"", 16), Err(DeltaError::Truncated));
    }
    assert_eq!(
        apply_delta(&[0, 0, 0, 0, 0], b"", 16),
        Err(DeltaError::BadMagic)
    );
    assert_eq!(
        apply_delta(&[0x72, 0x73, 0x02, 0x36], b"", 0),
        Err(DeltaError::Truncated)
    );
    assert_eq!(
        apply_delta(&[0x72, 0x73, 0x02, 0x36, 0, 0], b"", 0),
        Err(DeltaError::TrailingData)
    );
}

//= docs/parsing-requirements.md#parse-delta-005
//= type=test
//# A copy command MUST be rejected unless its offset and length describe a range wholly within the basis file.
#[test]
fn out_of_bounds_copies_and_output_exhaustion_fail() {
    assert_eq!(
        apply_delta(&stream(&[0x45, 0, 4]), b"abc", 10),
        Err(DeltaError::CopyOutOfBounds)
    );
    assert_eq!(
        apply_delta(&stream(&[0x45, 4, 0]), b"abc", 10),
        Err(DeltaError::CopyOutOfBounds)
    );
    assert_eq!(
        apply_delta(&stream(&[0x45, 0, 3]), b"abc", 2),
        Err(DeltaError::OutputLimit)
    );
    assert_eq!(
        apply_delta(&stream(&[3, b'a', b'b', b'c']), b"", 2),
        Err(DeltaError::OutputLimit)
    );
    let mut huge = vec![0x54];
    huge.extend(number(u64::MAX, 8));
    huge.extend(number(1, 8));
    assert_eq!(
        apply_delta(&stream(&huge), b"abc", 10),
        Err(DeltaError::CopyOutOfBounds)
    );
}

//= docs/parsing-requirements.md#parse-delta-006
//= type=test
//# Zero-length explicit literal and valid zero-length copy commands MUST consume their encoded arguments and leave reconstructed output unchanged.
#[test]
fn zero_length_commands_consume_arguments_without_ending() {
    for (opcode, width) in [(0x41, 1), (0x42, 2), (0x43, 4), (0x44, 8)] {
        let mut command = vec![opcode];
        command.extend(number(0, width));
        assert_eq!(
            decode_command(&command),
            Ok((1 + width, DecodedCommand::Literal(b"")))
        );
        command.extend_from_slice(&[1, b'x']);
        assert_eq!(apply_delta(&stream(&command), b"", 1), Ok(b"x".to_vec()));
    }
    assert_eq!(
        apply_delta(&stream(&[0x45, 3, 0, 1, b'x']), b"abc", 1),
        Ok(b"x".to_vec())
    );
}

//= docs/parsing-requirements.md#parse-delta-007
//= type=test
//# Reserved delta opcodes MUST produce an unsupported-command error.
#[test]
fn reserved_and_truncated_commands_fail() {
    for opcode in 0x55..=0xff {
        assert_eq!(
            decode_command(&[opcode]),
            Err(DeltaError::UnsupportedOpcode)
        );
    }
    let commands = [
        vec![3, b'a', b'b', b'c'],
        vec![0x44, 0, 0, 0, 0, 0, 0, 0, 3, b'a', b'b', b'c'],
        vec![0x54, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2],
    ];
    for command in commands {
        for length in 0..command.len() {
            assert_eq!(
                decode_command(&command[..length]),
                Err(DeltaError::Truncated)
            );
        }
    }
    let mut huge_literal = vec![0x44];
    huge_literal.extend(number(u64::MAX, 8));
    assert_eq!(decode_command(&huge_literal), Err(DeltaError::Truncated));
}
