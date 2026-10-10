use duplicity_core::delta::apply_delta;
use duplicity_core::envelope::EnvelopeError;
use duplicity_core::manifest::{Manifest, ManifestLimits};
use duplicity_core::multipart::*;
use duplicity_core::names::Envelope;
use sha1::Digest;
use sha2::Digest as _;
use std::path::Path;

fn member(name: &str, payload: &[u8]) -> Vec<u8> {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{:011o}\0", payload.len()).as_bytes());
    header[136..148].copy_from_slice(b"00000000001\0");
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..265].copy_from_slice(b"ustar\x0000");
    let sum: u64 = header.iter().map(|b| u64::from(*b)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut output = header.to_vec();
    output.extend_from_slice(payload);
    output.resize(output.len().next_multiple_of(512), 0);
    output
}
fn make_manifest(volumes: &[Vec<u8>], boundaries: &[(&str, &str)]) -> Manifest {
    let mut text = String::new();
    for (index, (volume, (start, end))) in volumes.iter().zip(boundaries).enumerate() {
        text.push_str(&format!(
            "Volume {}:\n StartingPath {start}\n EndingPath {end}\n Hash SHA1 {}\n",
            index + 1,
            sha1::Sha1::digest(volume)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
    }
    Manifest::parse(text.as_bytes(), ManifestLimits::default()).unwrap()
}
fn assemble(
    volumes: &[Vec<u8>],
    boundaries: &[(&str, &str)],
) -> Result<AssembledArchive, AssemblyError> {
    let manifest = make_manifest(volumes, boundaries);
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default())?;
    for (index, volume) in volumes.iter().enumerate() {
        assembler.push_stored_volume(index as u64 + 1, volume, Envelope::Plain)?;
    }
    assembler.finish()
}

//= docs/parsing-requirements.md#parse-multi-001
//= type=test
//# Multipart assembly MUST concatenate payloads for one operation and path in consecutive part-number order starting at 1.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn continuation_markers_zero_final_and_empty_volumes() {
    let full = member("multivol_snapshot/file/1", &vec![b'a'; 65536]);
    let final_part = member("multivol_snapshot/file/2", b"");
    let volumes = [full.clone(), final_part.clone()];
    let archive = assemble(&volumes, &[("file 1", "file 1"), ("file", "file")]).unwrap();
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.entries()[0].payload, vec![b'a'; 65536]);
    assert_eq!(archive.entries()[0].operation, LogicalOperation::Snapshot);
    assert_eq!(archive.entries()[0].mode, 0o644);
    assert_eq!(archive.entries()[0].mtime.seconds, 1);
    // Source-derived empty volumes preserve continuation, and their mandatory
    // hash/container checks do not get skipped merely because no entries exist.
    let volumes = [Vec::new(), full.clone(), Vec::new(), final_part, Vec::new()];
    let archive = assemble(
        &volumes,
        &[
            (".", "."),
            ("file 1", "file 1"),
            (".", "file 1"),
            ("file", "file"),
            (".", "file"),
        ],
    )
    .unwrap();
    assert_eq!(archive.entries()[0].payload.len(), 65536);
    assert!(assemble(&[Vec::new()], &[("file", "file")]).is_err());
    assert!(assemble(&[full], &[("file 1", "file 1")]).is_err());
}

