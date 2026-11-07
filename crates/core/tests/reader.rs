use duplicity_core::chain::{ChainError, SetKind};
use duplicity_core::names::{NameConfig, Timestamp};
use duplicity_core::path::BackupPath;
use duplicity_core::reader::{Catalog, DiscoveryStatus, ReaderError, ReaderLimits};
use duplicity_core::source::{
    Cancellation, ObjectIdentity, ObjectSource, SourceError, SourceLimits, StoredObject,
};
use sha2::Digest;
use std::io::{Cursor, Read};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

struct Source {
    objects: Vec<(StoredObject, Vec<u8>)>,
    mode: usize,
    opens: AtomicUsize,
    opened_keys: Mutex<Vec<String>>,
}
impl Source {
    fn official() -> Self {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain");
        let mut source = Self {
            objects: Vec::new(),
            mode: 0,
            opens: AtomicUsize::new(0),
            opened_keys: Mutex::new(Vec::new()),
        };
        for file in std::fs::read_dir(directory).unwrap() {
            let file = file.unwrap();
            source.add(
                &format!("backup/{}", file.file_name().to_str().unwrap()),
                std::fs::read(file.path()).unwrap(),
            );
        }
        source
    }
    fn add(&mut self, key: &str, bytes: Vec<u8>) {
        self.objects.push((
            StoredObject {
                key: key.into(),
                size: bytes.len() as u64,
                identity: ObjectIdentity {
                    version_id: Some("v1".into()),
                    etag: Some(format!("{:x}", sha2::Sha256::digest(&bytes))),
                },
            },
            bytes,
        ));
    }
}
impl ObjectSource for Source {
    fn list(
        &self,
        prefix: &str,
        _: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Vec<StoredObject>, SourceError> {
        if cancel.is_cancelled() {
            return Err(SourceError::Cancelled);
        }
        Ok(self
            .objects
            .iter()
            .filter(|(object, _)| object.key.starts_with(prefix))
            .map(|(object, _)| object.clone())
            .collect())
    }
    fn open(
        &self,
        object: &StoredObject,
        _: SourceLimits,
        cancel: &Cancellation,
    ) -> Result<Box<dyn Read + Send>, SourceError> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        self.opened_keys.lock().unwrap().push(object.key.clone());
        let (expected, bytes) = self
            .objects
            .iter()
            .find(|(expected, _)| expected.key == object.key)
            .ok_or(SourceError::Missing)?;
        if expected != object || self.mode == 1 {
            return Err(SourceError::Changed);
        }
        if object.key.contains(".to.20231116T221320Z.") {
            if self.mode == 5 {
                return Err(SourceError::Changed);
            }
            if self.mode == 6 {
                cancel.cancel();
            }
        }
        if self.mode == 2 {
            cancel.cancel();
        }
        let mut bytes = bytes.clone();
        if self.mode == 3 {
            bytes.push(0);
        }
        if self.mode == 4 {
            bytes.pop();
        }
        Ok(Box::new(Cursor::new(bytes)))
    }
}
fn discover(source: &Source) -> Catalog<'_> {
    Catalog::discover(
        source,
        "backup/",
        NameConfig::default(),
        ReaderLimits::default(),
        &Cancellation::default(),
    )
    .unwrap()
}
fn anchor(catalog: &Catalog<'_>) -> usize {
    catalog
        .manifests()
        .find(|(_, _, set)| set.kind() == SetKind::Full)
        .unwrap()
        .0
}
fn decode_oracle_hex(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(
        remainder.is_empty(),
        "oracle hex values contain whole bytes"
    );
    pairs
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(pair, 16).expect("oracle contains valid hexadecimal bytes")
        })
        .collect()
}

//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
#[test]
fn original_official_local_objects_reconstruct_all_three_dates_and_restore_plans() {
    let source = Source::official();
    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0); // discovery is metadata-only
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    for day in 0..3 {
        let snapshot = catalog
            .read_snapshot(
                anchor(&catalog),
                Timestamp(1700000000 + day * 86400),
                &Cancellation::default(),
            )
            .unwrap();
        let expected = oracle["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["mode"] == "plain" && e["index"] == day as u64)
            .unwrap()["entries"]
            .as_array()
            .unwrap();
        assert_eq!(snapshot.state().len(), expected.len());
        for (path, entry) in snapshot.state().entries() {
            let bytes = path
                .components()
                .iter()
                .map(|part| part.as_slice())
                .collect::<Vec<_>>()
                .join(&b'/');
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            let oracle = expected.iter().find(|e| e["path_hex"] == hex).unwrap();
            assert_eq!(
                entry.mode() & 0o777,
                oracle["mode"].as_u64().unwrap() as u32
            );
            // The recorded official restore/source comparison excludes symlink
            // mtimes: unchanged links do not emit source-only mtime changes.
            if oracle["kind"] != "symlink" {
                assert_eq!(
                    entry.mtime().seconds as i128 * 1000000000
                        + i128::from(entry.mtime().nanoseconds),
                    oracle["mtime_ns"].as_i64().unwrap() as i128,
                    "day={day} path={hex} kind={}",
                    oracle["kind"]
                );
            }
            match oracle["kind"].as_str().unwrap() {
                "file" => {
                    assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Regular);
                    assert_eq!(
                        format!("{:x}", sha2::Sha256::digest(entry.content().unwrap())),
                        oracle["sha256"].as_str().unwrap()
                    );
                }
                "symlink" => {
                    assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Symlink);
                    let link: String = entry
                        .linkname()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    assert_eq!(link, oracle["symlink_target_hex"].as_str().unwrap());
                }
                "directory" => assert_eq!(
                    entry.entry_type(),
                    duplicity_core::tar::EntryType::Directory
                ),
                other => panic!("unexpected fixture kind {other}"),
            }
        }
        assert_eq!(
            snapshot
                .restore_all(Default::default())
                .unwrap()
                .entries()
                .len(),
            expected.len()
        );
        let selected = BackupPath::parse(b"large.bin", false, 1024, 32).unwrap();
        assert!(!snapshot
            .restore_roots(&[selected], Default::default())
            .unwrap()
            .entries()
            .is_empty());
    }
}

