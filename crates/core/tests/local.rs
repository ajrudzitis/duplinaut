#![cfg(unix)]
use duplicity_core::{
    local::LocalSource,
    source::{Cancellation, ObjectSource, SourceError, SourceLimits},
};
use std::{
    fs,
    io::Read,
    os::unix::fs::symlink,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "duplicity-local-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn listed(source: &LocalSource) -> Vec<duplicity_core::source::StoredObject> {
    source
        .list("", SourceLimits::default(), &Cancellation::default())
        .unwrap()
}
// The adapter's conditional filesystem identity is a trusted I/O boundary.
#[test]
fn flat_regular_listing_and_anchored_repeated_reads() {
    let temp = Temp::new();
    fs::write(temp.0.join("backup"), b"hello").unwrap();
    fs::create_dir(temp.0.join("directory")).unwrap();
    symlink("backup", temp.0.join("link")).unwrap();
    let source = LocalSource::new(&temp.0).unwrap();
    let objects = listed(&source);
    assert_eq!(objects.len(), 1);
    assert_eq!(listed(&source), objects);
    for _ in 0..2 {
        let mut bytes = Vec::new();
        source
            .open(
                &objects[0],
                SourceLimits::default(),
                &Cancellation::default(),
            )
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, b"hello");
    }
    assert!(LocalSource::new(temp.0.join("link")).is_err());
    symlink("directory", temp.0.join("directory-link")).unwrap();
    assert!(LocalSource::new(temp.0.join("directory-link")).is_err());
    let trailing = format!("{}/directory-link/", temp.0.display());
    assert!(LocalSource::new(&trailing).is_err());
    assert!(source
        .list("../", SourceLimits::default(), &Cancellation::default())
        .is_err());
    for key in [
        "../backup",
        "/backup",
        "directory/backup",
        ".",
        "..",
        "",
        "a\0b",
        "a\\b",
    ] {
        let mut object = objects[0].clone();
        object.key = key.into();
        assert!(source
            .open(&object, SourceLimits::default(), &Cancellation::default())
            .is_err());
    }
}
//= docs/parsing-requirements.md#parse-source-001
//= type=test
//# The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.
#[test]
fn changes_replacements_and_eof_are_terminal() {
    let temp = Temp::new();
    let path = temp.0.join("backup");
    fs::write(&path, b"hello").unwrap();
    let source = LocalSource::new(&temp.0).unwrap();
    let original = listed(&source).remove(0);
    fs::write(&path, b"different").unwrap();
    assert!(matches!(
        source.open(&original, SourceLimits::default(), &Cancellation::default()),
        Err(SourceError::Changed)
    ));
    let object = listed(&source).remove(0);
    let mut stream = source
        .open(&object, SourceLimits::default(), &Cancellation::default())
        .unwrap();
    let mut bytes = vec![0; object.size as usize];
    stream.read_exact(&mut bytes).unwrap();
    fs::rename(&path, temp.0.join("old")).unwrap();
    fs::write(&path, b"different").unwrap();
    assert!(stream.read(&mut [0]).is_err());
    assert!(stream.read(&mut [0]).is_err());
    let object = listed(&source)
        .into_iter()
        .find(|x| x.key == "backup")
        .unwrap();
    let mut stream = source
        .open(&object, SourceLimits::default(), &Cancellation::default())
        .unwrap();
    fs::write(&path, b"short").unwrap();
    assert!(stream.read(&mut [0; 32]).is_err());
    fs::remove_file(&path).unwrap();
    symlink("old", &path).unwrap();
    assert!(source
        .open(&object, SourceLimits::default(), &Cancellation::default())
        .is_err());
}
#[test]
fn cancellation_limits_and_untrusted_descriptors() {
    let temp = Temp::new();
    fs::write(temp.0.join("backup"), b"hello").unwrap();
    let source = LocalSource::new(&temp.0).unwrap();
    let mut object = listed(&source).remove(0);
    let cancel = Cancellation::default();
    let mut stream = source
        .open(&object, SourceLimits::default(), &cancel)
        .unwrap();
    cancel.cancel();
    assert!(stream.read(&mut [0]).is_err());
    assert!(stream.read(&mut [0]).is_err());
    assert!(matches!(
        source.list("", SourceLimits::default(), &cancel),
        Err(SourceError::Cancelled)
    ));
    assert!(matches!(
        source.open(&object, SourceLimits::default(), &cancel),
        Err(SourceError::Cancelled)
    ));
    for limits in [
        SourceLimits {
            max_objects: 0,
            ..SourceLimits::default()
        },
        SourceLimits {
            max_metadata_bytes: 0,
            ..SourceLimits::default()
        },
        SourceLimits {
            max_stored_bytes: 0,
            ..SourceLimits::default()
        },
    ] {
        assert!(matches!(
            source.list("", limits, &Cancellation::default()),
            Err(SourceError::Limit)
        ));
    }
    assert!(matches!(
        source.open(
            &object,
            SourceLimits {
                max_stored_bytes: 0,
                ..SourceLimits::default()
            },
            &Cancellation::default()
        ),
        Err(SourceError::Limit)
    ));
    object.identity.version_id = None;
    assert!(matches!(
        source.open(&object, SourceLimits::default(), &Cancellation::default()),
        Err(SourceError::Changed)
    ));
}
#[test]
fn original_official_directory_reconstructs_three_dates() {
    use duplicity_core::{
        names::{NameConfig, Timestamp},
        reader::{Catalog, ReaderLimits},
    };
    let source = LocalSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/duplicity-3.0.7/plain"),
    )
    .unwrap();
    let cancel = Cancellation::default();
    let catalog = Catalog::discover(
        &source,
        "duplicity-",
        NameConfig::default(),
        ReaderLimits::default(),
        &cancel,
    )
    .unwrap();
    let anchor = catalog
        .manifests()
        .find(|(_, _, descriptor)| {
            descriptor.start() == Timestamp(1700000000) && descriptor.end() == Timestamp(1700000000)
        })
        .unwrap()
        .0;
    for day in 0..3 {
        let snapshot = catalog
            .read_snapshot(anchor, Timestamp(1700000000 + day * 86400), &cancel)
            .unwrap();
        assert!(!snapshot.state().is_empty());
    }
}