//= docs/parsing-requirements.md#parse-multi-002
//= type=test
//# Multipart assembly MUST reject missing, duplicate, reversed, mixed-operation, or interleaved parts of the same logical entry.
#[test]
fn malformed_parts_lengths_markers_and_interleaving_rejected() {
    let full = member("multivol_snapshot/file/1", &vec![1; 65536]);
    for next in [
        member("multivol_snapshot/file/1", b"x"),
        member("multivol_snapshot/file/3", b"x"),
        member("multivol_diff/file/2", b"x"),
        member("multivol_snapshot/other/2", b"x"),
        member("snapshot/other", b"x"),
        member("snapshot/file", b"x"),
    ] {
        assert!(assemble(&[[full.clone(), next].concat()], &[("file 1", "file")]).is_err());
    }
    assert!(assemble(
        &[member("multivol_snapshot/file/2", b"x")],
        &[("file", "file")]
    )
    .is_err());
    assert!(assemble(
        &[member("multivol_snapshot/file/1", &vec![0; 65537])],
        &[("file", "file")]
    )
    .is_err());
    let final_part = member("multivol_snapshot/file/2", b"x");
    for boundaries in [
        [("file", "file 1"), ("file", "file")],
        [("file 1", "file"), ("file", "file")],
        [("file 1", "file 1"), ("file 2", "file")],
        [("file 1", "file 1"), ("file", "file 2")],
        [("aaa 1", "file 1"), ("file", "file")],
    ] {
        assert!(assemble(&[full.clone(), final_part.clone()], &boundaries).is_err());
    }
    let repeated = [
        member("multivol_snapshot/file/1", b"done"),
        member("multivol_snapshot/file/1", b"duplicate"),
    ]
    .concat();
    assert!(matches!(
        assemble(&[repeated], &[("file", "file")]),
        Err(AssemblyError::DuplicateEntry)
    ));
    for entries in [
        [
            member("snapshot/file", b"ordinary"),
            member("multivol_snapshot/file/1", b"multipart"),
        ],
        [
            member("multivol_snapshot/file/1", b"multipart"),
            member("snapshot/file", b"ordinary"),
        ],
    ] {
        assert!(matches!(
            assemble(&[entries.concat()], &[("file", "file")]),
            Err(AssemblyError::DuplicateEntry)
        ));
    }
    let ordinary = [
        member("snapshot/file", b"first"),
        member("snapshot/file", b"second"),
    ]
    .concat();
    assert_eq!(
        assemble(&[ordinary], &[("file", "file")])
            .unwrap()
            .entries()
            .len(),
        2
    );
    let outside = [
        member("snapshot/a", b""),
        member("snapshot/zz", b""),
        member("snapshot/z", b""),
    ]
    .concat();
    assert!(matches!(
        assemble(&[outside], &[("a", "z")]),
        Err(AssemblyError::Boundary)
    ));
}

#[test]
fn component_metadata_growth_is_bounded_before_classification() {
    let nested = format!("{}a", "a/".repeat(39));
    let flat = "a".repeat(nested.len());
    for (path, succeeds) in [(flat, true), (nested, false)] {
        let volume = member(&format!("snapshot/{path}"), b"");
        let manifest = make_manifest(std::slice::from_ref(&volume), &[(&path, &path)]);
        let mut assembler = MultipartAssembler::new(
            &manifest,
            AssemblyLimits {
                max_metadata_bytes: 8000,
                ..AssemblyLimits::default()
            },
        )
        .unwrap();
        let result = assembler.push_stored_volume(1, &volume, Envelope::Plain);
        if succeeds {
            result.unwrap();
            assembler.finish().unwrap();
        } else {
            assert_eq!(result, Err(AssemblyError::Limit));
        }
    }
}

//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn all_volumes_integrity_container_limits_and_poisoning() {
    let empty = Manifest {
        hostname: None,
        localdir: None,
        records: Vec::new(),
        filelist: None,
    };
    assert!(matches!(
        MultipartAssembler::new(&empty, AssemblyLimits::default()),
        Err(AssemblyError::Incomplete)
    ));
    let volumes = [member("snapshot/file", b"abc"), Vec::new()];
    let manifest = make_manifest(&volumes, &[("file", "file"), (".", "file")]);
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assembler
        .push_stored_volume(1, &volumes[0], Envelope::Plain)
        .unwrap();
    assert!(matches!(assembler.finish(), Err(AssemblyError::Incomplete)));
    for limits in [
        AssemblyLimits {
            max_entries: 0,
            ..AssemblyLimits::default()
        },
        AssemblyLimits {
            max_payload_bytes: 2,
            ..AssemblyLimits::default()
        },
    ] {
        let mut assembler = MultipartAssembler::new(&manifest, limits).unwrap();
        assert!(matches!(
            assembler.push_stored_volume(1, &volumes[0], Envelope::Plain),
            Err(AssemblyError::Limit)
        ));
        assert_eq!(
            assembler.push_stored_volume(1, &volumes[0], Envelope::Plain),
            Err(AssemblyError::Poisoned)
        );
        assert!(matches!(assembler.finish(), Err(AssemblyError::Poisoned)));
    }
    assert!(matches!(
        MultipartAssembler::new(
            &manifest,
            AssemblyLimits {
                max_metadata_bytes: 0,
                ..AssemblyLimits::default()
            }
        ),
        Err(AssemblyError::Limit)
    ));
    let mut assembler = MultipartAssembler::new(
        &manifest,
        AssemblyLimits {
            max_metadata_bytes: 1024,
            ..AssemblyLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        assembler.push_stored_volume(1, &volumes[0], Envelope::Plain),
        Err(AssemblyError::Limit)
    );
    assert!(matches!(
        MultipartAssembler::new(
            &manifest,
            AssemblyLimits {
                max_volumes: 1,
                ..AssemblyLimits::default()
            }
        ),
        Err(AssemblyError::Limit)
    ));
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    assert_eq!(
        assembler.push_stored_volume(2, &volumes[1], Envelope::Plain),
        Err(AssemblyError::VolumeOrder)
    );
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let mut corrupt = volumes[0].clone();
    corrupt[0] ^= 1;
    assert_eq!(
        assembler.push_stored_volume(1, &corrupt, Envelope::Plain),
        Err(AssemblyError::Envelope(EnvelopeError::DigestMismatch))
    );
    let mut truncated = volumes[0].clone();
    truncated.pop();
    let malformed = make_manifest(&[truncated.clone()], &[("file", "file")]);
    let mut assembler = MultipartAssembler::new(&malformed, AssemblyLimits::default()).unwrap();
    assert!(matches!(
        assembler.push_stored_volume(1, &truncated, Envelope::Plain),
        Err(AssemblyError::Tar(_))
    ));
}
fn official_archive(directory: &Path, manifest_name: &str) -> AssembledArchive {
    let manifest = Manifest::parse(
        &std::fs::read(directory.join(manifest_name)).unwrap(),
        ManifestLimits::default(),
    )
    .unwrap();
    let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
    let prefix = manifest_name.strip_suffix(".manifest").unwrap();
    for record in manifest.volumes() {
        let stored =
            std::fs::read(directory.join(format!("{prefix}.vol{}.difftar.gz", record.number)))
                .unwrap();
        assembler
            .push_stored_volume(record.number, &stored, Envelope::Gzip)
            .unwrap();
    }
    assembler.finish().unwrap()
}
fn oracle_hash(index: u64) -> String {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["mode"] == "plain" && s["index"] == index)
        .unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["path_hex"] == "6c617267652e62696e")
        .unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned()
}