//= docs/parsing-requirements.md#parse-source-001
//= type=test
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
#[test]
fn identity_cancellation_and_exact_body_failures_publish_no_snapshot() {
    for mode in [5, 6] {
        let mut source = Source::official();
        source.mode = mode;
        let catalog = discover(&source);
        let result = catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700172800),
            &Cancellation::default(),
        );
        if mode == 5 {
            assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Changed))
            ));
        } else {
            assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Cancelled))
            ));
        }
        assert!(source.opens.load(Ordering::SeqCst) > 3);
    }
    for mode in 1..=4 {
        let mut source = Source::official();
        source.mode = mode;
        let catalog = discover(&source);
        let result = catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700000000),
            &Cancellation::default(),
        );
        match mode {
            1 => assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Changed))
            )),
            2 => assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Cancelled))
            )),
            _ => assert!(matches!(
                result,
                Err(ReaderError::Source(SourceError::Io(_)))
            )),
        }
    }
    let source = Source::official();
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(matches!(
        Catalog::discover(
            &source,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &cancel
        ),
        Err(ReaderError::Source(SourceError::Cancelled))
    ));
    let catalog = discover(&source);
    assert!(matches!(
        catalog.read_snapshot(anchor(&catalog), Timestamp(1700000000), &cancel),
        Err(ReaderError::Source(SourceError::Cancelled))
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
fn missing_corrupt_and_ambiguous_objects_fail_explicitly() {
    let mut missing = Source::official();
    missing.objects.retain(|(object, _)| {
        !object.key.ends_with("vol2.difftar.gz") || !object.key.contains("full")
    });
    assert!(missing.objects.iter().any(|(object, _)| {
        object.key.contains("full") && object.key.ends_with("vol3.difftar.gz")
    }));
    let catalog = discover(&missing);
    assert!(matches!(
        catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::MissingVolume { number: 2 })
    ));
    let mut corrupt = Source::official();
    let (_, bytes) = corrupt
        .objects
        .iter_mut()
        .find(|(object, _)| object.key.contains("full") && object.key.ends_with("vol2.difftar.gz"))
        .unwrap();
    bytes[0] ^= 1;
    let catalog = discover(&corrupt);
    assert!(matches!(
        catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::Assembly(_))
    ));
    let mut branch = Source::official();
    branch.add(
        "backup/duplicity-inc.20231114T221320Z.to.20231116T221320Z.manifest",
        Vec::new(),
    );
    let catalog = discover(&branch);
    assert!(matches!(
        catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700172800),
            &Cancellation::default()
        ),
        Err(ReaderError::Chain(ChainError::Ambiguous))
    ));
    let mut duplicate = Source::official();
    duplicate.add(
        "backup/duplicity-full.20231114T221320Z.manifest.gpg",
        Vec::new(),
    );
    assert!(matches!(
        Catalog::discover(
            &duplicate,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &Cancellation::default()
        ),
        Err(ReaderError::DuplicateArtifact)
    ));
    let mut duplicate_volume = Source::official();
    duplicate_volume.add(
        "backup/duplicity-full.20231114T221320Z.vol1.difftar",
        Vec::new(),
    );
    assert!(matches!(
        Catalog::discover(
            &duplicate_volume,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &Cancellation::default()
        ),
        Err(ReaderError::DuplicateArtifact)
    ));
    let mut duplicate_key = Source::official();
    duplicate_key.add("backup/note.txt", Vec::new());
    duplicate_key.add("backup/note.txt", Vec::new());
    assert!(matches!(
        Catalog::discover(
            &duplicate_key,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &Cancellation::default()
        ),
        Err(ReaderError::DuplicateObject)
    ));
    let mut nested = Source::official();
    nested.add(
        "backup/nested/duplicity-full.20231114T221320Z.manifest",
        Vec::new(),
    );
    assert!(matches!(
        Catalog::discover(
            &nested,
            "backup/",
            NameConfig::default(),
            ReaderLimits::default(),
            &Cancellation::default()
        ),
        Err(ReaderError::CollectionAmbiguous)
    ));
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_partial_full_volume_is_visible_but_cannot_satisfy_required_volume() {
    const ORIGINAL: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const PARTIAL: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.part.gz";
    const VOLUME_THREE: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let original_keys = {
        let mut keys = original_inventory
            .iter()
            .map(|(object, _)| object.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains full volume 2");
    let (original_volume_three, original_volume_three_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == VOLUME_THREE)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains full volume 3");

    let (renamed_volume_two, renamed_volume_two_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains full volume 2");
    renamed_volume_two.key = PARTIAL.into();
    assert_eq!(renamed_volume_two_bytes, &original_volume_two_bytes);
    assert_eq!(renamed_volume_two.size, original_volume_two.size);
    assert_eq!(renamed_volume_two.identity, original_volume_two.identity);

    let mut expected_keys = original_keys;
    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains full volume 2");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    assert!(!actual_keys.iter().any(|key| key == ORIGINAL));
    assert!(actual_keys.iter().any(|key| key == PARTIAL));

    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original inventory object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let (retained_volume_three, retained_volume_three_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == VOLUME_THREE)
        .expect("volume 3 remains in the inventory");
    assert_eq!(retained_volume_three, &original_volume_three);
    assert_eq!(retained_volume_three_bytes, &original_volume_three_bytes);

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial volume remains visible in discovery");
    let partial_name = partial_entry.name().expect("partial key is parsed");
    assert!(partial_name.partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let completed_volume_three = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_THREE)
        .expect("completed volume 3 remains visible in discovery");
    assert!(!completed_volume_three.name().unwrap().partial);
    assert_eq!(completed_volume_three.status(), DiscoveryStatus::Completed);

    let result = catalog.read_snapshot(
        anchor(&catalog),
        Timestamp(1700000000),
        &Cancellation::default(),
    );
    assert!(matches!(
        result,
        Err(ReaderError::MissingVolume { number: 2 })
    ));
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_partial_first_incremental_volume_is_visible_but_cannot_satisfy_required_volume() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";
    const PARTIAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.gz";
    const VOLUME_TWO: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume_one, original_volume_one_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 1");
    let (renamed_volume_one, renamed_volume_one_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains first-incremental volume 1");
    renamed_volume_one.key = PARTIAL.into();
    assert_eq!(renamed_volume_one_bytes, &original_volume_one_bytes);
    assert_eq!(renamed_volume_one.size, original_volume_one.size);
    assert_eq!(renamed_volume_one.identity, original_volume_one.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains first-incremental volume 1");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    assert!(!actual_keys.iter().any(|key| key == ORIGINAL));
    assert!(actual_keys.iter().any(|key| key == PARTIAL));
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial volume remains visible in discovery");
    let partial_name = partial_entry.name().expect("partial key is parsed");
    assert_eq!(
        partial_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (partial_name.start.0, partial_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(partial_name.volume, Some(1));
    assert!(partial_name.partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let completed_volume_two = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_TWO)
        .expect("completed first-incremental volume 2 remains visible");
    let volume_two_name = completed_volume_two.name().expect("volume 2 key is parsed");
    assert_eq!(
        volume_two_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (volume_two_name.start.0, volume_two_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(volume_two_name.volume, Some(2));
    assert!(!volume_two_name.partial);
    assert_eq!(completed_volume_two.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut intervals = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    assert_eq!(
        intervals,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("completed full manifest remains a public anchor");
    assert_eq!(
        (full_descriptor.start().0, full_descriptor.end().0),
        (1700000000, 1700000000)
    );
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 1 })
    ));
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
#[test]
fn official_partial_full_manifest_remains_visible_but_is_not_a_chain_anchor() {
    const ORIGINAL: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const PARTIAL: &str = "backup/duplicity-full.20231114T221320Z.manifest.part";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let original_keys = {
        let mut keys = original_inventory
            .iter()
            .map(|(object, _)| object.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the full manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the full manifest");
    renamed_manifest.key = PARTIAL.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let mut expected_keys = original_keys;
    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the full manifest");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial manifest remains visible in discovery");
    assert!(partial_entry.name().expect("partial key is parsed").partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    assert!(descriptors
        .iter()
        .all(|(_, _, descriptor)| descriptor.kind() == SetKind::Incremental));
    let (first_incremental_index, _, first_descriptor) = descriptors
        .first()
        .expect("both incremental manifests remain in the catalog");
    assert_eq!(first_descriptor.kind(), SetKind::Incremental);

    assert!(matches!(
        catalog.read_snapshot(
            *first_incremental_index,
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::Chain(ChainError::InvalidAnchor))
    ));
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_partial_first_incremental_manifest_leaves_missing_chain_transition() {
    const ORIGINAL: &str = "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";
    const PARTIAL: &str = "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let original_keys = {
        let mut keys = original_inventory
            .iter()
            .map(|(object, _)| object.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the first incremental manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the first incremental manifest");
    renamed_manifest.key = PARTIAL.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let mut expected_keys = original_keys;
    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the first incremental manifest");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial manifest remains visible in discovery");
    assert!(partial_entry.name().expect("partial key is parsed").partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    let full_descriptors = descriptors
        .iter()
        .filter(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .collect::<Vec<_>>();
    let incremental_descriptors = descriptors
        .iter()
        .filter(|(_, _, descriptor)| descriptor.kind() == SetKind::Incremental)
        .collect::<Vec<_>>();
    assert_eq!(full_descriptors.len(), 1);
    assert_eq!(incremental_descriptors.len(), 1);
    assert_eq!(full_descriptors[0].2.start().0, 1700000000);
    assert_eq!(full_descriptors[0].2.end().0, 1700000000);
    assert_eq!(incremental_descriptors[0].2.start().0, 1700086400);
    assert_eq!(incremental_descriptors[0].2.end().0, 1700172800);
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    assert_eq!(full_descriptor.start().0, 1700000000);
    assert_eq!(full_descriptor.end().0, 1700000000);

    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Chain(ChainError::Missing))
    ));
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_partial_final_incremental_manifest_preserves_prior_snapshot_and_reports_missing_final()
{
    const ORIGINAL: &str = "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest";
    const PARTIAL: &str = "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.part";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the final incremental manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the final incremental manifest");
    renamed_manifest.key = PARTIAL.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the final incremental manifest");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial manifest remains visible in discovery");
    let partial_name = partial_entry.name().expect("partial key is parsed");
    assert_eq!(
        partial_name.kind,
        duplicity_core::names::ObjectKind::IncrementalManifest
    );
    assert_eq!(
        (partial_name.start.0, partial_name.end.0),
        (1700086400, 1700172800)
    );
    assert!(partial_name.partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    let mut intervals = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    assert_eq!(
        intervals,
        vec![(0, 1700000000, 1700000000), (1, 1700000000, 1700086400)]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("completed full manifest remains a public anchor");
    assert_eq!(
        (full_descriptor.start().0, full_descriptor.end().0),
        (1700000000, 1700000000)
    );

    let prior = catalog
        .read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default())
        .expect("the completed chain through the prior date remains restorable");
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 1)
        .expect("official oracle contains the prior plain snapshot");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|exclusion| exclusion == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"].as_array().unwrap();
    assert_eq!(prior.state().len(), expected_entries.len());
    let mut expected_paths = expected_entries
        .iter()
        .map(|expected| {
            expected["components_hex"]
                .as_array()
                .unwrap()
                .iter()
                .map(|component| decode_oracle_hex(component.as_str().unwrap()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    expected_paths.sort();
    let actual_paths = prior
        .state()
        .entries()
        .map(|(path, _)| {
            path.components()
                .iter()
                .map(|component| component.as_slice().to_vec())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_paths,
        "snapshot paths and order are raw component bytes"
    );

    for ((path, entry), expected_components) in prior.state().entries().zip(expected_paths.iter()) {
        assert_eq!(
            path.components()
                .iter()
                .map(|component| component.as_slice())
                .collect::<Vec<_>>(),
            expected_components
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()
        );
        let expected = expected_entries
            .iter()
            .find(|expected| {
                expected["components_hex"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|component| decode_oracle_hex(component.as_str().unwrap()))
                    .collect::<Vec<_>>()
                    == *expected_components
            })
            .expect("oracle records each expected raw path");
        assert_eq!(
            entry.mode() & 0o777,
            expected["mode"].as_u64().unwrap() as u32
        );
        let kind = expected["kind"].as_str().unwrap();
        if kind != "symlink" {
            let mtime_ns = expected["mtime_ns"].as_i64().unwrap();
            assert_eq!(entry.mtime().seconds, mtime_ns.div_euclid(1_000_000_000));
            assert_eq!(
                i64::from(entry.mtime().nanoseconds),
                mtime_ns.rem_euclid(1_000_000_000)
            );
        }
        match kind {
            "file" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Regular);
                assert_eq!(
                    entry.content().unwrap().len() as u64,
                    expected["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(entry.content().unwrap())),
                    expected["sha256"].as_str().unwrap()
                );
            }
            "directory" => assert_eq!(
                entry.entry_type(),
                duplicity_core::tar::EntryType::Directory
            ),
            "symlink" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Symlink);
                assert_eq!(
                    entry.linkname(),
                    decode_oracle_hex(expected["symlink_target_hex"].as_str().unwrap())
                );
            }
            other => panic!("unexpected fixture kind {other}"),
        }
    }

    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Chain(ChainError::Missing))
    ));
}

//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_partial_final_incremental_volume_preserves_prior_snapshot_and_reports_missing_final() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz";
    const PARTIAL: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.part.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume, original_volume_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains final-incremental volume 1");
    let (renamed_volume, renamed_volume_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains final-incremental volume 1");
    renamed_volume.key = PARTIAL.into();
    assert_eq!(renamed_volume_bytes, &original_volume_bytes);
    assert_eq!(renamed_volume.size, original_volume.size);
    assert_eq!(renamed_volume.identity, original_volume.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains final-incremental volume 1");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    assert!(!actual_keys.iter().any(|key| key == ORIGINAL));
    assert!(actual_keys.iter().any(|key| key == PARTIAL));
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial final volume remains visible in discovery");
    let partial_name = partial_entry.name().expect("partial key is parsed");
    assert_eq!(
        partial_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (partial_name.start.0, partial_name.end.0),
        (1700086400, 1700172800)
    );
    assert_eq!(partial_name.volume, Some(1));
    assert!(partial_name.partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut intervals = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    assert_eq!(
        intervals,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("completed full manifest remains a public anchor");
    assert_eq!(
        (full_descriptor.start().0, full_descriptor.end().0),
        (1700000000, 1700000000)
    );

    let prior = catalog
        .read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default())
        .expect("the completed chain through the prior date remains restorable");
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 1)
        .expect("official oracle contains the prior plain snapshot");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|exclusion| exclusion == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"].as_array().unwrap();
    assert_eq!(prior.state().len(), expected_entries.len());
    let mut expected_rows = expected_entries
        .iter()
        .map(|expected| {
            (
                expected["components_hex"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|component| decode_oracle_hex(component.as_str().unwrap()))
                    .collect::<Vec<_>>(),
                expected,
            )
        })
        .collect::<Vec<_>>();
    expected_rows.sort_by(|left, right| left.0.cmp(&right.0));
    let actual_rows = prior.state().entries().collect::<Vec<_>>();
    assert_eq!(actual_rows.len(), expected_rows.len());

    for ((path, entry), (expected_components, expected)) in
        actual_rows.into_iter().zip(expected_rows.iter())
    {
        assert_eq!(
            path.components()
                .iter()
                .map(|component| component.as_slice())
                .collect::<Vec<_>>(),
            expected_components
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            entry.mode() & 0o777,
            expected["mode"].as_u64().unwrap() as u32
        );
        let kind = expected["kind"].as_str().unwrap();
        if kind != "symlink" {
            let mtime_ns = expected["mtime_ns"].as_i64().unwrap();
            assert_eq!(entry.mtime().seconds, mtime_ns.div_euclid(1_000_000_000));
            assert_eq!(
                i64::from(entry.mtime().nanoseconds),
                mtime_ns.rem_euclid(1_000_000_000)
            );
        }
        match kind {
            "file" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Regular);
                assert_eq!(
                    entry.content().unwrap().len() as u64,
                    expected["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(entry.content().unwrap())),
                    expected["sha256"].as_str().unwrap()
                );
            }
            "directory" => assert_eq!(
                entry.entry_type(),
                duplicity_core::tar::EntryType::Directory
            ),
            "symlink" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Symlink);
                assert_eq!(
                    entry.linkname(),
                    decode_oracle_hex(expected["symlink_target_hex"].as_str().unwrap())
                );
            }
            other => panic!("unexpected fixture kind {other}"),
        }
    }

    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 1 })
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_partial_first_incremental_volume_two_is_visible_but_cannot_satisfy_volume_two() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";
    const PARTIAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.part.gz";
    const VOLUME_ONE: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 2");
    let (renamed_volume_two, renamed_volume_two_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains first-incremental volume 2");
    renamed_volume_two.key = PARTIAL.into();
    assert_eq!(renamed_volume_two_bytes, &original_volume_two_bytes);
    assert_eq!(renamed_volume_two.size, original_volume_two.size);
    assert_eq!(renamed_volume_two.identity, original_volume_two.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains first-incremental volume 2");
    *old_key = PARTIAL.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            PARTIAL
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let partial_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == PARTIAL)
        .expect("renamed partial volume remains visible in discovery");
    let partial_name = partial_entry.name().expect("partial key is parsed");
    assert_eq!(
        partial_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (partial_name.start.0, partial_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(partial_name.volume, Some(2));
    assert_eq!(partial_name.envelope, duplicity_core::names::Envelope::Gzip);
    assert!(partial_name.partial);
    assert_eq!(partial_entry.status(), DiscoveryStatus::Partial);

    let completed_volume_one = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_ONE)
        .expect("completed first-incremental volume 1 remains visible");
    let volume_one_name = completed_volume_one.name().expect("volume 1 key is parsed");
    assert_eq!(
        volume_one_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (volume_one_name.start.0, volume_one_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(volume_one_name.volume, Some(1));
    assert_eq!(
        volume_one_name.envelope,
        duplicity_core::names::Envelope::Gzip
    );
    assert!(!volume_one_name.partial);
    assert_eq!(completed_volume_one.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("completed full manifest remains a public anchor");
    assert_eq!(
        (descriptor.start().0, descriptor.end().0),
        (1700000000, 1700000000)
    );

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 2 })
    ));
    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 6);
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys
        .iter()
        .any(|key| key == ORIGINAL || key == PARTIAL));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_first_incremental_manifest_is_visible_but_leaves_missing_transition() {
    const ORIGINAL: &str = "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";
    const UNSUPPORTED: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.p7m";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the first incremental manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the first incremental manifest");
    renamed_manifest.key = UNSUPPORTED.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the first incremental manifest");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported manifest remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalManifest
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    let mut intervals = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    intervals.sort_unstable();
    assert_eq!(
        intervals,
        vec![(0, 1700000000, 1700000000), (1, 1700086400, 1700172800)]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    assert_eq!(full_descriptor.start().0, 1700000000);
    assert_eq!(full_descriptor.end().0, 1700000000);
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Chain(ChainError::Missing))
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_final_incremental_manifest_is_visible_but_leaves_missing_transition() {
    const ORIGINAL: &str = "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest";
    const UNSUPPORTED: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.p7m";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the final incremental manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the final incremental manifest");
    renamed_manifest.key = UNSUPPORTED.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the final incremental manifest");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported manifest remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalManifest
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700086400, 1700172800)
    );
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![(0, 1700000000, 1700000000), (1, 1700000000, 1700086400),]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    assert_eq!(full_descriptor.start().0, 1700000000);
    assert_eq!(full_descriptor.end().0, 1700000000);
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Chain(ChainError::Missing))
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_pkcs7_full_signatures_archive_preserves_plain_snapshot_and_required_open_set() {
    const ORIGINAL: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const UNSUPPORTED: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.p7m";
    const FIRST_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";
    const SECOND_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the full-signatures archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the full-signatures archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the full-signatures archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported full-signatures archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::FullSignatures
    );
    assert_eq!(unsupported_name.start, Timestamp(1700000000));
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect("the unsupported full-signatures archive does not block plain restoration");

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == FIRST_INCREMENTAL_SIGNATURES
            || key == SECOND_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_partial_first_incremental_signature_archive_preserves_plain_snapshot_and_required_open_set(
) {
    const ORIGINAL: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gz";
    const FULL_SIGNATURES: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const SECOND_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the first-incremental signature archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the first-incremental signature archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the first-incremental signature archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed partial first-incremental signature archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalSignatures
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(unsupported_name.volume, None);
    assert!(unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Gzip
    );
    assert_eq!(unsupported_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect(
            "the unsupported first-incremental signature archive does not block plain restoration",
        );

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == ORIGINAL
            || key == FULL_SIGNATURES
            || key == SECOND_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_first_incremental_volume_is_visible_but_cannot_satisfy_volume_one() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.p7m";
    const VOLUME_TWO: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume_one, original_volume_one_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 1");
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == VOLUME_TWO)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 2");
    let (renamed_volume_one, renamed_volume_one_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains first-incremental volume 1");
    renamed_volume_one.key = UNSUPPORTED.into();
    assert_eq!(renamed_volume_one_bytes, &original_volume_one_bytes);
    assert_eq!(renamed_volume_one.size, original_volume_one.size);
    assert_eq!(renamed_volume_one.identity, original_volume_one.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains first-incremental volume 1");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported volume remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(unsupported_name.volume, Some(1));
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let completed_volume_two = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_TWO)
        .expect("completed incremental volume 2 remains visible");
    let volume_two_name = completed_volume_two.name().expect("volume key is parsed");
    assert_eq!(
        volume_two_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (volume_two_name.start.0, volume_two_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(volume_two_name.volume, Some(2));
    assert_eq!(completed_volume_two.status(), DiscoveryStatus::Completed);
    assert_eq!(completed_volume_two.object(), &original_volume_two);
    assert_eq!(
        source
            .objects
            .iter()
            .find(|(object, _)| object.key == VOLUME_TWO)
            .unwrap()
            .1,
        original_volume_two_bytes
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    assert_eq!(full_descriptor.start().0, 1700000000);
    assert_eq!(full_descriptor.end().0, 1700000000);
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 1 })
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-name-005
//= type=test
//# Objects marked partial MUST be excluded from completed restore sets.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_partial_pkcs7_first_incremental_volume_two_is_visible_but_cannot_satisfy_volume_two() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";
    const COMBINED: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.part.p7m";
    const VOLUME_ONE: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 2");
    let (renamed_volume_two, renamed_volume_two_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains first-incremental volume 2");
    renamed_volume_two.key = COMBINED.into();
    assert_eq!(renamed_volume_two_bytes, &original_volume_two_bytes);
    assert_eq!(renamed_volume_two.size, original_volume_two.size);
    assert_eq!(renamed_volume_two.identity, original_volume_two.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains first-incremental volume 2");
    *old_key = COMBINED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            COMBINED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let combined_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == COMBINED)
        .expect("renamed partial and unsupported volume remains visible in discovery");
    let combined_name = combined_entry.name().expect("volume key is parsed");
    assert_eq!(
        combined_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (combined_name.start.0, combined_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(combined_name.volume, Some(2));
    assert!(combined_name.partial);
    assert_eq!(
        combined_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(combined_entry.status(), DiscoveryStatus::Partial);

    let completed_volume_one = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_ONE)
        .expect("completed first-incremental volume 1 remains visible");
    let volume_one_name = completed_volume_one.name().expect("volume key is parsed");
    assert_eq!(
        volume_one_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (volume_one_name.start.0, volume_one_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(volume_one_name.volume, Some(1));
    assert!(!volume_one_name.partial);
    assert_eq!(completed_volume_one.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 2 })
    ));
    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 6);
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys
        .iter()
        .any(|key| key == COMBINED || key == ORIGINAL));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_first_incremental_volume_two_is_visible_but_cannot_satisfy_volume_two() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.p7m";
    const VOLUME_ONE: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains first-incremental volume 2");
    let (renamed_volume_two, renamed_volume_two_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains first-incremental volume 2");
    renamed_volume_two.key = UNSUPPORTED.into();
    assert_eq!(renamed_volume_two_bytes, &original_volume_two_bytes);
    assert_eq!(renamed_volume_two.size, original_volume_two.size);
    assert_eq!(renamed_volume_two.identity, original_volume_two.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains first-incremental volume 2");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported volume remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("volume key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(unsupported_name.volume, Some(2));
    assert!(!unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let completed_volume_one = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_ONE)
        .expect("completed first-incremental volume 1 remains visible");
    let volume_one_name = completed_volume_one.name().expect("volume key is parsed");
    assert_eq!(
        volume_one_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (volume_one_name.start.0, volume_one_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(volume_one_name.volume, Some(1));
    assert!(!volume_one_name.partial);
    assert_eq!(completed_volume_one.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 2 })
    ));
    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 6);
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys
        .iter()
        .any(|key| key == UNSUPPORTED || key == ORIGINAL));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_partial_full_signature_archive_preserves_plain_snapshot_and_required_open_set() {
    const ORIGINAL: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const UNSUPPORTED: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.part.gz";
    const FIRST_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";
    const SECOND_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the full-signatures archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the full-signatures archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the full-signatures archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed partial full-signatures archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::FullSignatures
    );
    assert_eq!(unsupported_name.start, Timestamp(1700000000));
    assert_eq!(unsupported_name.volume, None);
    assert!(unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Gzip
    );
    assert_eq!(unsupported_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect(
            "the unsupported first-incremental signature archive does not block plain restoration",
        );

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == ORIGINAL
            || key == FIRST_INCREMENTAL_SIGNATURES
            || key == SECOND_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
#[test]
fn official_pkcs7_full_manifest_is_visible_but_cannot_be_a_chain_anchor() {
    const ORIGINAL: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const UNSUPPORTED: &str = "backup/duplicity-full.20231114T221320Z.manifest.p7m";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let original_keys = {
        let mut keys = original_inventory
            .iter()
            .map(|(object, _)| object.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    let (original_manifest, original_manifest_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the full manifest");
    let (renamed_manifest, renamed_manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the full manifest");
    renamed_manifest.key = UNSUPPORTED.into();
    assert_eq!(renamed_manifest_bytes, &original_manifest_bytes);
    assert_eq!(renamed_manifest.size, original_manifest.size);
    assert_eq!(renamed_manifest.identity, original_manifest.identity);

    let mut expected_keys = original_keys;
    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the full manifest");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported manifest remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("key is parsed");
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 2);
    assert!(!descriptors
        .iter()
        .any(|(_, _, descriptor)| descriptor.kind() == SetKind::Full));
    let mut incremental_intervals = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            assert_eq!(descriptor.kind(), SetKind::Incremental);
            (descriptor.start().0, descriptor.end().0)
        })
        .collect::<Vec<_>>();
    incremental_intervals.sort_unstable();
    assert_eq!(
        incremental_intervals,
        vec![(1700000000, 1700086400), (1700086400, 1700172800)]
    );

    let (first_index, _, first_descriptor) = descriptors
        .first()
        .expect("both incremental descriptors remain in the public iterator");
    assert_eq!(first_descriptor.kind(), SetKind::Incremental);
    assert!(matches!(
        catalog.read_snapshot(
            *first_index,
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::Chain(ChainError::InvalidAnchor))
    ));
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_partial_final_incremental_signature_archive_preserves_plain_snapshot_and_required_open_set(
) {
    const ORIGINAL: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.part.gz";
    const FULL_SIGNATURES: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const FIRST_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the final-incremental signature archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the final-incremental signature archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the final-incremental signature archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed partial final-incremental signature archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalSignatures
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700086400, 1700172800)
    );
    assert_eq!(unsupported_name.volume, None);
    assert!(unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Gzip
    );
    assert_eq!(unsupported_entry.status(), DiscoveryStatus::Partial);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect(
            "the unsupported first-incremental signature archive does not block plain restoration",
        );

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == ORIGINAL
            || key == FULL_SIGNATURES
            || key == FIRST_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_pkcs7_final_incremental_signature_archive_preserves_plain_snapshot_and_required_open_set(
) {
    const ORIGINAL: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.p7m";
    const FULL_SIGNATURES: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const FIRST_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the final-incremental signature archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the final-incremental signature archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the final-incremental signature archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported final-incremental signature archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalSignatures
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700086400, 1700172800)
    );
    assert_eq!(unsupported_name.volume, None);
    assert!(!unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect(
            "the unsupported first-incremental signature archive does not block plain restoration",
        );

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == ORIGINAL
            || key == FULL_SIGNATURES
            || key == FIRST_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_full_volume_is_visible_but_cannot_satisfy_volume_two() {
    const ORIGINAL: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const UNSUPPORTED: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.p7m";
    const VOLUME_THREE: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let original_keys = {
        let mut keys = original_inventory
            .iter()
            .map(|(object, _)| object.key.clone())
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    let (original_volume_two, original_volume_two_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains full volume 2");
    let (original_volume_three, original_volume_three_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == VOLUME_THREE)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains full volume 3");
    let (renamed_volume_two, renamed_volume_two_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains full volume 2");
    renamed_volume_two.key = UNSUPPORTED.into();
    assert_eq!(renamed_volume_two_bytes, &original_volume_two_bytes);
    assert_eq!(renamed_volume_two.size, original_volume_two.size);
    assert_eq!(renamed_volume_two.identity, original_volume_two.identity);

    let mut expected_keys = original_keys;
    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains full volume 2");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported volume remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::FullVolume
    );
    assert_eq!(unsupported_name.volume, Some(2));
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let completed_volume_three = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == VOLUME_THREE)
        .expect("completed volume 3 remains visible in discovery");
    let volume_three_name = completed_volume_three.name().expect("volume key is parsed");
    assert_eq!(
        volume_three_name.kind,
        duplicity_core::names::ObjectKind::FullVolume
    );
    assert_eq!(volume_three_name.volume, Some(3));
    assert!(!volume_three_name.partial);
    assert_eq!(completed_volume_three.status(), DiscoveryStatus::Completed);
    assert_eq!(completed_volume_three.object(), &original_volume_three);
    assert_eq!(
        source
            .objects
            .iter()
            .find(|(object, _)| object.key == VOLUME_THREE)
            .unwrap()
            .1,
        original_volume_three_bytes
    );

    assert!(matches!(
        catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::MissingVolume { number: 2 })
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
//= docs/parsing-requirements.md#parse-chain-002
//= type=test
//# The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.
#[test]
fn official_pkcs7_final_incremental_volume_preserves_prior_snapshot_and_reports_missing_final() {
    const ORIGINAL: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.p7m";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_volume, original_volume_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains final-incremental volume 1");
    let (renamed_volume, renamed_volume_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains final-incremental volume 1");
    renamed_volume.key = UNSUPPORTED.into();
    assert_eq!(renamed_volume_bytes, &original_volume_bytes);
    assert_eq!(renamed_volume.size, original_volume.size);
    assert_eq!(renamed_volume.identity, original_volume.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains final-incremental volume 1");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported final volume remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("volume key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalVolume
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700086400, 1700172800)
    );
    assert_eq!(unsupported_name.volume, Some(1));
    assert!(!unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, full_descriptor) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    assert_eq!(full_descriptor.start().0, 1700000000);
    assert_eq!(full_descriptor.end().0, 1700000000);

    let prior = catalog
        .read_snapshot(*full_index, Timestamp(1700086400), &Cancellation::default())
        .expect("the completed chain through the prior date remains restorable");
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|snapshot| snapshot["mode"] == "plain" && snapshot["index"] == 1)
        .expect("official oracle contains the prior plain snapshot");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|exclusion| exclusion == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"].as_array().unwrap();
    assert_eq!(prior.state().len(), expected_entries.len());
    let mut expected_rows = expected_entries
        .iter()
        .map(|expected| {
            (
                expected["components_hex"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|component| decode_oracle_hex(component.as_str().unwrap()))
                    .collect::<Vec<_>>(),
                expected,
            )
        })
        .collect::<Vec<_>>();
    expected_rows.sort_by(|left, right| left.0.cmp(&right.0));
    let actual_rows = prior.state().entries().collect::<Vec<_>>();
    assert_eq!(actual_rows.len(), expected_rows.len());
    for ((path, entry), (expected_components, expected)) in
        actual_rows.into_iter().zip(expected_rows.iter())
    {
        assert_eq!(
            path.components()
                .iter()
                .map(|component| component.as_slice())
                .collect::<Vec<_>>(),
            expected_components
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            entry.mode() & 0o777,
            expected["mode"].as_u64().unwrap() as u32
        );
        let kind = expected["kind"].as_str().unwrap();
        if kind != "symlink" {
            let mtime_ns = expected["mtime_ns"].as_i64().unwrap();
            assert_eq!(entry.mtime().seconds, mtime_ns.div_euclid(1_000_000_000));
            assert_eq!(
                i64::from(entry.mtime().nanoseconds),
                mtime_ns.rem_euclid(1_000_000_000)
            );
        }
        match kind {
            "file" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Regular);
                assert_eq!(
                    entry.content().unwrap().len() as u64,
                    expected["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(entry.content().unwrap())),
                    expected["sha256"].as_str().unwrap()
                );
            }
            "directory" => assert_eq!(
                entry.entry_type(),
                duplicity_core::tar::EntryType::Directory
            ),
            "symlink" => {
                assert_eq!(entry.entry_type(), duplicity_core::tar::EntryType::Symlink);
                assert_eq!(
                    entry.linkname(),
                    decode_oracle_hex(expected["symlink_target_hex"].as_str().unwrap())
                );
            }
            other => panic!("unexpected fixture kind {other}"),
        }
    }

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::MissingVolume { number: 1 })
    ));
    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 8);
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| key == UNSUPPORTED));
}

