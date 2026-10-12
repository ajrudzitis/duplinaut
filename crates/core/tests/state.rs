use duplicity_core::manifest::{Manifest, ManifestLimits};
use duplicity_core::multipart::{AssembledArchive, AssemblyLimits, MultipartAssembler};
use duplicity_core::names::Envelope;
use duplicity_core::path::BackupPath;
use duplicity_core::state::{SnapshotState, StateError, StateLimits};
use duplicity_core::tar::EntryType;
use sha1::Digest;
use sha2::Digest as _;
use std::path::Path;
fn path(bytes: &[u8]) -> BackupPath {
    BackupPath::parse(bytes, false, 1024, 100).unwrap()
}
fn member(name: &str, kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{:011o}\0", payload.len()).as_bytes());
    header[136..148].copy_from_slice(b"00000000001\0");
    header[148..156].fill(b' ');
    header[156] = kind;
    header[257..265].copy_from_slice(b"ustar\x0000");
    if kind == b'2' {
        header[157..163].copy_from_slice(b"target");
    }
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut output = header.to_vec();
    output.extend_from_slice(payload);
    output.resize(output.len().next_multiple_of(512), 0);
    output
}
fn archive(records: &[(&str, u8, &[u8])]) -> AssembledArchive {
    let stored: Vec<u8> = records
        .iter()
        .flat_map(|(name, kind, payload)| member(name, *kind, payload))
        .collect();
    // Some adversarial state records are deliberately unordered. Inclusive tar
    // boundaries must still encompass all paths and equal first/last records.
    let start = records
        .first()
        .map_or(".", |r| r.0.split_once('/').unwrap().1);
    let end = records
        .last()
        .map_or(".", |r| r.0.split_once('/').unwrap().1);
    let digest = sha1::Sha1::digest(&stored)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let manifest = Manifest::parse(
        format!("Volume 1:\n StartingPath {start}\n EndingPath {end}\n Hash SHA1 {digest}\n")
            .as_bytes(),
        ManifestLimits::default(),
    )
    .unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assembler
        .push_stored_volume(1, &stored, Envelope::Plain)
        .unwrap();
    assembler.finish().unwrap()
}
fn initial() -> SnapshotState {
    SnapshotState::from_full(
        &archive(&[
            ("snapshot/.", b'5', b""),
            ("snapshot/dir", b'5', b""),
            ("snapshot/dir/child", b'0', b"child"),
            ("snapshot/file", b'0', b"old"),
        ]),
        StateLimits::default(),
    )
    .unwrap()
}
fn copy_delta() -> Vec<u8> {
    vec![0x72, 0x73, 0x02, 0x36, 0x45, 0, 3, 0]
}

//= docs/parsing-requirements.md#parse-state-002
//= type=test
//# A delta entry MUST use the reconstructed regular-file content for that same path from the preceding state as its basis.
//= docs/parsing-requirements.md#parse-state-002
//= type=test
//# A delta entry MUST use the reconstructed regular-file content for that same path from the preceding state as its basis.
#[test]
fn immutable_preceding_set_basis_and_transactional_failure() {
    let previous = initial();
    let delta = copy_delta();
    let changes = archive(&[
        ("snapshot/file", b'0', b"replacement"),
        ("diff/file", b'0', &delta),
        ("diff/file", b'0', &delta),
    ]);
    let next = previous
        .apply_incremental(&changes, StateLimits::default())
        .unwrap();
    assert_eq!(next.get(&path(b"file")).unwrap().content().unwrap(), b"old");
    assert_eq!(
        previous.get(&path(b"file")).unwrap().content().unwrap(),
        b"old"
    );
    let invalid = archive(&[
        ("snapshot/file", b'0', b"changed"),
        ("diff/missing", b'0', &delta),
    ]);
    assert!(matches!(
        previous.apply_incremental(&invalid, StateLimits::default()),
        Err(StateError::MissingBasis)
    ));
    assert_eq!(
        previous.get(&path(b"file")).unwrap().content().unwrap(),
        b"old"
    );
    let nonregular = archive(&[("diff/dir", b'0', &delta)]);
    assert!(matches!(
        previous.apply_incremental(&nonregular, StateLimits::default()),
        Err(StateError::IncompatibleBasis)
    ));
    let bad = archive(&[("diff/file", b'0', b"bad")]);
    assert!(matches!(
        previous.apply_incremental(&bad, StateLimits::default()),
        Err(StateError::Delta(_))
    ));
}

