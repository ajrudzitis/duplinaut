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

struct Source {
    objects: Vec<(StoredObject, Vec<u8>)>,
    mode: usize,
    opens: AtomicUsize,
}
impl Source {
    fn official() -> Self {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain");
        let mut source = Self {
            objects: Vec::new(),
            mode: 0,
            opens: AtomicUsize::new(0),
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