#[test]
fn discovery_diagnostics_crypto_and_limits_remain_distinct_from_validation() {
    let mut source = Source::official();
    for name in [
        "note.txt",
        "duplicity-full.invalid.manifest",
        "duplicity-full.20231114T221320Z.manifest.part",
        "duplicity-full.20241114T221320Z.manifest.gpg",
        "duplicity-full.20241114T221320Z.manifest.p7m",
    ] {
        source.add(&format!("backup/{name}"), Vec::new());
    }
    let catalog = discover(&source);
    for status in [
        DiscoveryStatus::Unknown,
        DiscoveryStatus::Partial,
        DiscoveryStatus::UnsupportedEnvelope,
    ] {
        assert!(catalog
            .objects()
            .iter()
            .any(|object| object.status() == status));
    }
    assert!(catalog
        .objects()
        .iter()
        .any(|object| matches!(object.status(), DiscoveryStatus::Malformed(_))));
    let encrypted = catalog
        .manifests()
        .find(|(_, object, _)| object.key.ends_with(".gpg"))
        .unwrap()
        .0;
    assert!(matches!(
        catalog.read_snapshot(encrypted, Timestamp(1731622400), &Cancellation::default()),
        Err(ReaderError::Envelope(_))
    ));
    for limits in [
        ReaderLimits {
            catalog_metadata_bytes: 0,
            ..ReaderLimits::default()
        },
        ReaderLimits {
            source: SourceLimits {
                max_objects: 0,
                ..SourceLimits::default()
            },
            ..ReaderLimits::default()
        },
    ] {
        assert!(matches!(
            Catalog::discover(
                &source,
                "backup/",
                NameConfig::default(),
                limits,
                &Cancellation::default()
            ),
            Err(ReaderError::Limit)
        ));
    }
    let catalog = Catalog::discover(
        &source,
        "backup/",
        NameConfig::default(),
        ReaderLimits {
            stored_buffer_bytes: 0,
            ..ReaderLimits::default()
        },
        &Cancellation::default(),
    )
    .unwrap();
    assert!(matches!(
        catalog.read_snapshot(
            anchor(&catalog),
            Timestamp(1700000000),
            &Cancellation::default()
        ),
        Err(ReaderError::Limit)
    ));
}

