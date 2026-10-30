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

#[test]
fn deleted_bases_remain_available_from_preceding_set() {
    let previous = initial();
    let delta = copy_delta();
    let next = previous
        .apply_incremental(
            &archive(&[("deleted/file", b'0', b""), ("diff/file", b'0', &delta)]),
            StateLimits::default(),
        )
        .unwrap();
    assert_eq!(next.get(&path(b"file")).unwrap().content().unwrap(), b"old");
    let next = previous
        .apply_incremental(
            &archive(&[
                ("deleted/dir", b'0', b""),
                ("snapshot/dir", b'5', b""),
                ("diff/dir/child", b'0', &delta),
            ]),
            StateLimits::default(),
        )
        .unwrap();
    assert_eq!(
        next.get(&path(b"dir/child")).unwrap().content().unwrap(),
        b"chi"
    );
}

#[test]
//= docs/parsing-requirements.md#parse-gen-005
//= type=test
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
fn subtree_removal_preserves_near_prefixes_and_final_errors_are_atomic() {
    let previous = SnapshotState::from_full(
        &archive(&[
            ("snapshot/.", b'5', b""),
            ("snapshot/a", b'5', b""),
            ("snapshot/a/child", b'0', b"child"),
            ("snapshot/a-", b'0', b"dash"),
            ("snapshot/a0", b'0', b"zero"),
            ("snapshot/aλ", b'0', b"bytes"),
        ]),
        StateLimits::default(),
    )
    .unwrap();
    assert_eq!(previous.payload_bytes(), 18);
    for operation in ["deleted/a", "snapshot/a"] {
        let next = previous
            .apply_incremental(&archive(&[(operation, b'0', b"")]), StateLimits::default())
            .unwrap();
        assert_eq!(next.payload_bytes(), 13);
        assert!(next.get(&path(b"a/child")).is_none());
        for (name, contents) in [
            ("a-", b"dash".as_slice()),
            ("a0", b"zero"),
            ("aλ", b"bytes"),
        ] {
            assert_eq!(
                next.get(&path(name.as_bytes())).unwrap().content().unwrap(),
                contents
            );
        }
    }
    let invalid = archive(&[
        ("snapshot/.", b'5', b""),
        ("snapshot/a0", b'0', b"changed"),
        ("snapshot/missing/child", b'0', b"orphan"),
    ]);
    assert!(matches!(
        previous.apply_incremental(&invalid, StateLimits::default()),
        Err(StateError::Ancestor)
    ));
    let invalid_root = archive(&[("snapshot/.", b'0', b"new root")]);
    assert!(matches!(
        previous.apply_incremental(&invalid_root, StateLimits::default()),
        Err(StateError::RootType)
    ));
    assert_eq!(previous.len(), 6);
    assert_eq!(previous.payload_bytes(), 18);
    assert_eq!(
        previous.get(&path(b"a0")).unwrap().content().unwrap(),
        b"zero"
    );
    assert_eq!(
        previous.get(&path(b"a/child")).unwrap().content().unwrap(),
        b"child"
    );
    assert_eq!(
        previous.get(&path(b".")).unwrap().entry_type(),
        EntryType::Directory
    );
}

#[test]
fn exact_payload_entry_and_live_limits_accept_without_spare_budget() {
    let input = archive(&[("snapshot/.", b'5', b""), ("snapshot/file", b'0', b"abc")]);
    let stored_capacity: usize = input
        .entries()
        .iter()
        .map(|entry| entry.payload.capacity())
        .sum();
    let limits = StateLimits {
        payload_bytes: 3,
        file_bytes: 3,
        entries: 2,
        live_payload_bytes: stored_capacity + 3,
        ..StateLimits::default()
    };
    let state = SnapshotState::from_full(&input, limits).unwrap();
    assert_eq!(state.payload_bytes(), 3);
    assert_eq!(state.len(), 2);
    for insufficient in [
        StateLimits {
            payload_bytes: 2,
            ..limits
        },
        StateLimits {
            file_bytes: 2,
            ..limits
        },
        StateLimits {
            entries: 1,
            ..limits
        },
        StateLimits {
            live_payload_bytes: stored_capacity + 2,
            ..limits
        },
    ] {
        assert!(matches!(
            SnapshotState::from_full(&input, insufficient),
            Err(StateError::Limit)
        ));
    }
    assert!(SnapshotState::from_full(
        &archive(&[]),
        StateLimits {
            payload_bytes: 0,
            file_bytes: 0,
            entries: 0,
            live_payload_bytes: 0,
            ..StateLimits::default()
        }
    )
    .unwrap()
    .is_empty());
}

