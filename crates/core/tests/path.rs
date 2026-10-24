use duplicity_core::path::{BackupPath, PathError};
fn parse(bytes: &[u8]) -> Result<BackupPath, PathError> {
    BackupPath::parse(bytes, false, 1024, 32)
}

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
    for bytes in [
        b"/a".as_slice(),
        b"a//b",
        b"a/",
        b"a/./b",
        b"a/../b",
        b"..",
        b"a\0b",
    ] {
        assert!(parse(bytes).is_err(), "{bytes:?}");
    }
    assert!(parse(b"").unwrap().components().is_empty());
    assert!(parse(b".").unwrap().components().is_empty());
    assert!(BackupPath::parse(b"./", true, 2, 1)
        .unwrap()
        .components()
        .is_empty());
    assert_eq!(
        BackupPath::parse(b"a/", true, 2, 1).unwrap(),
        parse(b"a").unwrap()
    );
    assert!(BackupPath::parse(b"a//", true, 3, 2).is_err());
    assert_eq!(
        BackupPath::parse(b"ab", false, 1, 1),
        Err(PathError::ByteLimit)
    );
    assert_eq!(
        BackupPath::parse(b"a/b", false, 3, 1),
        Err(PathError::ComponentLimit)
    );
}

#[test]
fn comparison_covers_component_and_byte_prefix_boundaries() {
    let paths = [
        b"".as_slice(),
        b"a",
        b"a/b",
        b"a/b/child",
        b"a/b!",
        b"a/bc",
        b"a/c",
        b"a!",
        b"a!/b",
        b"ab",
        b"raw-\xff",
        b"raw-\xff/a",
    ];
    for (left_index, left) in paths.iter().enumerate() {
        for (right_index, right) in paths.iter().enumerate() {
            let left = parse(left).unwrap();
            let right = parse(right).unwrap();
            let expected = if left_index < right_index {
                -1
            } else if left_index > right_index {
                1
            } else {
                0
            };
            assert_eq!(left.compare(&right), expected, "{left} / {right}");
            assert_eq!(
                left.compare(&right),
                match left.cmp(&right) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
#[test]
fn component_prefix_covers_root_equality_descendants_and_sibling_boundaries() {
    let root = parse(b".").unwrap();
    let a = parse(b"a").unwrap();
    let child = parse(b"a/child").unwrap();
    let sibling = parse(b"a-").unwrap();

    assert!(root.is_component_prefix_of(&root));
    assert!(root.is_component_prefix_of(&a));
    assert!(a.is_component_prefix_of(&a));
    assert!(a.is_component_prefix_of(&child));
    assert!(!a.is_component_prefix_of(&sibling));
    assert!(!child.is_component_prefix_of(&a));
}
