use duplicity_core::manifest::{DigestAlgorithm, Manifest, ManifestError, ManifestLimits};
use duplicity_core::path::BackupPath;

fn volume(number: u64, start: &str, end: &str) -> String {
    format!(
        "Volume {number}:\n StartingPath {start}\n EndingPath {end}\n Hash SHA1 {}\n",
        "a".repeat(40)
    )
}
fn parse(text: &str) -> Result<Manifest, ManifestError> {
    Manifest::parse(text.as_bytes(), ManifestLimits::default())
}
fn path(text: &[u8]) -> BackupPath {
    BackupPath::parse(text, false, 1000, 100).unwrap()
}

//= docs/parsing-requirements.md#parse-man-001
//# The manifest parser MUST recover ordered volume records containing volume number, starting path, ending path, optional block boundaries, and declared hashes.
//= docs/parsing-requirements.md#parse-man-006
//# The reader MUST retain the distinction between absent block boundaries and explicit numeric block boundaries.
//= docs/parsing-requirements.md#parse-man-007
//= type=test
//# The manifest parser MUST accept an optional counted Filelist section before or after volume records without interpreting its lines as volume fields.
#[test]
fn official_plain_manifests_all_producers() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    fn walk(root: &std::path::Path, count: &mut usize) {
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, count);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "manifest")
            {
                let bytes = std::fs::read(&path).unwrap();
                let manifest = Manifest::parse(&bytes, ManifestLimits::default())
                    .unwrap_or_else(|error| panic!("{}: {error:?}", path.display()));
                assert!(!manifest.records.is_empty());
                assert!(!manifest.volumes().is_empty());
                assert!(manifest.filelist.is_some());
                for record in &manifest.records {
                    assert!(record.number > 0);
                    assert!(record.start.path.compare(&record.end.path) <= 0);
                    assert!(record
                        .hashes
                        .iter()
                        .any(|hash| hash.algorithm == DigestAlgorithm::Sha1));
                }
                *count += 1;
            }
        }
    }
    let mut count = 0;
    walk(&fixtures, &mut count);
    assert!(
        count >= 12,
        "expected actual manifests from all four official producers; found {count}"
    );
}