#[test]
//= docs/parsing-requirements.md#parse-gen-005
//= type=test
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
fn reader_budget_edges_accept_exact_and_reject_one_less() {
    let source = Source::official();
    let full_manifest = source
        .objects
        .iter()
        .find(|(object, _)| {
            object
                .key
                .ends_with("duplicity-full.20231114T221320Z.manifest")
        })
        .unwrap();
    let full_volumes = source
        .objects
        .iter()
        .filter(|(object, _)| {
            object.key.contains("duplicity-full.") && object.key.ends_with(".difftar.gz")
        })
        .count();
    assert_eq!(full_volumes, 3); // original producer inventory, not parsed hints
    let comparisons = source.objects.len() * full_volumes;
    let normalization = 4 * 2 * std::mem::size_of::<usize>();
    for (exact, smaller) in [
        (
            ReaderLimits {
                max_object_checks: comparisons,
                ..Default::default()
            },
            ReaderLimits {
                max_object_checks: comparisons - 1,
                ..Default::default()
            },
        ),
        (
            ReaderLimits {
                decoded_manifest_bytes: full_manifest.1.len(),
                ..Default::default()
            },
            ReaderLimits {
                decoded_manifest_bytes: full_manifest.1.len() - 1,
                ..Default::default()
            },
        ),
        (
            ReaderLimits {
                normalization_metadata_bytes: normalization,
                ..Default::default()
            },
            ReaderLimits {
                normalization_metadata_bytes: normalization - 1,
                ..Default::default()
            },
        ),
    ] {
        let exact = Catalog::discover(
            &source,
            "backup/",
            NameConfig::default(),
            exact,
            &Cancellation::default(),
        )
        .unwrap();
        assert!(exact
            .read_snapshot(
                anchor(&exact),
                Timestamp(1700000000),
                &Cancellation::default()
            )
            .is_ok());
        let smaller = Catalog::discover(
            &source,
            "backup/",
            NameConfig::default(),
            smaller,
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(
            smaller.read_snapshot(
                anchor(&smaller),
                Timestamp(1700000000),
                &Cancellation::default()
            ),
            Err(ReaderError::Limit | ReaderError::Envelope(_))
        ));
    }
}