#[test]
fn exact_metadata_and_live_metadata_limits_include_transition_scratch() {
    use duplicity_core::multipart::LogicalEntry;
    use std::mem::size_of;
    let input = archive(&[("snapshot/.", b'5', b""), ("snapshot/file", b'0', b"abc")]);
    let baseline = SnapshotState::from_full(&input, StateLimits::default()).unwrap();
    // The documented logical budget includes the final ancestor stack, even
    // though that transient storage is absent from the returned state's charge.
    let logical = baseline.metadata_bytes() + 2 * size_of::<&BackupPath>();
    let path_storage = |path: &BackupPath| {
        size_of::<BackupPath>()
            + path.components().capacity() * size_of::<Vec<u8>>()
            + path.components().iter().map(Vec::capacity).sum::<usize>()
    };
    let archive_metadata = input.entries().len().max(4) * 2 * size_of::<LogicalEntry>()
        + input
            .entries()
            .iter()
            .map(|entry| {
                path_storage(&entry.path)
                    + entry.linkname.capacity()
                    + entry.uname.capacity()
                    + entry.gname.capacity()
            })
            .sum::<usize>();
    let scratch = input
        .entries()
        .iter()
        .map(|entry| path_storage(&entry.path))
        .max()
        .unwrap();
    let live = size_of::<SnapshotState>() + archive_metadata + scratch + logical;
    let exact = StateLimits {
        metadata_bytes: logical,
        live_metadata_bytes: live,
        ..StateLimits::default()
    };
    assert_eq!(
        SnapshotState::from_full(&input, exact)
            .unwrap()
            .metadata_bytes(),
        baseline.metadata_bytes()
    );
    for insufficient in [
        StateLimits {
            metadata_bytes: logical - 1,
            ..exact
        },
        StateLimits {
            live_metadata_bytes: live - 1,
            ..exact
        },
    ] {
        assert!(matches!(
            SnapshotState::from_full(&input, insufficient),
            Err(StateError::Limit)
        ));
    }
}

//= docs/parsing-requirements.md#restore-001
//= type=test
//# A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.
#[test]
fn restore_plan_preserves_component_paths_and_closes_directory_ancestors() {
    use duplicity_core::restore_plan::{RestorePlan, RestorePlanLimits};
    let state = initial();
    let selected = [path(b"file")];
    let plan = RestorePlan::from_roots(&state, &selected, RestorePlanLimits::default()).unwrap();
    assert_eq!(plan.entries().len(), 2);
    assert_eq!(plan.entries()[0].path(), &path(b"."));
    assert_eq!(plan.entries()[1].path(), &selected[0]);
    assert!(std::ptr::eq(
        plan.entries()[1].entry(),
        state.get(&selected[0]).unwrap()
    ));
    let overlapping = [path(b"dir/child"), path(b"dir"), path(b"dir/child")];
    let plan = RestorePlan::from_roots(&state, &overlapping, RestorePlanLimits::default()).unwrap();
    assert_eq!(plan.entries().len(), 3);
    assert_eq!(plan.entries()[0].path(), &path(b"."));
    assert_eq!(plan.entries()[1].path(), &path(b"dir"));
    assert_eq!(plan.entries()[2].path(), &path(b"dir/child"));
    assert_eq!(
        RestorePlan::all(&state, RestorePlanLimits::default())
            .unwrap()
            .entries()
            .len(),
        state.len()
    );
}