//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
#[test]
fn directory_preservation_type_replacement_deletion_and_recreation() {
    let previous = initial();
    let directory = archive(&[("snapshot/dir", b'5', b"")]);
    let next = previous
        .apply_incremental(&directory, StateLimits::default())
        .unwrap();
    assert_eq!(
        next.get(&path(b"dir/child")).unwrap().content().unwrap(),
        b"child"
    );
    let replaced = archive(&[
        ("snapshot/dir", b'2', b""),
        ("deleted/dir/child", b'0', b""),
    ]);
    let next = previous
        .apply_incremental(&replaced, StateLimits::default())
        .unwrap();
    assert_eq!(
        next.get(&path(b"dir")).unwrap().entry_type(),
        EntryType::Symlink
    );
    assert_eq!(next.get(&path(b"dir")).unwrap().linkname(), b"target");
    assert_eq!(next.get(&path(b"dir")).unwrap().mtime().seconds, 1);
    assert!(next.get(&path(b"dir/child")).is_none());
    let deleted = archive(&[("deleted/dir", b'0', b"")]);
    let next = previous
        .apply_incremental(&deleted, StateLimits::default())
        .unwrap();
    assert!(next.get(&path(b"dir")).is_none());
    assert!(next.get(&path(b"dir/child")).is_none());
    let recreated = archive(&[("snapshot/dir", b'0', b"new")]);
    let next = next
        .apply_incremental(&recreated, StateLimits::default())
        .unwrap();
    assert_eq!(next.get(&path(b"dir")).unwrap().content().unwrap(), b"new");
    assert!(next.get(&path(b"dir/child")).is_none());
    // Add a child before its newly added parent, with valid enclosing bounds.
    let temporary = archive(&[
        ("snapshot/.", b'5', b""),
        ("snapshot/a/b", b'0', b"child"),
        ("snapshot/a", b'5', b""),
        ("snapshot/z", b'0', b"last"),
    ]);
    let next = previous
        .apply_incremental(&temporary, StateLimits::default())
        .unwrap();
    assert_eq!(
        next.get(&path(b"a/b")).unwrap().content().unwrap(),
        b"child"
    );
}
#[test]
fn missing_ancestors_root_types_empty_state_and_unsupported_metadata() {
    assert!(
        SnapshotState::from_full(&archive(&[]), StateLimits::default())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        SnapshotState::from_full(
            &archive(&[("snapshot/.", b'0', b"")]),
            StateLimits::default()
        ),
        Err(StateError::RootType)
    ));
    assert!(matches!(
        SnapshotState::from_full(
            &archive(&[("snapshot/file", b'0', b"file")]),
            StateLimits::default()
        ),
        Err(StateError::Ancestor)
    ));
    let non_directory_parent = archive(&[("snapshot/file/child", b'0', b"data")]);
    assert!(matches!(
        initial().apply_incremental(&non_directory_parent, StateLimits::default()),
        Err(StateError::Ancestor)
    ));
    let unsupported = SnapshotState::from_full(
        &archive(&[("snapshot/.", b'5', b""), ("snapshot/device", b'3', b"")]),
        StateLimits::default(),
    )
    .unwrap();
    assert!(!unsupported
        .get(&path(b"device"))
        .unwrap()
        .supported_restore_type());
    assert!(unsupported
        .get(&path(b"device"))
        .unwrap()
        .content()
        .is_none());
    assert!(matches!(
        SnapshotState::from_full(
            &archive(&[("deleted/file", b'0', b"")]),
            StateLimits::default()
        ),
        Err(StateError::FullOperation)
    ));
    let malformed = archive(&[("deleted/file", b'0', b"illegal")]);
    assert!(matches!(
        initial().apply_incremental(&malformed, StateLimits::default()),
        Err(StateError::EntryPayload)
    ));
}
#[test]
fn resulting_and_live_limits_prevent_partial_state_success() {
    let previous = initial();
    let changes = archive(&[("snapshot/file", b'0', b"replacement")]);
    for limits in [
        StateLimits {
            payload_bytes: 7,
            ..StateLimits::default()
        },
        StateLimits {
            file_bytes: 2,
            ..StateLimits::default()
        },
        StateLimits {
            entries: 3,
            ..StateLimits::default()
        },
        StateLimits {
            metadata_bytes: 0,
            ..StateLimits::default()
        },
        StateLimits {
            live_payload_bytes: 1,
            ..StateLimits::default()
        },
        StateLimits {
            live_metadata_bytes: 0,
            ..StateLimits::default()
        },
    ] {
        assert!(matches!(
            previous.apply_incremental(&changes, limits),
            Err(StateError::Limit)
        ));
    }
    assert_eq!(
        previous.get(&path(b"file")).unwrap().content().unwrap(),
        b"old"
    );
    let delta = archive(&[("diff/file", b'0', &copy_delta())]);
    assert!(matches!(
        previous.apply_incremental(
            &delta,
            StateLimits {
                file_bytes: 2,
                ..StateLimits::default()
            }
        ),
        Err(StateError::Limit)
    ));
    let mut literal = vec![0x72, 0x73, 0x02, 0x36, 0x41, 100];
    literal.extend_from_slice(&[b'x'; 100]);
    literal.push(0);
    let changed = archive(&[("diff/file", b'0', &literal)]);
    // Logical new content fits, but preceding/archive/unchanged buffers leave
    // only 100 live bytes for delta reconstruction, including Vec growth.
    let live = previous.payload_bytes() + changed.entries()[0].payload.capacity() + 5 + 100;
    assert!(matches!(
        previous.apply_incremental(
            &changed,
            StateLimits {
                live_payload_bytes: live,
                ..StateLimits::default()
            }
        ),
        Err(StateError::Limit)
    ));
    let next = previous
        .apply_incremental(
            &changed,
            StateLimits {
                live_payload_bytes: live + 200,
                ..StateLimits::default()
            },
        )
        .unwrap();
    assert_eq!(
        next.get(&path(b"file")).unwrap().content().unwrap(),
        &[b'x'; 100]
    );
}
fn official_archive(root: &Path, name: &str) -> AssembledArchive {
    let manifest = Manifest::parse(
        &std::fs::read(root.join(name)).unwrap(),
        ManifestLimits::default(),
    )
    .unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let prefix = name.strip_suffix(".manifest").unwrap();
    for record in manifest.volumes() {
        let stored =
            std::fs::read(root.join(format!("{prefix}.vol{}.difftar.gz", record.number))).unwrap();
        assembler
            .push_stored_volume(record.number, &stored, Envelope::Gzip)
            .unwrap();
    }
    assembler.finish().unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn check_oracle(state: &SnapshotState, index: u64) {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == index)
        .unwrap()["entries"]
        .as_array()
        .unwrap();
    assert_eq!(state.len(), expected.len());
    for (path, entry) in state.entries() {
        let mut bytes = Vec::new();
        for component in path.components() {
            if !bytes.is_empty() {
                bytes.push(b'/');
            }
            bytes.extend_from_slice(component);
        }
        let item = expected
            .iter()
            .find(|item| item["path_hex"] == hex(&bytes))
            .unwrap();
        let kind = match entry.entry_type() {
            EntryType::Regular => "file",
            EntryType::Directory => "directory",
            EntryType::Symlink => "symlink",
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(item["kind"], kind);
        assert_eq!(
            item["mode"].as_u64().unwrap(),
            u64::from(entry.mode() & 0o777)
        );
        // Same explicitly recorded exclusion as official restoration comparison
        // in tests/producer/generate.py. A source symlink mtime can change without
        // a data record; native state preserves the last emitted link metadata.
        if entry.entry_type() != EntryType::Symlink {
            assert_eq!(
                i128::from(item["mtime_ns"].as_i64().unwrap()),
                i128::from(entry.mtime().seconds) * 1_000_000_000
                    + i128::from(entry.mtime().nanoseconds),
                "snapshot {index} path {}",
                hex(&bytes)
            );
        }
        if let Some(content) = entry.content() {
            assert_eq!(
                item["sha256"],
                format!("{:x}", sha2::Sha256::digest(content))
            );
            assert_eq!(item["size"].as_u64().unwrap(), content.len() as u64);
        }
        if entry.entry_type() == EntryType::Symlink {
            assert_eq!(item["symlink_target_hex"], hex(entry.linkname()));
        }
    }
}

//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
//= docs/parsing-requirements.md#parse-state-002
//= type=test
//# A delta entry MUST use the reconstructed regular-file content for that same path from the preceding state as its basis.
//= docs/parsing-requirements.md#parse-state-001
//= type=test
//# Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.
#[test]
fn official_three_snapshot_dates_match_contents_types_and_metadata() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain");
    let full = official_archive(&root, "duplicity-full.20231114T221320Z.manifest");
    let state = SnapshotState::from_full(&full, StateLimits::default()).unwrap();
    check_oracle(&state, 0);
    let first = official_archive(
        &root,
        "duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
    );
    let state = state
        .apply_incremental(&first, StateLimits::default())
        .unwrap();
    check_oracle(&state, 1);
    let second = official_archive(
        &root,
        "duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
    );
    let state = state
        .apply_incremental(&second, StateLimits::default())
        .unwrap();
    check_oracle(&state, 2);
}