//= docs/parsing-requirements.md#parse-man-004
//= type=test
//# Manifest interpretation MUST replace earlier records of a repeated volume number with its last record and discard volume records above the last recorded volume number.
#[test]
fn restart_normalization_replaces_and_discards() {
    let text = volume(1, ".", "a")
        + &volume(2, "a 2", "b 4")
        + &volume(3, "b", "c")
        + &volume(2, "a 5", "z");
    let manifest = parse(&text).unwrap();
    assert_eq!(manifest.records.len(), 4);
    let records = manifest.volumes();
    assert_eq!(
        records
            .iter()
            .map(|record| record.number)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(records[1].start.block, Some(5));
    assert_eq!(records[1].end.block, None);
}

//= docs/parsing-requirements.md#parse-man-005
//= type=test
//# Volume selection MUST include every volume whose declared inclusive path interval can contain a selected entry or descendant, including shared multipart boundary paths.
#[test]
fn inclusive_componentwise_selection_includes_descendants_and_shared_boundaries() {
    let manifest = parse(&(volume(1, ".", "a/b 2") + &volume(2, "a/b 3", "z"))).unwrap();
    assert!(manifest
        .records
        .iter()
        .all(|record| record.may_contain(&path(b"a/b"))));
    assert!(manifest.records[0].may_contain(&path(b"a")));
    assert!(manifest.records[1].may_contain(&path(b"a")));
    // Component ordering: a! precedes a/b because "a" precedes "a!".
    let manifest = parse(&volume(1, "a/b", "a!")).unwrap();
    assert!(manifest.records[0].may_contain(&path(b"a")));
    assert!(!manifest.records[0].may_contain(&path(b"z")));
    assert!(manifest.records[0].may_contain(&path(b".")));
}

#[test]
fn filelist_positions_case_crlf_and_byte_tokens() {
    let list = "Filelist 2\n new \"quote\\x22\\xff\"\n deleted old\n";
    for text in [
        list.to_owned() + &volume(1, ".", "z"),
        volume(1, ".", "z") + list,
    ] {
        let text = text
            .replace("Volume", "vOlUmE")
            .replace("StartingPath", "sTaRtInGpAtH")
            .replace('\n', "\r\n");
        let manifest = parse(text.trim_end_matches("\r\n")).unwrap();
        assert_eq!(
            manifest.filelist.unwrap()[0].path.components(),
            &vec![b"quote\"\xff".to_vec()]
        );
    }
}

//= docs/parsing-requirements.md#parse-env-006
//= type=test
//# An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.
#[test]
fn digest_shapes_conflicts_and_unsupported_integrity() {
    let valid = volume(1, ".", "z");
    assert_eq!(
        parse(&valid.replace("SHA1", "Future")).unwrap_err(),
        ManifestError::UnsupportedIntegrity
    );
    let mixed = valid.clone() + " Hash Future abcd\n Hash MD5 0123456789ABCDEF0123456789abcdef\n";
    let manifest = parse(&mixed).unwrap();
    assert_eq!(manifest.records[0].hashes.len(), 3);
    assert_eq!(
        manifest.records[0].hashes[2].hex,
        b"0123456789abcdef0123456789abcdef"
    );
    assert!(parse(&(valid.clone() + &format!(" Hash sha1 {}\n", "b".repeat(40)))).is_err());
    assert!(parse(&valid.replace(&"a".repeat(40), "abcd")).is_err());
    assert!(parse(&valid.replace(&"a".repeat(40), &"g".repeat(40))).is_err());
    assert_eq!(
        parse(&(valid + &format!(" Hash SHA1 {}\n", "A".repeat(40))))
            .unwrap()
            .records[0]
            .hashes
            .len(),
        1
    );
}

#[test]
fn malformed_fields_paths_counts_and_boundaries_fail() {
    let valid = volume(1, ".", "z");
    for invalid in [
        volume(0, ".", "z"),
        volume(1, "z", "a"),
        volume(1, "a 0", "z"),
        volume(1, "a 18446744073709551616", "z"),
        volume(1, "\"a\"garbage", "z"),
        volume(1, "../a", "z"),
        volume(1, "\"\\x00\"", "z"),
        valid.clone() + " StartingPath .\n",
        valid.clone() + " Unknown ignored\n",
        valid.clone() + "Filelist 2\n new a\n",
        valid.clone() + "Filelist 0\n new a\n",
        valid.clone() + "Filelist 1\n modified a\n",
        valid.clone() + "Filelist 1\n new a extra\n",
        valid.clone() + "Filelist 0\nFilelist 0\n",
        "Volume 1:\n StartingPath .\n".to_owned(),
        valid.clone() + "Filelist 0\n" + &volume(2, "z", "z"),
        valid.clone().replace("Volume 1:", "Volume 1 :"),
    ] {
        assert!(parse(&invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn explicit_limits_are_enforced_without_arithmetic_wrap() {
    let bytes = volume(1, "a/b", "z").into_bytes();
    let default = ManifestLimits::default();
    for limits in [
        ManifestLimits {
            metadata_bytes: bytes.len() - 1,
            ..default
        },
        ManifestLimits {
            metadata_bytes: 1000,
            ..default
        },
        ManifestLimits {
            line_bytes: 5,
            ..default
        },
        ManifestLimits {
            entries: 1,
            ..default
        },
        ManifestLimits {
            path_bytes: 1,
            ..default
        },
        ManifestLimits {
            path_components: 1,
            ..default
        },
    ] {
        assert_eq!(
            Manifest::parse(&bytes, limits).unwrap_err(),
            ManifestError::Limit
        );
    }
    assert!(Manifest::parse(
        &bytes,
        ManifestLimits {
            metadata_bytes: usize::MAX,
            ..default
        }
    )
    .is_ok());
}

#[test]
fn official_multipart_boundaries_and_byte_path_values_are_independent_expectations() {
    let bytes = include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.manifest"
    );
    let manifest = Manifest::parse(bytes, ManifestLimits::default()).unwrap();
    assert_eq!(manifest.records.len(), 3);
    assert_eq!(manifest.records[0].start.path, path(b"."));
    assert_eq!(manifest.records[0].start.block, None);
    assert_eq!(manifest.records[0].end.path, path(b"large.bin"));
    assert_eq!(manifest.records[0].end.block, Some(15));
    assert_eq!(manifest.records[1].start.block, Some(16));
    assert_eq!(manifest.records[1].end.block, Some(30));
    assert_eq!(manifest.records[2].start.block, Some(31));
    assert_eq!(manifest.records[2].end.block, None);
    assert_eq!(
        manifest.records[0].hashes[0].hex,
        b"578d6b80accc992770bbe59989b3178599e80d9b"
    );
    let filelist = manifest.filelist.unwrap();
    assert_eq!(filelist.len(), 13);
    assert!(filelist
        .iter()
        .any(|change| change.path == path(b"raw-\xff.bin")));
    assert!(filelist
        .iter()
        .any(|change| change.path == path(b"line\nbreak\ttab")));
    assert!(filelist
        .iter()
        .any(|change| change.path == path(b"space quote'\" backslash\\.txt")));
}