//= docs/parsing-requirements.md#restore-005
//= type=test
//# A restore selection requiring unsupported filesystem entry types MUST fail before publication with an entry-specific diagnostic.
#[test]
fn restore_plan_rejects_selected_special_types_missing_roots_and_budget_exhaustion() {
    use duplicity_core::restore_plan::{RestorePlan, RestorePlanError, RestorePlanLimits};
    let state = SnapshotState::from_full(
        &archive(&[
            ("snapshot/.", b'5', b""),
            ("snapshot/device", b'3', b""),
            ("snapshot/file", b'0', b"contents"),
        ]),
        StateLimits::default(),
    )
    .unwrap();
    assert!(
        matches!(RestorePlan::all(&state, RestorePlanLimits::default()), Err(RestorePlanError::Unsupported { path: bad }) if bad == &path(b"device"))
    );
    let roots = [path(b"file")];
    assert_eq!(
        RestorePlan::from_roots(&state, &roots, RestorePlanLimits::default())
            .unwrap()
            .entries()
            .len(),
        2
    );
    assert!(matches!(
        RestorePlan::from_roots(&state, &[path(b"missing")], RestorePlanLimits::default()),
        Err(RestorePlanError::MissingSelection { index: 0 })
    ));
    assert!(matches!(
        RestorePlan::from_roots(&state, &[], RestorePlanLimits::default()),
        Err(RestorePlanError::EmptySelection)
    ));
    for insufficient in [
        RestorePlanLimits {
            max_roots: 0,
            ..RestorePlanLimits::default()
        },
        RestorePlanLimits {
            max_state_entries: 2,
            ..RestorePlanLimits::default()
        },
        RestorePlanLimits {
            max_entries: 1,
            ..RestorePlanLimits::default()
        },
        RestorePlanLimits {
            max_metadata_bytes: 0,
            ..RestorePlanLimits::default()
        },
        RestorePlanLimits {
            max_candidate_checks: 2,
            ..RestorePlanLimits::default()
        },
    ] {
        assert!(matches!(
            RestorePlan::from_roots(&state, &roots, insufficient),
            Err(RestorePlanError::Limit)
        ));
    }
    assert!(RestorePlan::from_roots(
        &state,
        &roots,
        RestorePlanLimits {
            max_entries: 2,
            max_candidate_checks: 3,
            ..RestorePlanLimits::default()
        }
    )
    .is_ok());
    let empty = SnapshotState::from_full(&archive(&[]), StateLimits::default()).unwrap();
    assert!(RestorePlan::all(
        &empty,
        RestorePlanLimits {
            max_entries: 0,
            max_candidate_checks: 0,
            ..RestorePlanLimits::default()
        }
    )
    .unwrap()
    .entries()
    .is_empty());
}

