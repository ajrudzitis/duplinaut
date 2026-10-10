//! Payloads extracted from official duplicity archives, with independent source
//! contents and original object/member hashes recorded beside the fixtures.
use duplicity_core::delta::apply_delta;

const FIRST: &[u8] =
    include_bytes!("../../../tests/fixtures/duplicity-3.0.7/deltas/plain-inc1.rdiff");
const SECOND: &[u8] =
    include_bytes!("../../../tests/fixtures/duplicity-3.0.7/deltas/plain-inc2.rdiff");
const META_FIRST: &[u8] =
    include_bytes!("../../../tests/fixtures/duplicity-3.0.7/deltas/metadata-inc1.rdiff");
const META_SECOND: &[u8] =
    include_bytes!("../../../tests/fixtures/duplicity-3.0.7/deltas/metadata-inc2.rdiff");

//= docs/parsing-requirements.md#parse-delta-001
//# The delta parser MUST require the reviewed delta magic before decoding commands.
//= docs/parsing-requirements.md#parse-delta-004
//# A copy command MUST append exactly the indicated byte range from the fixed basis file.
//= docs/parsing-requirements.md#parse-delta-008
//# A complete delta stream MUST contain END immediately after its last complete command and contain no bytes after END.
#[test]
fn official_incremental_payloads_match_independent_source_contents() {
    let second = apply_delta(FIRST, b"original\n", 1024).unwrap();
    assert_eq!(second, b"second\n");
    let third = apply_delta(SECOND, &second, 1024).unwrap();
    assert_eq!(third, b"third\n");
}

#[test]
fn official_metadata_only_deltas_preserve_contents() {
    let basis = b"unchanged-content\n";
    let first = apply_delta(META_FIRST, basis, basis.len()).unwrap();
    assert_eq!(first, basis);
    let second = apply_delta(META_SECOND, &first, basis.len()).unwrap();
    assert_eq!(second, basis);
    assert!(apply_delta(META_FIRST, basis, basis.len() - 1).is_err());
}
