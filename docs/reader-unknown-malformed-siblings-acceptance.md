# Official Plain Reader Unknown and Malformed Sibling Objects Acceptance

**Status:** The implementation and independent review pass. The raw-component
order assertion is linked to `PARSE-STATE-003` and registered in the
snapshot-entry-order assurance milestone; the targeted query and full Duvet
trace pass. Focused/full core tests, formatting, strict Clippy, and full local
`cargo xtask ci` pass (Verus 2,047/3,183/1,329, zero errors; the generated
receipt is `target/assurance/ci.json`). Hosted Linux assurance and Duvet Pages
pass in [run 42362198467](https://github.com/ajrudzitis/duplinaut/actions/runs/42362198467).

## Goal and scope

Verify that an unknown sibling object and a malformed recognized-name sibling
remain visible as catalog diagnostics without entering the completed restore
chain or preventing a valid official snapshot read. The public reader MUST
restore the final plain snapshot from the official Duplicity 3.0.7 fixture
while opening only the required manifests and data volumes. This acceptance
covers these two exact sibling names; it does not make a claim about corrupt
content or every kind of unrelated object.

## Fixture setup

Begin with the unchanged twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Add exactly these two zero-byte
objects, without modifying any official key, byte, size, or identity:

```text
backup/notes.txt
backup/duplicity-full.invalid.manifest
```

The first key is unrelated to the duplicity object-name grammar. The second
uses a recognized Full manifest stem with an invalid date and MUST be
classified as malformed. The local source identities are synthetic; this
test makes no S3 version/ETag claim. The source listing therefore contains
exactly fourteen objects: the twelve unchanged fixture objects and these two
diagnostic-only additions.

## Required public observations

1. Call public `Catalog::discover` and assert `backup/notes.txt` remains in
   `Catalog::objects()` with status `DiscoveryStatus::Unknown`, while
   `backup/duplicity-full.invalid.manifest` remains visible with a
   `DiscoveryStatus::Malformed(_)` diagnostic.
2. Assert `Catalog::manifests()` still contains exactly the three official
   completed descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`. Neither diagnostic-only object may appear as a
   descriptor or alter descriptor membership.
3. Derive the Full index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete state with `mode == "plain", index == 2` in
   `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Assert source opens equal exactly the three required manifests and six data
   volumes listed below. The unknown key, malformed key, and all three
   signatures archives MUST NOT be opened:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz
   ```

## Snapshot oracle

Compare every final entry with the official `expected.json` final plain
snapshot:

- Require exact entry count, raw component-byte path identities from
  `components_hex`, and `SnapshotState::entries()` order equal to the
  oracle's lexicographically sorted component sequences. Preserve component
  boundaries; do not compare rendered or slash-joined paths.
- Require exact entry kind and `mode & 0o777`. For regular files and
  directories, convert `mtime_ns` to checked signed `i64`; compare seconds via
  `div_euclid(1_000_000_000)` and nanoseconds via
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `official_comparison_exclusions` lists `symlink_mtime_ns`, and the producer
  skips symlink `utime`.
- For regular files, require exact size and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match their
  recorded paths, kinds, modes, and timestamps. Do not skip entries without
  content.

## Requirements

1. MUST retain both added objects as visible diagnostics: unrelated
   `backup/notes.txt` is `Unknown`, and the recognized Full-manifest stem with
   an invalid date is `Malformed`. MUST keep them out of parsed manifest
   descriptors. Link complete-name classification and unrelated-object
   handling to `PARSE-NAME-001`.
2. MUST retain exactly the three official descriptors and successfully read
   the final target through the uniquely contiguous Full-plus-Incremental
   chain. Link the chain behavior to `PARSE-CHAIN-001`.
3. MUST compare the complete final snapshot with the recorded `expected.json`
   fields, including raw component paths, kind/mode, supported
   timestamps, regular-file size/hash, and symlink target; omit symlink mtime
   as documented. Link these checks to `PARSE-PATH-001` and `ASSURE-TEST-001`.
4. MUST enumerate `SnapshotState::entries()` in bytewise lexicographic order
   of raw backup-relative path components. Compare components as unsigned
   byte sequences, keep component boundaries significant, and order a proper
   component-sequence prefix before its extension. The test MUST compare
   actual iteration order with the oracle's sorted component sequences and
   link this assertion to `PARSE-STATE-003`.
5. MUST assert the exact nine-key open set above. Neither diagnostic sibling
   may be opened during this read. This is a bounded fixture result, not a
   universal claim for arbitrary extra objects.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`. Preserve
  all official fixture objects exactly and add only the two specified keys.
- Add Duvet test links for unknown/malformed classification and ignored
  siblings to `PARSE-NAME-001`, successful chain restoration to
  `PARSE-CHAIN-001`, byte paths to `PARSE-PATH-001`, and the official fixture
  oracle to `ASSURE-TEST-001`. The existing iteration-order assertion MUST
  also link to `PARSE-STATE-003`.
- Run the focused test, targeted Duvet queries/full trace, focused/full core
  tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the two exact added keys, their public
  diagnostics, unchanged official inventory, descriptor membership, snapshot
  oracle, and exact open set.

## Exclusions

This acceptance covers only `backup/notes.txt` and
`backup/duplicity-full.invalid.manifest` alongside the official plain fixture.
It does not test corrupt recognized content, unsupported envelopes, partial
objects, arbitrary ignored-key classes, alternate profiles, S3 identity,
filesystem restoration, complete reader/controller refinement, or a formal
reader proof.

## Checklist

- [x] Independent specification review accepts the exact added keys,
      diagnostics, descriptor set, and open set.
- [x] Independent specification review accepts the added `PARSE-STATE-003`
      iteration-order requirement and traceability scope.
- [x] Both sibling objects remain visible as `Unknown` and `Malformed`,
      without changing the official descriptor membership.
- [x] The final snapshot matches all documented oracle fields and source opens
      equal exactly the three manifests plus six data volumes.
- [x] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      strict Clippy, and diff checks pass locally.
- [x] Independent implementation/test review passes.
- [x] Full pinned local `cargo xtask ci` passes with fresh verification,
      formatting, strict workspace Clippy, workspace tests, and named-gate
      validation.
- [x] Hosted Linux assurance and Duvet Pages publication pass in [run
      42362198467](https://github.com/ajrudzitis/duplinaut/actions/runs/42362198467).

## Current local evidence

- Focused reader acceptance test passed (1/1).
- `cargo test -p duplicity-core` passed.
- Targeted Duvet queries for `PARSE-NAME-001`, `PARSE-CHAIN-001`,
  `PARSE-PATH-001`, and `ASSURE-TEST-001`, `cargo xtask trace-update`, and
  `cargo xtask trace` passed.
- `cargo fmt --all -- --check`, strict core Clippy, and `git diff --check`
  passed.
- Independent implementation review and full pinned local CI passed. Hosted
  Linux assurance and Duvet Pages passed in [run
  42362198467](https://github.com/ajrudzitis/duplinaut/actions/runs/42362198467).
- `PARSE-STATE-003` is linked to the raw-component order assertion in this
  test and registered under `snapshot-entry-order`; its evidence is limited to
  the observed order of this fixture snapshot and does not prove
  standard-library map internals.
