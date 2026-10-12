use duplicity_core::signature::{
    Signature, SignatureError, SignatureLimits, StrongChecksum, WeakChecksum,
};
use std::io::Read;

fn header(magic: u32, block: u32, strong: u32) -> Vec<u8> {
    [
        magic.to_be_bytes(),
        block.to_be_bytes(),
        strong.to_be_bytes(),
    ]
    .concat()
}

//= docs/parsing-requirements.md#parse-sig-001
//= type=test
//# The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

//= docs/parsing-requirements.md#parse-sig-002
//= type=test
//# The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

#[test]
fn all_algorithms_and_strong_length_boundaries() {
    for (magic, weak, strong, maximum) in [
        (0x72730136, WeakChecksum::Rollsum, StrongChecksum::Md4, 16),
        (
            0x72730137,
            WeakChecksum::Rollsum,
            StrongChecksum::Blake2,
            32,
        ),
        (0x72730146, WeakChecksum::RabinKarp, StrongChecksum::Md4, 16),
        (
            0x72730147,
            WeakChecksum::RabinKarp,
            StrongChecksum::Blake2,
            32,
        ),
    ] {
        for length in 1..=maximum {
            let mut bytes = header(magic, u32::MAX, length);
            for checksum in [0x12345678u32, u32::MAX] {
                bytes.extend_from_slice(&checksum.to_be_bytes());
                bytes.extend((0..length).map(|n| n as u8));
            }
            let parsed = Signature::parse(&bytes, SignatureLimits::default()).unwrap();
            assert_eq!(
                (parsed.header().weak, parsed.header().strong),
                (weak, strong)
            );
            assert_eq!(parsed.header().block_length, u32::MAX);
            assert_eq!(parsed.record_count(), 2);
            let mut records = parsed.records();
            assert_eq!(records.len(), 2);
            assert_eq!(records.next().unwrap().weak, 0x12345678);
            let last = records.next().unwrap();
            assert_eq!(last.weak, u32::MAX);
            assert_eq!(
                last.strong,
                &(0..length).map(|n| n as u8).collect::<Vec<_>>()
            );
            assert!(records.next().is_none());
            assert!(records.next().is_none());
            assert_eq!(parsed.original_size(), None);
        }
        for length in [0, maximum + 1, u32::MAX] {
            assert_eq!(
                Signature::parse(&header(magic, 1, length), SignatureLimits::default())
                    .unwrap_err(),
                SignatureError::InvalidStrongLength
            );
        }
    }
}

//= docs/parsing-requirements.md#parse-sig-002
//= type=test
//# The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

#[test]
fn malformed_records_and_resource_limits() {
    let bytes = header(0x72730136, 512, 8);
    for prefix in 0..12 {
        assert_eq!(
            Signature::parse(&bytes[..prefix], SignatureLimits::default()).unwrap_err(),
            SignatureError::TruncatedHeader
        );
    }
    for magic in [0, 0x72730236, u32::MAX] {
        assert_eq!(
            Signature::parse(&header(magic, 512, 8), SignatureLimits::default()).unwrap_err(),
            SignatureError::UnsupportedMagic
        );
    }
    assert_eq!(
        Signature::parse(&header(0x72730136, 0, 8), SignatureLimits::default()).unwrap_err(),
        SignatureError::InvalidBlockLength
    );
    let mut bytes = bytes;
    for added in 1..12 {
        bytes.push(0);
        assert_eq!(bytes.len(), 12 + added);
        assert_eq!(
            Signature::parse(&bytes, SignatureLimits::default()).unwrap_err(),
            SignatureError::IncompleteRecord
        );
    }
    bytes.push(0);
    assert_eq!(
        Signature::parse(
            &bytes,
            SignatureLimits {
                max_bytes: bytes.len(),
                max_records: 1
            }
        )
        .unwrap()
        .record_count(),
        1
    );
    assert_eq!(
        Signature::parse(
            &bytes,
            SignatureLimits {
                max_bytes: bytes.len() - 1,
                max_records: 1
            }
        )
        .unwrap_err(),
        SignatureError::Limit
    );
    assert_eq!(
        Signature::parse(
            &bytes,
            SignatureLimits {
                max_bytes: bytes.len(),
                max_records: 0
            }
        )
        .unwrap_err(),
        SignatureError::Limit
    );
}

//= docs/parsing-requirements.md#parse-sig-003
//= type=test
//# The reader MUST represent original regular-file size as unknown when only a signature payload provides size information.

#[test]
fn empty_and_partial_source_blocks_never_infer_exact_size() {
    let empty = header(0x72730136, 512, 8);
    let signature = Signature::parse(
        &empty,
        SignatureLimits {
            max_bytes: 12,
            max_records: 0,
        },
    )
    .unwrap();
    assert_eq!(signature.record_count(), 0);
    assert_eq!(signature.original_size(), None);
    let one = [empty, vec![0; 12]].concat();
    assert_eq!(
        Signature::parse(&one, SignatureLimits::default())
            .unwrap()
            .original_size(),
        None
    );
}

//= docs/parsing-requirements.md#parse-sig-001
//= type=test
//# The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

//= docs/parsing-requirements.md#parse-sig-002
//= type=test
//# The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

//= docs/parsing-requirements.md#parse-sig-003
//= type=test
//# The reader MUST represent original regular-file size as unknown when only a signature payload provides size information.

#[test]
fn signatures_from_all_plain_official_producer_corpora() {
    use duplicity_core::names::{parse_name, NameConfig};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut pending = vec![root];
    let mut archives = 0;
    let mut signatures = 0;
    let mut short_sources = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
                continue;
            }
            let path = entry.path();
            let basename = entry.file_name();
            let bytes = basename.as_encoded_bytes();
            if !(bytes.ends_with(b".sigtar.gz") || bytes.ends_with(b".st.z")) {
                continue;
            }
            let name = parse_name(
                bytes,
                NameConfig {
                    time_separator: if bytes.contains(&b'_') { b'_' } else { b':' },
                    ..NameConfig::default()
                },
            )
            .unwrap()
            .unwrap();
            let oracle: serde_json::Value = serde_json::from_slice(
                &std::fs::read(
                    path.parent()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join("expected.json"),
                )
                .unwrap(),
            )
            .unwrap();
            let snapshot = oracle["snapshots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| {
                    s["time"].as_i64().map(i128::from) == Some(name.end.0) && s["mode"] == "plain"
                })
                .unwrap();
            let gzip = flate2::read::GzDecoder::new(std::fs::File::open(&path).unwrap());
            let mut tar = tar::Archive::new(gzip);
            for item in tar.entries().unwrap() {
                let mut item = item.unwrap();
                let archive_path = item.path_bytes();
                let Some(source_path) = archive_path.strip_prefix(b"signature/") else {
                    continue;
                };
                let hex: String = source_path
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                let source = snapshot["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["path_hex"] == hex)
                    .unwrap();
                let source_size = source["size"].as_u64().unwrap();
                let mut payload = Vec::new();
                item.read_to_end(&mut payload).unwrap();
                let signature = Signature::parse(&payload, SignatureLimits::default()).unwrap();
                let block = u64::from(signature.header().block_length);
                assert_eq!(signature.record_count() as u64, source_size.div_ceil(block));
                assert_eq!(signature.original_size(), None);
                assert_eq!(signature.records().count(), signature.record_count());
                if source_size % block != 0 {
                    short_sources += 1;
                }
                signatures += 1;
            }
            archives += 1;
        }
    }
    assert_eq!(archives, 21);
    assert!(signatures >= 200, "only {signatures} signatures");
    assert!(short_sources > 100);
}