#[test]
//= docs/parsing-requirements.md#parse-gen-005
//= type=test
//# The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.
fn catalog_metadata_budget_rejects_before_catalog_construction() {
    let source = Source::official();
    assert!(matches!(
        Catalog::discover(
            &source,
            "backup/",
            NameConfig::default(),
            ReaderLimits {
                catalog_metadata_bytes: 1,
                ..ReaderLimits::default()
            },
            &Cancellation::default(),
        ),
        Err(ReaderError::Limit)
    ));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_pkcs7_first_incremental_signature_archive_preserves_plain_snapshot_and_required_open_set(
) {
    const ORIGINAL: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz";
    const UNSUPPORTED: &str =
        "backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.p7m";
    const FULL_SIGNATURES: &str = "backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz";
    const FINAL_INCREMENTAL_SIGNATURES: &str =
        "backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let mut expected_keys = original_inventory
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    expected_keys.sort();
    let (original_archive, original_archive_bytes) = source
        .objects
        .iter()
        .find(|(object, _)| object.key == ORIGINAL)
        .map(|(object, bytes)| (object.clone(), bytes.clone()))
        .expect("official fixture contains the first-incremental signature archive");
    let (renamed_archive, renamed_archive_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == ORIGINAL)
        .expect("official fixture contains the first-incremental signature archive");
    renamed_archive.key = UNSUPPORTED.into();
    assert_eq!(renamed_archive_bytes, &original_archive_bytes);
    assert_eq!(renamed_archive.size, original_archive.size);
    assert_eq!(renamed_archive.identity, original_archive.identity);

    let old_key = expected_keys
        .iter_mut()
        .find(|key| key.as_str() == ORIGINAL)
        .expect("original inventory contains the first-incremental signature archive");
    *old_key = UNSUPPORTED.into();
    expected_keys.sort();
    let mut actual_keys = source
        .objects
        .iter()
        .map(|(object, _)| object.key.clone())
        .collect::<Vec<_>>();
    actual_keys.sort();
    assert_eq!(actual_keys, expected_keys);
    assert_eq!(source.objects.len(), 12);
    for (original_object, original_bytes) in &original_inventory {
        let expected_key = if original_object.key == ORIGINAL {
            UNSUPPORTED
        } else {
            original_object.key.as_str()
        };
        let (actual_object, actual_bytes) = source
            .objects
            .iter()
            .find(|(object, _)| object.key == expected_key)
            .expect("each original object remains under its expected key");
        let mut expected_object = original_object.clone();
        expected_object.key = expected_key.into();
        assert_eq!(actual_object, &expected_object);
        assert_eq!(actual_bytes, original_bytes);
    }

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("renamed unsupported first-incremental signature archive remains visible");
    let unsupported_name = unsupported_entry.name().expect("archive name is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::IncrementalSignatures
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(unsupported_name.volume, None);
    assert!(!unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect(
            "the unsupported first-incremental signature archive does not block plain restoration",
        );

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), expected_opened_keys.len());
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| {
        key == UNSUPPORTED
            || key == ORIGINAL
            || key == FULL_SIGNATURES
            || key == FINAL_INCREMENTAL_SIGNATURES
    }));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#parse-state-003
