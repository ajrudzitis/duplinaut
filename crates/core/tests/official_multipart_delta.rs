//! Independent tar extraction is test infrastructure here. Production tar and
//! multipart parsing have their own pending implementation/proof milestones.
use duplicity_core::delta::apply_delta;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

const FULL: &[&[u8]] = &[
    include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.vol1.difftar.gz"
    ),
    include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.vol2.difftar.gz"
    ),
    include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-full.20231114T221320Z.vol3.difftar.gz"
    ),
];
const INCREMENTAL: &[&[u8]] = &[
    include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz"
    ),
    include_bytes!(
        "../../../tests/fixtures/duplicity-3.0.7/plain/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz"
    ),
];

fn extract_parts(volumes: &[&[u8]], prefix: &[u8]) -> Vec<u8> {
    let mut parts = BTreeMap::new();
    for volume in volumes {
        let mut archive = tar::Archive::new(GzDecoder::new(*volume));
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let name = entry.path_bytes().into_owned();
            if let Some(number) = name.strip_prefix(prefix) {
                let number: usize = std::str::from_utf8(number).unwrap().parse().unwrap();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                assert!(parts.insert(number, bytes).is_none());
            }
        }
    }
    assert!(parts.len() > 1, "fixture must exercise multipart data");
    let count = parts.len();
    let mut output = Vec::new();
    for (index, (number, bytes)) in parts.into_iter().enumerate() {
        assert_eq!(number, index + 1);
        if index + 1 < count {
            assert_eq!(bytes.len(), 65536);
        } else {
            assert!(bytes.len() < 65536);
        }
        output.extend(bytes);
    }
    output
}

fn source_hash(snapshot_index: u64) -> String {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/duplicity-3.0.7/expected.json"
    ))
    .unwrap();
    let snapshot = oracle["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["mode"] == "plain" && item["index"] == snapshot_index)
        .unwrap();
    snapshot["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path_hex"] == "6c617267652e62696e")
        .unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned()
}

//= docs/parsing-requirements.md#parse-delta-003
//# A literal command MUST append exactly its indicated payload bytes to the reconstructed output.
//= docs/parsing-requirements.md#parse-delta-004
//# A copy command MUST append exactly the indicated byte range from the fixed basis file.
#[test]
fn official_large_delta_matches_source_tree_oracle() {
    let basis = extract_parts(FULL, b"multivol_snapshot/large.bin/");
    assert_eq!(format!("{:x}", Sha256::digest(&basis)), source_hash(0));
    let delta = extract_parts(INCREMENTAL, b"multivol_diff/large.bin/");
    assert!(delta.len() > 1024 * 1024);
    let result = apply_delta(&delta, &basis, 2621440).unwrap();
    assert_eq!(result.len(), 2621440);
    assert_eq!(format!("{:x}", Sha256::digest(&result)), source_hash(1));
    assert!(apply_delta(&delta, &basis, 2621439).is_err());
}