#[test]
fn restore_plan_distinguishes_byte_paths_and_component_prefix_siblings() {
    use duplicity_core::restore_plan::{RestorePlan, RestorePlanLimits};
    let mut byte_member = member("snapshot/x", b'0', b"byte file");
    byte_member[9] = 0xff;
    byte_member[148..156].fill(b' ');
    let checksum: u64 = byte_member[..512].iter().map(|b| u64::from(*b)).sum();
    byte_member[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    let stored = [
        member("snapshot/.", b'5', b""),
        member("snapshot/a", b'5', b""),
        member("snapshot/a/child", b'0', b"child"),
        member("snapshot/a-", b'0', b"dash"),
        member("snapshot/a0", b'0', b"zero"),
        byte_member,
    ]
    .concat();
    let hash: String = sha1::Sha1::digest(&stored)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let manifest = Manifest::parse(
        format!("Volume 1:\n StartingPath .\n EndingPath \"\\xff\"\n Hash SHA1 {hash}\n")
            .as_bytes(),
        ManifestLimits::default(),
    )
    .unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assembler
        .push_stored_volume(1, &stored, Envelope::Plain)
        .unwrap();
    let state =
        SnapshotState::from_full(&assembler.finish().unwrap(), StateLimits::default()).unwrap();
    let byte_path = path(&[0xff]);
    let byte_plan = RestorePlan::from_roots(
        &state,
        std::slice::from_ref(&byte_path),
        RestorePlanLimits::default(),
    )
    .unwrap();
    assert_eq!(byte_plan.entries().len(), 2);
    assert_eq!(byte_plan.entries()[1].path(), &byte_path);
    assert_eq!(
        byte_plan.entries()[1].entry().content().unwrap(),
        b"byte file"
    );
    let directory =
        RestorePlan::from_roots(&state, &[path(b"a")], RestorePlanLimits::default()).unwrap();
    assert_eq!(directory.entries().len(), 3);
    assert_eq!(directory.entries()[1].path(), &path(b"a"));
    assert_eq!(directory.entries()[2].path(), &path(b"a/child"));
}

//= docs/parsing-requirements.md#parse-state-003
//= type=test
//# `SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components. Each component MUST be compared as an unsigned byte sequence using lexicographic byte order, with a proper prefix ordered before the longer sequence. Paths MUST be compared as sequences of components using the same lexicographic rule, with a proper component-sequence prefix ordered before the longer path. Component boundaries MUST remain significant; the ordering MUST NOT be derived by comparing slash-joined or rendered path strings.
#[test]
fn snapshot_entries_use_bytewise_component_lexicographic_order() {
    let mut raw_member = member("snapshot/x", b'0', b"raw byte");
    raw_member[9] = 0xff;
    raw_member[148..156].fill(b' ');
    let checksum: u64 = raw_member[..512].iter().map(|byte| u64::from(*byte)).sum();
    raw_member[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());

    let stored = [
        member("snapshot/.", b'5', b""),
        member("snapshot/a", b'5', b""),
        member("snapshot/a/child", b'0', b"a child"),
        member("snapshot/a-", b'5', b""),
        member("snapshot/a-/child", b'0', b"a-dash child"),
        member("snapshot/a0", b'5', b""),
        member("snapshot/a0/child", b'0', b"a-zero child"),
        member("snapshot/aa", b'0', b"aa file"),
        raw_member,
    ]
    .concat();
    let hash: String = sha1::Sha1::digest(&stored)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let manifest = Manifest::parse(
        format!("Volume 1:\n StartingPath .\n EndingPath \"\\xff\"\n Hash SHA1 {hash}\n")
            .as_bytes(),
        ManifestLimits::default(),
    )
    .unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assembler
        .push_stored_volume(1, &stored, Envelope::Plain)
        .unwrap();
    let state =
        SnapshotState::from_full(&assembler.finish().unwrap(), StateLimits::default()).unwrap();

    let expected = [
        path(b"."),
        path(b"a"),
        path(b"a/child"),
        path(b"a-"),
        path(b"a-/child"),
        path(b"a0"),
        path(b"a0/child"),
        path(b"aa"),
        path(&[0xff]),
    ];
    let actual: Vec<_> = state
        .entries()
        .map(|(entry_path, _)| entry_path.clone())
        .collect();
    assert_eq!(actual, expected);

    // In particular, component order puts `a/child` before `a-/child`; a
    // slash-joined byte comparison would reverse these two paths.
    for (left_index, left) in expected.iter().enumerate() {
        for (right_index, right) in expected.iter().enumerate() {
            let expected_order = if left_index < right_index {
                -1
            } else if left_index > right_index {
                1
            } else {
                0
            };
            let expected_cmp = match expected_order {
                -1 => std::cmp::Ordering::Less,
                0 => std::cmp::Ordering::Equal,
                1 => std::cmp::Ordering::Greater,
                _ => unreachable!(),
            };
            assert_eq!(
                left.compare(right),
                expected_order,
                "left={left:?}, right={right:?}"
            );
            assert_eq!(left.cmp(right), expected_cmp);
            assert_eq!(left.partial_cmp(right), Some(expected_cmp));
        }
    }
}
