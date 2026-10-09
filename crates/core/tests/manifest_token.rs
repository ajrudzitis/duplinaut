use duplicity_core::manifest_token::{decode_token, TokenError};

//= docs/parsing-requirements.md#parse-man-002
//# The manifest token parser MUST decode a valid double-quoted token's `\xHH` escape to exactly the represented byte.
#[test]
fn escapes_preserve_every_byte_and_both_hex_cases() {
    for byte in 0u16..=255 {
        for encoded in [format!("\"\\x{byte:02x}\""), format!("\"\\x{byte:02X}\"")] {
            let (used, decoded) = decode_token(encoded.as_bytes(), 6).unwrap();
            assert_eq!(used, 6);
            assert_eq!(decoded, [byte as u8]);
        }
    }
}

//= docs/parsing-requirements.md#parse-man-003
//# The manifest token parser MUST reject incomplete escapes, non-hexadecimal escape digits, unsupported escape forms, and unterminated quoted tokens.
#[test]
fn malformed_quotes_and_escapes_are_rejected() {
    for invalid in [
        b"\"".as_slice(), b"\"abc", b"\"\\", b"\"\\x", b"\"\\x0", b"\"\\x00",
        b"\"\\xgg\"", b"\"\\x0g\"", b"\"\\n\"", b"\"\\X00\"", b"\"raw\0\"",
        b"\"raw\n\"", b"\"raw\r\"", b"'single'", b"", b"\\x00",
    ] {
        assert_eq!(decode_token(invalid, 1024), Err(TokenError::Invalid), "{invalid:?}");
    }
}

#[test]
fn quoted_and_unquoted_tokens_leave_delimiters_for_the_caller() {
    assert_eq!(decode_token(b"\"a b\t'\xff\" 17", 1024).unwrap(), (8, b"a b\t'\xff".to_vec()));
    assert_eq!(decode_token(b"raw-\xff/path\t12", 1024).unwrap(), (10, b"raw-\xff/path".to_vec()));
    assert_eq!(decode_token(b"\"\"", 2).unwrap(), (2, Vec::new()));
    assert_eq!(decode_token(b"abc", 2), Err(TokenError::Limit));
}