//= type=test
//# `SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_unknown_and_malformed_siblings_preserve_plain_snapshot_and_required_open_set() {
    // The ordering assertion below is limited to this observed fixture
    // snapshot; it does not prove standard-library map internals.
    const UNKNOWN: &str = "backup/notes.txt";
    const MALFORMED: &str = "backup/duplicity-full.invalid.manifest";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    source.add(UNKNOWN, Vec::new());
    source.add(MALFORMED, Vec::new());
    assert_eq!(source.objects.len(), 14);
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        assert_eq!(&source.objects[index].0, original_object);
        assert_eq!(&source.objects[index].1, original_bytes);
    }
    let unknown_object = &source.objects[12].0;
    assert_eq!(unknown_object.key, UNKNOWN);
    assert_eq!(unknown_object.size, 0);
    let malformed_object = &source.objects[13].0;
    assert_eq!(malformed_object.key, MALFORMED);
    assert_eq!(malformed_object.size, 0);
    assert!(source.objects[12].1.is_empty());
    assert!(source.objects[13].1.is_empty());

    let catalog = discover(&source);
    let unknown_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNKNOWN)
        .expect("unknown sibling remains visible in discovery");
    assert_eq!(unknown_entry.status(), DiscoveryStatus::Unknown);
    let malformed_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == MALFORMED)
        .expect("malformed sibling remains visible in discovery");
    assert!(matches!(
        malformed_entry.status(),
        DiscoveryStatus::Malformed(_)
    ));

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect("unknown and malformed siblings do not prevent a valid snapshot read");

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys
        .iter()
        .any(|key| key == UNKNOWN || key == MALFORMED || key.ends_with(".sigtar.gz")));
}

//= docs/parsing-requirements.md#parse-name-001
//= type=test
//# The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.
//= docs/parsing-requirements.md#parse-chain-001
//= type=test
//# A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.
//= docs/parsing-requirements.md#parse-path-001
//= type=test
//# The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.
//= docs/parsing-requirements.md#assure-test-001
//= type=test
//# Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.
#[test]
fn official_unsupported_semantic_duplicate_preserves_plain_snapshot_and_required_open_set() {
    const SUPPORTED: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz";
    const UNSUPPORTED: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.p7m";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    source.add(UNSUPPORTED, Vec::new());
    assert_eq!(source.objects.len(), 13);
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        assert_eq!(&source.objects[index].0, original_object);
        assert_eq!(&source.objects[index].1, original_bytes);
    }
    let added_object = &source.objects[12].0;
    assert_eq!(added_object.key, UNSUPPORTED);
    assert_eq!(added_object.size, 0);
    assert!(source.objects[12].1.is_empty());

    let catalog = discover(&source);
    let unsupported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == UNSUPPORTED)
        .expect("unsupported semantic duplicate remains visible in discovery");
    let unsupported_name = unsupported_entry.name().expect("unsupported key is parsed");
    assert_eq!(
        unsupported_name.kind,
        duplicity_core::names::ObjectKind::FullVolume
    );
    assert_eq!(
        (unsupported_name.start.0, unsupported_name.end.0),
        (1700000000, 1700000000)
    );
    assert_eq!(unsupported_name.volume, Some(1));
    assert!(!unsupported_name.partial);
    assert_eq!(
        unsupported_name.envelope,
        duplicity_core::names::Envelope::Pkcs7
    );
    assert_eq!(
        unsupported_entry.status(),
        DiscoveryStatus::UnsupportedEnvelope
    );

    let supported_entry = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == SUPPORTED)
        .expect("official gzip sibling remains visible in discovery");
    let supported_name = supported_entry.name().expect("supported key is parsed");
    assert_eq!(
        supported_name.kind,
        duplicity_core::names::ObjectKind::FullVolume
    );
    assert_eq!(
        (supported_name.start.0, supported_name.end.0),
        (unsupported_name.start.0, unsupported_name.end.0)
    );
    assert_eq!(supported_name.volume, unsupported_name.volume);
    assert!(!supported_name.partial);
    assert_eq!(
        supported_name.envelope,
        duplicity_core::names::Envelope::Gzip
    );
    assert_eq!(supported_entry.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let (full_index, _, _) = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the completed full manifest remains a public anchor");
    let snapshot = catalog
        .read_snapshot(*full_index, Timestamp(1700172800), &Cancellation::default())
        .expect("unknown and malformed siblings do not prevent a valid snapshot read");

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        "backup/duplicity-full.20231114T221320Z.manifest",
        "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz",
        "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz",
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest",
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz",
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| key == UNSUPPORTED));
}

//= docs/parsing-requirements.md#parse-gen-001
//= type=test
//# A parser MUST return a structured error for malformed or truncated required input.
#[test]
fn official_empty_required_full_manifest_returns_structured_malformed_error() {
    const FULL_MANIFEST: &str = "backup/duplicity-full.20231114T221320Z.manifest";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let (manifest_object, manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == FULL_MANIFEST)
        .expect("the official fixture includes its required full manifest");
    manifest_bytes.clear();
    manifest_object.size = 0;
    let empty_body_etag = format!("{:x}", sha2::Sha256::digest([]));
    manifest_object.identity = ObjectIdentity {
        version_id: Some("empty-malformed-body-v1".into()),
        etag: Some(empty_body_etag.clone()),
    };

    assert_eq!(source.objects.len(), original_inventory.len());
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        let (object, bytes) = &source.objects[index];
        if original_object.key == FULL_MANIFEST {
            assert_eq!(object.key, original_object.key);
            assert_eq!(object.size, 0);
            assert_eq!(
                object.identity.version_id.as_deref(),
                Some("empty-malformed-body-v1")
            );
            assert_eq!(
                object.identity.etag.as_deref(),
                Some(empty_body_etag.as_str())
            );
            assert!(bytes.is_empty());
        } else {
            assert_eq!(object, original_object);
            assert_eq!(bytes, original_bytes);
        }
    }

    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
    let discovered_full = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == FULL_MANIFEST)
        .expect("the valid object name remains visible during metadata-only discovery");
    let full_name = discovered_full
        .name()
        .expect("the Full key has a valid name");
    assert_eq!(
        full_name.kind,
        duplicity_core::names::ObjectKind::FullManifest
    );
    assert_eq!(full_name.start.0, 1700000000);
    assert_eq!(discovered_full.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let full_index = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the name-derived Full descriptor remains available")
        .0;

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Manifest(
            duplicity_core::manifest::ManifestError::Malformed
        ))
    ));
    let opened_keys = source.opened_keys.lock().unwrap().clone();
    assert_eq!(opened_keys, vec![FULL_MANIFEST]);
    assert_eq!(source.opens.load(Ordering::SeqCst), 1);
}

//= docs/parsing-requirements.md#parse-gen-001
//= type=test
//# A parser MUST return a structured error for malformed or truncated required input.
#[test]
fn official_empty_required_first_incremental_manifest_returns_structured_malformed_error() {
    const FIRST_INCREMENTAL: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";
    const FULL_MANIFEST: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const FULL_VOL1: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz";
    const FULL_VOL2: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const FULL_VOL3: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let (manifest_object, manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == FIRST_INCREMENTAL)
        .expect("the official fixture includes its first incremental manifest");
    manifest_bytes.clear();
    manifest_object.size = 0;
    let empty_body_etag = format!("{:x}", sha2::Sha256::digest([]));
    manifest_object.identity = ObjectIdentity {
        version_id: Some("empty-malformed-body-v1".into()),
        etag: Some(empty_body_etag.clone()),
    };

    assert_eq!(source.objects.len(), original_inventory.len());
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        let (object, bytes) = &source.objects[index];
        if original_object.key == FIRST_INCREMENTAL {
            assert_eq!(object.key, original_object.key);
            assert_eq!(object.size, 0);
            assert_eq!(
                object.identity.version_id.as_deref(),
                Some("empty-malformed-body-v1")
            );
            assert_eq!(
                object.identity.etag.as_deref(),
                Some(empty_body_etag.as_str())
            );
            assert!(bytes.is_empty());
        } else {
            assert_eq!(object, original_object);
            assert_eq!(bytes, original_bytes);
        }
    }

    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
    let discovered_incremental = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == FIRST_INCREMENTAL)
        .expect("the valid object name remains visible in metadata-only discovery");
    let incremental_name = discovered_incremental
        .name()
        .expect("the incremental key has a valid name");
    assert_eq!(
        incremental_name.kind,
        duplicity_core::names::ObjectKind::IncrementalManifest
    );
    assert_eq!(
        (incremental_name.start.0, incremental_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(discovered_incremental.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let full_index = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the name-derived Full descriptor is available")
        .0;

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Manifest(
            duplicity_core::manifest::ManifestError::Malformed
        ))
    ));
    let mut expected_opened_keys = vec![
        FULL_MANIFEST,
        FULL_VOL1,
        FULL_VOL2,
        FULL_VOL3,
        FIRST_INCREMENTAL,
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 5);
    assert_eq!(opened_keys, expected_opened_keys);
    assert_eq!(source.opens.load(Ordering::SeqCst), 5);
}

