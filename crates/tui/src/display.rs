//! Terminal representations never serve as backup path identities.
use duplicity_core::path::BackupPath;
use std::fmt::Write;

/// Escape every control/non-ASCII byte and the escape character itself. The
/// result is ASCII, reversible, and safe to pass as widget text (never ANSI).
pub fn bytes(bytes: &[u8]) -> String {
    let mut text = String::new();
    for &byte in bytes {
        match byte {
            b'\\' => text.push_str("\\\\"),
            0x20..=0x7e => text.push(char::from(byte)),
            _ => write!(text, "\\x{byte:02x}").expect("writing to String cannot fail"),
        }
    }
    text
}

pub fn path(path: &BackupPath) -> String {
    if path.components().is_empty() {
        return "/".into();
    }
    path.components()
        .iter()
        .map(|component| bytes(component))
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    //= docs/parsing-requirements.md#parse-path-001
    //= type=test
    //# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
    #[test]
    fn controls_invalid_utf8_and_literal_escapes_stay_distinct() {
        assert_eq!(bytes(b"\x1b[2J\n\r\t\xff"), "\\x1b[2J\\x0a\\x0d\\x09\\xff");
        assert_ne!(bytes(b"\xff"), bytes(b"\\xff"));
        let all = bytes(&(0..=255).collect::<Vec<_>>());
        assert!(all.bytes().all(|byte| (0x20..=0x7e).contains(&byte)));
        let original = BackupPath::parse(b"a/\xff\\name", false, 1024, 32).unwrap();
        assert_eq!(path(&original), "a/\\xff\\\\name");
        assert_eq!(original.components()[1], b"\xff\\name");
        assert_eq!(path(&BackupPath::parse(b"", true, 1024, 32).unwrap()), "/");
    }
}
