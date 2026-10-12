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
            assert!(matches!(result, Err(ReaderError::Source(SourceError::Changed))));
        } else {
            assert!(matches!(result, Err(ReaderError::Source(SourceError::Cancelled))));
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
fn missing_corrupt_and_ambiguous_objects_fail_explicitly() {
    let mut missing = Source::official();
    missing.objects.retain(|(object, _)| {
        !object.key.ends_with("vol2.difftar.gz") || !object.key.contains("full")
    });
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

#[test]
fn discovery_diagnostics_crypto_and_limits_remain_distinct_from_validation() {
    let mut source = Source::official();
    for name in [
        "note.txt",
        "duplicity-full.invalid.manifest",
        "duplicity-full.20231114T221320Z.manifest.part",
        "duplicity-full.20241114T221320Z.manifest.gpg",
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