//= docs/parsing-requirements.md#parse-env-005
//= type=test
//# Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.
#[test]
fn official_incremental_volume_stored_digest_mismatch_returns_no_snapshot() {
    const MUTATED_VOLUME: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";
    const FULL_MANIFEST: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const FULL_VOL1: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz";
    const FULL_VOL2: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const FULL_VOL3: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";
    const FIRST_INCREMENTAL_MANIFEST: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let (volume_object, volume_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == MUTATED_VOLUME)
        .expect("the official fixture includes first-incremental volume 1");
    assert!(!volume_bytes.is_empty());
    let original_len = volume_bytes.len();
    let mut expected_mutated_bytes = volume_bytes.clone();
    expected_mutated_bytes[0] ^= 1;
    volume_bytes[0] ^= 1;
    assert_eq!(volume_bytes, &expected_mutated_bytes);
    assert_eq!(volume_bytes.len(), original_len);
    let mutated_etag = format!("{:x}", sha2::Sha256::digest(volume_bytes.as_slice()));
    volume_object.identity = ObjectIdentity {
        version_id: Some("mutated-volume-body-v1".into()),
        etag: Some(mutated_etag.clone()),
    };

    assert_eq!(source.objects.len(), original_inventory.len());
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        let (object, bytes) = &source.objects[index];
        if original_object.key == MUTATED_VOLUME {
            assert_eq!(object.key, original_object.key);
            assert_eq!(object.size, original_object.size);
            assert_eq!(bytes.len(), original_bytes.len());
            assert_eq!(bytes, &expected_mutated_bytes);
            assert_eq!(
                object.identity.version_id.as_deref(),
                Some("mutated-volume-body-v1")
            );
            assert_eq!(object.identity.etag.as_deref(), Some(mutated_etag.as_str()));
        } else {
            assert_eq!(object, original_object);
            assert_eq!(bytes, original_bytes);
        }
    }

    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let full_index = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the Full descriptor is the selected history anchor")
        .0;

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Assembly(
            duplicity_core::multipart::AssemblyError::Envelope(
                duplicity_core::envelope::EnvelopeError::DigestMismatch
            )
        ))
    ));
    let mut expected_opened_keys = vec![
        FULL_MANIFEST,
        FULL_VOL1,
        FULL_VOL2,
        FULL_VOL3,
        FIRST_INCREMENTAL_MANIFEST,
        MUTATED_VOLUME,
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 6);
    assert_eq!(opened_keys, expected_opened_keys);
    assert_eq!(source.opens.load(Ordering::SeqCst), 6);
}

//= docs/parsing-requirements.md#parse-env-006
//= type=test
//# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
#[test]
fn official_incremental_unsupported_digest_fails_before_volume_open() {
    const FIRST_INCREMENTAL_MANIFEST: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";
    const FULL_MANIFEST: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const FULL_VOL1: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz";
    const FULL_VOL2: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const FULL_VOL3: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let (manifest_object, manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == FIRST_INCREMENTAL_MANIFEST)
        .expect("the official fixture includes its first incremental manifest");
    let original_size = manifest_object.size;
    let volume_one_header = manifest_bytes
        .windows(b"Volume 1:".len())
        .position(|window| window == b"Volume 1:")
        .expect("the official manifest starts its Volume 1 section");
    let algorithm_relative = manifest_bytes[volume_one_header + b"Volume 1:".len()..]
        .windows(b"Hash SHA1 ".len())
        .position(|window| window == b"Hash SHA1 ")
        .expect("Volume 1 declares its SHA1 digest")
        + volume_one_header
        + b"Volume 1:".len();
    let algorithm_start = algorithm_relative + b"Hash ".len();
    assert_eq!(
        &manifest_bytes[algorithm_start..algorithm_start + 4],
        b"SHA1"
    );
    manifest_bytes[algorithm_start..algorithm_start + 4].copy_from_slice(b"SHA2");
    assert_eq!(manifest_object.size, original_size);
    let updated_etag = format!("{:x}", sha2::Sha256::digest(manifest_bytes.as_slice()));
    manifest_object.identity = ObjectIdentity {
        version_id: Some("unsupported-volume-digest-v1".into()),
        etag: Some(updated_etag.clone()),
    };

    assert_eq!(source.objects.len(), original_inventory.len());
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        let (object, bytes) = &source.objects[index];
        if original_object.key == FIRST_INCREMENTAL_MANIFEST {
            assert_eq!(object.key, original_object.key);
            assert_eq!(object.size, original_object.size);
            assert_eq!(bytes.len(), original_bytes.len());
            assert_eq!(&bytes[algorithm_start..algorithm_start + 4], b"SHA2");
            assert_eq!(
                object.identity.version_id.as_deref(),
                Some("unsupported-volume-digest-v1")
            );
            assert_eq!(object.identity.etag.as_deref(), Some(updated_etag.as_str()));
            let changed = bytes
                .iter()
                .zip(original_bytes)
                .enumerate()
                .filter(|(_, (new, old))| new != old)
                .map(|(offset, _)| offset)
                .collect::<Vec<_>>();
            let expected_changed = (algorithm_start..algorithm_start + 4)
                .filter(|offset| b"SHA2"[offset - algorithm_start] != original_bytes[*offset])
                .collect::<Vec<_>>();
            assert_eq!(
                changed, expected_changed,
                "only byte differences within the digest algorithm token are allowed"
            );
        } else {
            assert_eq!(object, original_object);
            assert_eq!(bytes, original_bytes);
        }
    }

    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
    let discovered_manifest = catalog
        .objects()
        .iter()
        .find(|entry| entry.object().key == FIRST_INCREMENTAL_MANIFEST)
        .expect("the first incremental manifest remains visible during discovery");
    let parsed_name = discovered_manifest
        .name()
        .expect("the incremental manifest key has a valid name");
    assert_eq!(
        parsed_name.kind,
        duplicity_core::names::ObjectKind::IncrementalManifest
    );
    assert_eq!(
        (parsed_name.start.0, parsed_name.end.0),
        (1700000000, 1700086400)
    );
    assert_eq!(discovered_manifest.status(), DiscoveryStatus::Completed);

    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let full_index = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the Full descriptor is the selected history anchor")
        .0;

    source.opened_keys.lock().unwrap().clear();
    assert!(matches!(
        catalog.read_snapshot(full_index, Timestamp(1700172800), &Cancellation::default()),
        Err(ReaderError::Manifest(
            duplicity_core::manifest::ManifestError::UnsupportedIntegrity
        ))
    ));
    let mut expected_opened_keys = vec![
        FULL_MANIFEST,
        FULL_VOL1,
        FULL_VOL2,
        FULL_VOL3,
        FIRST_INCREMENTAL_MANIFEST,
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 5);
    assert_eq!(opened_keys, expected_opened_keys);
    assert_eq!(source.opens.load(Ordering::SeqCst), 5);
}

