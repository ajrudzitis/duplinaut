use duplicity_core::path::{BackupPath, PathError};
fn parse(bytes: &[u8]) -> Result<BackupPath, PathError> { BackupPath::parse(bytes, false, 1024, 32) }

//= docs/parsing-requirements.md#parse-path-001
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
#[test]
fn identity_and_order_use_byte_components() {
    let raw = parse(b"raw-\xff/a").unwrap();
    assert_eq!(raw.components(), &[b"raw-\xff".to_vec(), b"a".to_vec()]);
    assert_eq!(raw.to_string(), "raw-\\xff/a");
    assert!(parse(b"a/b").unwrap() < parse(b"a!/b").unwrap());
    assert_ne!(parse(b"raw-\\xff/a").unwrap(), raw);
    assert_ne!(parse(b"raw-\\xff/a").unwrap().to_string(), raw.to_string());
}

//= docs/parsing-requirements.md#parse-path-002
//# A restore path MUST be relative to the backup root and contain no NUL, empty interior component, dot component, or parent component after format-specific root decoding.
#[test]
fn reject_escape_and_ambiguous_components() {
    for bytes in [b"/a".as_slice(), b"a//b", b"a/", b"a/./b", b"a/../b", b"..", b"a\0b"] {
        assert!(parse(bytes).is_err(), "{bytes:?}");
    }
    assert!(parse(b"").unwrap().components().is_empty());
    assert!(parse(b".").unwrap().components().is_empty());
    assert!(BackupPath::parse(b"./", true, 2, 1).unwrap().components().is_empty());
    assert_eq!(BackupPath::parse(b"a/", true, 2, 1).unwrap(), parse(b"a").unwrap());
    assert!(BackupPath::parse(b"a//", true, 3, 2).is_err());
    assert_eq!(BackupPath::parse(b"ab", false, 1, 1), Err(PathError::ByteLimit));
    assert_eq!(BackupPath::parse(b"a/b", false, 3, 1), Err(PathError::ComponentLimit));
}
