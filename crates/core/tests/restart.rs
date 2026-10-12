use duplicity_core::manifest::{Manifest, ManifestLimits};
use duplicity_core::multipart::{AssemblyError, AssemblyLimits, MultipartAssembler};
use duplicity_core::names::Envelope;
use duplicity_core::path::BackupPath;
use duplicity_core::selection::{SelectionError, SelectionLimits};
use sha1::Digest;
use sha2::Digest as _;

fn member(name: &str, payload: &[u8]) -> Vec<u8> {
    let mut h = [0u8; 512];
    h[..name.len()].copy_from_slice(name.as_bytes());
    h[100..108].copy_from_slice(b"0000644\0");
    h[124..136].copy_from_slice(format!("{:011o}\0", payload.len()).as_bytes());
    h[148..156].fill(b' ');
    h[156] = b'0';
    h[257..265].copy_from_slice(b"ustar\x0000");
    let sum: u64 = h.iter().map(|b| u64::from(*b)).sum();
    h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut bytes = h.to_vec();
    bytes.extend_from_slice(payload);
    bytes.resize(bytes.len().next_multiple_of(512), 0);
    bytes
}
fn record(number: u64, bytes: &[u8], start: &str, end: &str) -> String {
    let hash: String = sha1::Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("Volume {number}:\n StartingPath {start}\n EndingPath {end}\n Hash SHA1 {hash}\n")
}
fn make_manifest(bytes: &[u8], start: &str, end: &str, repeat: bool) -> Manifest {
    let mut text = if repeat {
        record(1, bytes, ".", end)
    } else {
        String::new()
    };
    text.push_str(&record(1, bytes, start, end));
    Manifest::parse(text.as_bytes(), ManifestLimits::default()).unwrap()
}
fn check(bytes: &[u8], start: &str, end: &str, repeat: bool) -> Result<usize, AssemblyError> {
    let manifest = make_manifest(bytes, start, end, repeat);
    let mut a = MultipartAssembler::new(&manifest, AssemblyLimits::default())?;
    if let Err(error) = a.push_stored_volume(1, bytes, Envelope::Plain) {
        assert_eq!(
            a.push_stored_volume(1, bytes, Envelope::Plain),
            Err(AssemblyError::Poisoned)
        );
        assert!(matches!(a.finish(), Err(AssemblyError::Poisoned)));
        return Err(error);
    }
    Ok(a.finish()?.entries().len())
}

//= docs/parsing-requirements.md#parse-man-009
//= type=test
//# A reader accepting queued-restart compatibility MUST restrict a nonempty volume's StartingPath exception to the source-derived one-entry root/None or second-entry boundary forms with retained raw restart-rewrite evidence.
#[test]
fn queued_exceptions_need_exact_shape_rewrite_and_entry_aligned_eof() {
    let sole = member("snapshot/a", b"a");
    assert_eq!(check(&sole, ".", "a", true), Ok(1));
    assert_eq!(check(&sole, ".", "a", false), Err(AssemblyError::Boundary));
    assert_eq!(check(&sole, ". 1", "a", true), Err(AssemblyError::Boundary));
    let two = [sole.clone(), member("snapshot/b", b"b")].concat();
    assert_eq!(check(&two, "b", "b", true), Ok(2));
    assert_eq!(check(&two, "b", "b", false), Err(AssemblyError::Boundary));
    assert_eq!(check(&two, ".", "b", true), Err(AssemblyError::Boundary));
    assert_eq!(check(&two, "b 1", "b", true), Err(AssemblyError::Boundary));
    assert_eq!(check(&two, "b", "z", true), Err(AssemblyError::Boundary));
    let three = [two.clone(), member("snapshot/c", b"c")].concat();
    assert_eq!(check(&three, "c", "c", true), Err(AssemblyError::Boundary));
    for bytes in [&sole, &two] {
        let footer = [bytes.clone(), vec![0; 512]].concat();
        let (start, end) = if bytes.len() == sole.len() {
            (".", "a")
        } else {
            ("b", "b")
        };
        assert_eq!(
            check(&footer, start, end, true),
            Err(AssemblyError::UnsupportedBoundaryProfile)
        );
    }
    // Rewritten normal starts still obey their normal footer profile.
    let normal_footer = [two, vec![0; 512]].concat();
    assert_eq!(check(&normal_footer, "a", "b", true), Ok(2));
}

#[test]
fn queued_compatibility_preserves_parts_interval_and_volume_specific_evidence() {
    let parts = [
        member("multivol_snapshot/a/1", &vec![7; 65536]),
        member("multivol_snapshot/a/2", b"end"),
    ]
    .concat();
    assert_eq!(check(&parts, "a", "a", true), Ok(1));
    let incomplete = member("multivol_snapshot/a/1", &vec![7; 65536]);
    assert_eq!(
        check(&incomplete, ".", "a 1", true),
        Err(AssemblyError::Incomplete)
    );
    let outside = [
        member("snapshot/a", b""),
        member("snapshot/b", b""),
        member("snapshot/aa", b""),
        member("snapshot/c", b""),
    ]
    .concat();
    assert_eq!(
        check(&outside, "b", "c", true),
        Err(AssemblyError::Boundary)
    );
    let a = member("snapshot/a", b"");
    let b = member("snapshot/b", b"");
    let text = record(1, &a, "a", "a") + &record(1, &a, "a", "a") + &record(2, &b, ".", "b");
    let manifest = Manifest::parse(text.as_bytes(), ManifestLimits::default()).unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assembler
        .push_stored_volume(1, &a, Envelope::Plain)
        .unwrap();
    assert_eq!(
        assembler.push_stored_volume(2, &b, Envelope::Plain),
        Err(AssemblyError::Boundary)
    );
    let two = [a, b].concat();
    let rewritten = make_manifest(&two, "b", "b", true);
    let mut corrupt = two.clone();
    corrupt[0] ^= 1;
    let mut assembler = MultipartAssembler::new(&rewritten, AssemblyLimits::default()).unwrap();
    assert_eq!(
        assembler.push_stored_volume(1, &corrupt, Envelope::Plain),
        Err(AssemblyError::Envelope(
            duplicity_core::envelope::EnvelopeError::DigestMismatch
        ))
    );
    assert!(matches!(assembler.finish(), Err(AssemblyError::Poisoned)));
    let truncated = &two[..two.len() - 1];
    assert!(matches!(
        check(truncated, "b", "b", true),
        Err(AssemblyError::Tar(_))
    ));
}