//= docs/parsing-requirements.md#parse-env-006
//= type=test
//# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
//= docs/parsing-requirements.md#parse-state-003
//= type=test
//# `SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components.
#[test]
fn official_incremental_supported_digest_ignores_unknown_sibling() {
    const FIRST_INCREMENTAL_MANIFEST: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest";
    const FULL_MANIFEST: &str = "backup/duplicity-full.20231114T221320Z.manifest";
    const FULL_VOL1: &str = "backup/duplicity-full.20231114T221320Z.vol1.difftar.gz";
    const FULL_VOL2: &str = "backup/duplicity-full.20231114T221320Z.vol2.difftar.gz";
    const FULL_VOL3: &str = "backup/duplicity-full.20231114T221320Z.vol3.difftar.gz";
    const FIRST_INCREMENTAL_VOL1: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz";
    const FIRST_INCREMENTAL_VOL2: &str =
        "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz";
    const FINAL_INCREMENTAL_MANIFEST: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest";
    const FINAL_INCREMENTAL_VOL1: &str =
        "backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz";

    let mut source = Source::official();
    assert_eq!(source.objects.len(), 12);
    let original_inventory = source.objects.clone();
    let (manifest_object, manifest_bytes) = source
        .objects
        .iter_mut()
        .find(|(object, _)| object.key == FIRST_INCREMENTAL_MANIFEST)
        .expect("the official fixture includes its first incremental manifest");
    let volume_one_header = manifest_bytes
        .windows(b"Volume 1:".len())
        .position(|window| window == b"Volume 1:")
        .expect("the manifest starts its Volume 1 section");
    let declaration_relative = manifest_bytes[volume_one_header + b"Volume 1:".len()..]
        .windows(b"Hash SHA1 ".len())
        .position(|window| window == b"Hash SHA1 ")
        .expect("Volume 1 declares a supported SHA1 digest")
        + volume_one_header
        + b"Volume 1:".len();
    let digest_start = declaration_relative + b"Hash SHA1 ".len();
    let digest_end = digest_start + 40;
    assert!(manifest_bytes[digest_start..digest_end]
        .iter()
        .all(u8::is_ascii_hexdigit));
    let declaration_end = manifest_bytes[digest_end..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| digest_end + offset + 1)
        .expect("the supported digest declaration ends with LF");
    let line_start = manifest_bytes[..declaration_relative]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |offset| offset + 1);
    let indentation = &manifest_bytes[line_start..declaration_relative];
    let mut inserted_line = Vec::with_capacity(indentation.len() + 10 + 40 + 1);
    inserted_line.extend_from_slice(indentation);
    inserted_line.extend_from_slice(b"Hash SHA2 ");
    inserted_line.extend_from_slice(&manifest_bytes[digest_start..digest_end]);
    inserted_line.push(b'\n');
    assert_eq!(inserted_line.len(), indentation.len() + 51);
    assert!(inserted_line.ends_with(b"\n"));

    let original_size = manifest_object.size;
    let original_sha1 = manifest_bytes[declaration_relative..declaration_end].to_vec();
    let original_digest = manifest_bytes[digest_start..digest_end].to_vec();
    let mut expected_mutated_body = manifest_bytes.clone();
    expected_mutated_body.splice(declaration_end..declaration_end, inserted_line.clone());
    manifest_bytes.splice(declaration_end..declaration_end, inserted_line.clone());
    assert_eq!(*manifest_bytes, expected_mutated_body);
    assert_eq!(
        manifest_bytes[declaration_relative..declaration_end],
        original_sha1
    );
    assert_eq!(
        &manifest_bytes[declaration_end..declaration_end + inserted_line.len()],
        inserted_line.as_slice()
    );
    assert_eq!(
        &inserted_line[inserted_line.len() - original_digest.len() - 1..inserted_line.len() - 1],
        original_digest.as_slice()
    );
    manifest_object.size =
        u64::try_from(manifest_bytes.len()).expect("mutated manifest size fits u64");
    assert_eq!(
        manifest_object.size,
        original_size + inserted_line.len() as u64
    );
    let updated_etag = format!("{:x}", sha2::Sha256::digest(manifest_bytes.as_slice()));
    manifest_object.identity = ObjectIdentity {
        version_id: Some("supported-volume-digest-unknown-sibling-v1".into()),
        etag: Some(updated_etag.clone()),
    };

    assert_eq!(source.objects.len(), original_inventory.len());
    for (index, (original_object, original_bytes)) in original_inventory.iter().enumerate() {
        let (object, bytes) = &source.objects[index];
        if original_object.key == FIRST_INCREMENTAL_MANIFEST {
            assert_eq!(object.key, original_object.key);
            assert_eq!(bytes, &expected_mutated_body);
            assert_eq!(object.size, bytes.len() as u64);
            assert_eq!(
                object.identity.version_id.as_deref(),
                Some("supported-volume-digest-unknown-sibling-v1")
            );
            assert_eq!(object.identity.etag.as_deref(), Some(updated_etag.as_str()));
        } else {
            assert_eq!(object, original_object);
            assert_eq!(bytes, original_bytes);
        }
    }
    let supported_volume = source
        .objects
        .iter()
        .find(|(object, _)| {
            object.key
                == "backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz"
        })
        .expect("the supported first-incremental volume remains present");
    let original_supported_volume = original_inventory
        .iter()
        .find(|(object, _)| object.key == supported_volume.0.key)
        .expect("the original inventory includes the supported volume");
    assert_eq!(supported_volume, original_supported_volume);

    let catalog = discover(&source);
    assert_eq!(source.opens.load(Ordering::SeqCst), 0);
    let descriptors = catalog.manifests().collect::<Vec<_>>();
    assert_eq!(descriptors.len(), 3);
    let mut descriptor_keys = descriptors
        .iter()
        .map(|(_, _, descriptor)| {
            (
                match descriptor.kind() {
                    SetKind::Full => 0,
                    SetKind::Incremental => 1,
                },
                descriptor.start().0,
                descriptor.end().0,
            )
        })
        .collect::<Vec<_>>();
    descriptor_keys.sort_unstable();
    assert_eq!(
        descriptor_keys,
        vec![
            (0, 1700000000, 1700000000),
            (1, 1700000000, 1700086400),
            (1, 1700086400, 1700172800),
        ]
    );
    let full_index = descriptors
        .iter()
        .find(|(_, _, descriptor)| descriptor.kind() == SetKind::Full)
        .expect("the Full descriptor is the selected history anchor")
        .0;
    let snapshot = catalog
        .read_snapshot(full_index, Timestamp(1700172800), &Cancellation::default())
        .expect("a supported SHA1 declaration validates despite its SHA2 sibling");

    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let expected_snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == 2)
        .expect("the final plain snapshot oracle exists");
    assert!(expected_snapshot["official_comparison_exclusions"]
        .as_array()
        .expect("official comparison exclusions are listed")
        .iter()
        .any(|excluded| excluded == "symlink_mtime_ns"));
    let expected_entries = expected_snapshot["entries"]
        .as_array()
        .expect("official snapshot entries are listed");
    assert_eq!(snapshot.state().len(), expected_entries.len());
    let mut expected_by_components = std::collections::BTreeMap::new();
    for expected_entry in expected_entries {
        let components = expected_entry["components_hex"]
            .as_array()
            .expect("oracle raw path components")
            .iter()
            .map(|component| decode_oracle_hex(component.as_str().unwrap()))
            .collect::<Vec<_>>();
        assert!(
            expected_by_components
                .insert(components, expected_entry)
                .is_none(),
            "oracle contains each raw component path once"
        );
    }
    let actual_entries = snapshot.state().entries().collect::<Vec<_>>();
    let actual_paths = actual_entries
        .iter()
        .map(|(path, _)| path.components().clone())
        .collect::<Vec<_>>();
    let expected_sorted_paths = expected_by_components.keys().cloned().collect::<Vec<_>>();
    // This checks the observed order for this fixture snapshot against raw
    // oracle components; it does not prove standard-library map internals.
    assert_eq!(
        actual_paths, expected_sorted_paths,
        "final snapshot paths and iteration order match raw oracle components"
    );
    for (path, entry) in actual_entries {
        let expected_entry = expected_by_components
            .get(path.components())
            .expect("every snapshot entry has an oracle record");
        let expected_kind = match expected_entry["kind"].as_str().unwrap() {
            "file" => duplicity_core::tar::EntryType::Regular,
            "directory" => duplicity_core::tar::EntryType::Directory,
            "symlink" => duplicity_core::tar::EntryType::Symlink,
            other => panic!("unexpected oracle entry kind {other}"),
        };
        assert_eq!(entry.entry_type(), expected_kind);
        assert_eq!(
            entry.mode() & 0o777,
            u32::try_from(expected_entry["mode"].as_u64().unwrap()).unwrap() & 0o777
        );
        if expected_kind != duplicity_core::tar::EntryType::Symlink {
            let mtime_ns = expected_entry["mtime_ns"]
                .as_i64()
                .expect("oracle mtime_ns fits checked signed i64");
            assert_eq!(
                entry.mtime(),
                duplicity_core::tar::TarTime {
                    seconds: mtime_ns.div_euclid(1_000_000_000),
                    nanoseconds: u32::try_from(mtime_ns.rem_euclid(1_000_000_000)).unwrap(),
                }
            );
        }
        match expected_kind {
            duplicity_core::tar::EntryType::Regular => {
                let content = entry
                    .content()
                    .expect("every oracle file has retained content");
                assert_eq!(
                    content.len() as u64,
                    expected_entry["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(content)),
                    expected_entry["sha256"].as_str().unwrap()
                );
            }
            duplicity_core::tar::EntryType::Symlink => assert_eq!(
                entry.linkname(),
                decode_oracle_hex(expected_entry["symlink_target_hex"].as_str().unwrap())
            ),
            duplicity_core::tar::EntryType::Directory => {}
            other => panic!("unexpected snapshot entry type {other:?}"),
        }
    }

    let mut expected_opened_keys = vec![
        FULL_MANIFEST,
        FULL_VOL1,
        FULL_VOL2,
        FULL_VOL3,
        FIRST_INCREMENTAL_MANIFEST,
        FIRST_INCREMENTAL_VOL1,
        FIRST_INCREMENTAL_VOL2,
        FINAL_INCREMENTAL_MANIFEST,
        FINAL_INCREMENTAL_VOL1,
    ];
    expected_opened_keys.sort_unstable();
    let mut opened_keys = source.opened_keys.lock().unwrap().clone();
    opened_keys.sort_unstable();
    assert_eq!(opened_keys.len(), 9);
    assert_eq!(opened_keys, expected_opened_keys);
    assert!(!opened_keys.iter().any(|key| key.ends_with(".sigtar.gz")));
}