//= docs/parsing-requirements.md#parse-multi-001
//= type=test
//# Multipart assembly MUST concatenate payloads for one operation and path in consecutive part-number order starting at 1.
//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn official_multipart_snapshot_and_delta_match_source_tree_oracles() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicity-3.0.7/plain");
    let full = official_archive(&root, "duplicity-full.20231114T221320Z.manifest");
    let basis = &full
        .entries()
        .iter()
        .find(|entry| entry.path.components().as_slice() == [b"large.bin".to_vec()])
        .unwrap()
        .payload;
    assert_eq!(format!("{:x}", sha2::Sha256::digest(basis)), oracle_hash(0));
    let incremental = official_archive(
        &root,
        "duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest",
    );
    let delta = &incremental
        .entries()
        .iter()
        .find(|entry| entry.path.components().as_slice() == [b"large.bin".to_vec()])
        .unwrap()
        .payload;
    assert!(delta.len() > 1024 * 1024);
    let restored = apply_delta(delta, basis, 2621440).unwrap();
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&restored)),
        oracle_hash(1)
    );
    assert_eq!(restored.len(), 2621440);
}

//= docs/parsing-requirements.md#parse-multi-003
//= type=test
//# The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.
#[test]
fn every_failure_class_latches_poison_before_publication() {
    let valid = member("snapshot/file", b"abc");
    let malformed = valid[..511].to_vec();
    for case in 0..4 {
        let stored = if case == 2 {
            malformed.as_slice()
        } else {
            valid.as_slice()
        };
        let boundaries = if case == 1 {
            [(".", "file")]
        } else {
            [("file", "file")]
        };
        let manifest = make_manifest(&[stored.to_vec()], &boundaries);
        let mut assembler = MultipartAssembler::new(&manifest, AssemblyLimits::default()).unwrap();
        let mut bad_hash = stored.to_vec();
        bad_hash[0] ^= 1;
        let (number, input, expected) = match case {
            0 => (
                1,
                bad_hash.as_slice(),
                AssemblyError::Envelope(EnvelopeError::DigestMismatch),
            ),
            // This error happens after one complete logical entry was staged.
            1 => (1, stored, AssemblyError::Boundary),
            2 => (
                1,
                stored,
                AssemblyError::Tar(duplicity_core::tar::TarError::Truncated),
            ),
            _ => (2, stored, AssemblyError::VolumeOrder),
        };
        assert_eq!(
            assembler.push_stored_volume(number, input, Envelope::Plain),
            Err(expected)
        );
        assert_eq!(
            assembler.push_stored_volume(1, &valid, Envelope::Plain),
            Err(AssemblyError::Poisoned)
        );
        assert!(matches!(assembler.finish(), Err(AssemblyError::Poisoned)));
    }
}