//= docs/parsing-requirements.md#parse-man-010
//= type=test
//# A reader MUST scan every normalized volume of a restart-rewritten set when raw manifest intervals have not been certified to cover decoded entries by an object-identity-bound content index.
#[test]
fn selection_retains_all_normalized_records_when_any_raw_number_repeats() {
    let bytes = member("snapshot/b", b"");
    let selected = BackupPath::parse(b"a", false, 1024, 32).unwrap();
    let normal = make_manifest(&bytes, "b", "b", false);
    assert!(normal
        .select_volumes(&selected, SelectionLimits::default())
        .unwrap()
        .records()
        .is_empty());
    let rewritten = make_manifest(&bytes, "b", "b", true);
    let selection = rewritten
        .select_volumes(&selected, SelectionLimits::default())
        .unwrap();
    assert!(selection.requires_full_scan());
    assert_eq!(selection.records().len(), 1);
    for limits in [
        SelectionLimits {
            max_raw_records: 1,
            ..SelectionLimits::default()
        },
        SelectionLimits {
            max_metadata_bytes: 0,
            ..SelectionLimits::default()
        },
    ] {
        assert!(matches!(
            rewritten.select_volumes(&selected, limits),
            Err(SelectionError::Limit)
        ));
    }
    // A discarded repeated suffix still conservatively disables pruning.
    let text = record(1, &bytes, "b", "b")
        + &record(2, &bytes, "b", "b")
        + &record(2, &bytes, "b", "b")
        + &record(1, &bytes, "b", "b");
    let discarded = Manifest::parse(text.as_bytes(), ManifestLimits::default()).unwrap();
    let selection = discarded
        .select_volumes(&selected, SelectionLimits::default())
        .unwrap();
    assert!(selection.requires_full_scan());
    assert_eq!(selection.records()[0].number, 1);
}

//= docs/parsing-requirements.md#parse-man-008
//= type=test
//# A supported nonempty volume MUST have an EndingPath matching its actual last classified logical-data entry path and pending multipart block state.
//= docs/parsing-requirements.md#parse-man-009
//= type=test
//# A reader accepting queued-restart compatibility MUST restrict a nonempty volume's StartingPath exception to the source-derived one-entry root/None or second-entry boundary forms with retained raw restart-rewrite evidence.
#[test]
fn original_official_restart_archives_match_independent_source_oracles() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/duplicity-3.0.7-restart");
    for profile in ["one-entry", "two-entry", "empty-tail"] {
        let directory = root.join(profile);
        let objects = directory.join("objects");
        let name = "duplicity-full.20231114T221320Z.manifest";
        let manifest = Manifest::parse(
            &std::fs::read(objects.join(name)).unwrap(),
            ManifestLimits::default(),
        )
        .unwrap();
        if profile != "empty-tail" {
            let selected = BackupPath::parse(b"f016", false, 1024, 32).unwrap();
            let selection = manifest
                .select_volumes(&selected, SelectionLimits::default())
                .unwrap();
            assert!(selection.requires_full_scan());
            assert_eq!(selection.records().len(), manifest.volumes().len());
            if profile == "two-entry" {
                assert!(!manifest.volumes()[1].may_contain(&selected));
            }
        }
        let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
        for record in manifest.volumes() {
            let bytes = std::fs::read(objects.join(format!(
                "duplicity-full.20231114T221320Z.vol{}.difftar.gz",
                record.number
            )))
            .unwrap();
            assembler
                .push_stored_volume(record.number, &bytes, Envelope::Gzip)
                .unwrap();
        }
        let archive = assembler.finish().unwrap();
        let oracle: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("expected.json")).unwrap())
                .unwrap();
        let expected = oracle["snapshots"][0]["entries"].as_array().unwrap();
        assert_eq!(archive.entries().len(), expected.len());
        for entry in archive.entries() {
            let name = entry
                .path
                .components()
                .iter()
                .map(|part| part.as_slice())
                .collect::<Vec<_>>()
                .join(&b'/');
            let hex: String = name.iter().map(|b| format!("{b:02x}")).collect();
            let expected = expected.iter().find(|e| e["path_hex"] == hex).unwrap();
            assert_eq!(
                entry.mode & 0o777,
                expected["mode"].as_u64().unwrap() as u32
            );
            assert_eq!(
                entry.mtime.seconds as i128 * 1000000000 + i128::from(entry.mtime.nanoseconds),
                expected["mtime_ns"].as_i64().unwrap() as i128
            );
            if expected["kind"] == "regular" {
                assert_eq!(entry.entry_type, duplicity_core::tar::EntryType::Regular);
                assert_eq!(
                    entry.payload.len() as u64,
                    expected["size"].as_u64().unwrap()
                );
                assert_eq!(
                    format!("{:x}", sha2::Sha256::digest(&entry.payload)),
                    expected["sha256"].as_str().unwrap()
                );
            } else {
                assert_eq!(entry.entry_type, duplicity_core::tar::EntryType::Directory);
            }
        }
    }
}
