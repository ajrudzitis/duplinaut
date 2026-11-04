# Encrypted reader official snapshot-oracle acceptance plan

Status: accepted. Focused/full core tests, targeted Duvet queries, full trace,
formatting, strict Clippy, diff checks, local `cargo xtask ci`, independent
review, hosted Linux assurance, and Duvet Pages publication passed in [run
41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167).

The existing encrypted-reader source-oracle test checks snapshot entry count
and regular-file paths/content hashes. This acceptance extends it to compare
every retained entry against the independent
[official 3.0.7 `expected.json`](../tests/fixtures/duplicity-3.0.7/expected.json)
oracle for the symmetric, public-key, and signed profiles at all three backup
dates. It is traceable to [PARSE-ENV-024](parsing-requirements.md#parse-env-024),
[PARSE-STATE-001](parsing-requirements.md#parse-state-001), and
[PARSE-STATE-003](parsing-requirements.md#parse-state-003). The check is native
reader compatibility evidence; it is not a filesystem publication test or a
complete controller proof.

## Acceptance requirements

1. For each of the three crypto profiles and each of the three official target
   dates, compare the full returned `SnapshotState` with the corresponding
   `expected.json` snapshot. Require equal total entry counts and an exact
   match of the complete path set. Compare each path as its raw ordered
   component byte sequence (`components_hex`); MUST NOT derive identity by
   comparing rendered or slash-joined path strings. Require the order yielded
   by `SnapshotState::entries()` to equal the oracle entries sorted
   lexicographically as sequences of raw component byte strings. Component
   boundaries MUST remain significant; a proper component prefix sorts before
   the longer component, and a proper path-component-sequence prefix sorts
   before the longer path.
2. For every matched entry, require the exact expected entry kind and mode.
   For regular files and directories, also require the exact modification
   timestamp: convert `mtime_ns` to a checked signed `i64`, then compute
   seconds with `div_euclid(1_000_000_000)` and nanoseconds with
   `rem_euclid(1_000_000_000)`. Compare both exact results with the snapshot
   entry's `TarTime`; do not round, use host timezone conversion, or compare
   only whole seconds. Do not assert symlink mtime: `expected.json` records
   `symlink_mtime_ns` in `official_comparison_exclusions`, and the
   [producer generator](../tests/producer/generate.py) documents that
   duplicity 3.0.7 skips symlink `utime`.
3. For every regular file, require exact expected byte length and SHA-256 of
   its content. For every symlink, require exact `symlink_target_hex` bytes.
   Directory entries MUST be present with their expected path, kind, mode,
   and timestamp. Do not silently skip entries whose content is absent.
4. Add Duvet test annotations linking the oracle comparison to PARSE-ENV-024,
   PARSE-STATE-001, and PARSE-STATE-003. The test MUST use the checked-in
   independent oracle and official encrypted fixtures without changing
   producer data or invoking duplicity, GnuPG, or other external crypto tools.
5. The comparison MUST be limited to fields recorded in `expected.json` and
   its stated comparison exclusions: path components, kind, mode, regular
   file/directory mtime, file length/content hash, and symlink target. Symlink
   mtime is explicitly excluded. It MUST NOT imply verification of uid/gid,
   owner names, atime/ctime, filesystem effects, or metadata absent from that
   oracle.

## Verification and evidence

### Current local evidence

- `original_encrypted_chains_reconstruct_each_official_date_with_bounded_evidence`
  compares all three fixture profiles and all three recorded dates. For each
  snapshot it requires equal entry counts, an exact raw component-vector path
  sequence, oracle order, kind and mode, and checked file/directory timestamps.
  It requires every file's retained content, exact byte length and SHA-256, and
  every symlink's exact target bytes. The test makes no assertion about symlink
  mtime because `expected.json` lists `symlink_mtime_ns` in
  `official_comparison_exclusions`; `tests/producer/generate.py` documents that
  Duplicity 3.0.7 skips symlink `utime`.
- The focused test passed across all nine profile/date combinations. Full
  `cargo test -p duplicity-core` passed, including 195 unit tests and all
  integration and doc tests. Targeted Duvet test queries for PARSE-ENV-024,
  PARSE-STATE-001, and PARSE-STATE-003 passed, as did `cargo xtask trace-update`
  and `cargo xtask trace`.
- `cargo fmt --all -- --check`,
  `cargo clippy -p duplicity-core --all-targets -- -D warnings`, and
  `git diff --check` passed. Independent review and local `cargo xtask ci`
  passed using checked-in fixtures only. Hosted Linux assurance and Duvet Pages
  publication passed in [run
  41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167).
- Independent review verified path-component preservation, exact timestamp
  conversion, entry-kind mapping, and participation of every expected entry for
  every profile/date pair.
- The result establishes oracle agreement for the enumerated fields in the
  public encrypted reader. It does not prove upstream crypto primitives,
  complete reader/controller refinement, or filesystem publication.

## Scope limits

This acceptance covers the official duplicity 3.0.7 symmetric, public-key, and
signed encrypted fixtures at their three recorded dates and only the fields
present in `expected.json`. It does not change fixtures, add crypto profiles,
assert filesystem restoration behavior, or close snapshot-state resource or
controller proofs.
