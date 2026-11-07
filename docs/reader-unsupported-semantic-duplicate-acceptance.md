# Official Plain Reader Unsupported Semantic-Duplicate Acceptance

**Status:** Implementation and independent implementation review passed,
including the fixture-scoped `PARSE-STATE-003` mapping. Focused/full core
tests, Duvet queries/full trace, formatting, and strict Clippy pass. The
final-tree full pinned local CI passes with Verus partitions 2,047/3,183/1,329
and zero errors; its receipt is generated at `target/assurance/ci.json`.
Hosted Linux/Pages assurance is pending.

## Goal and scope

Verify that a recognized unsupported-envelope object with the same parsed
kind, timestamp, and volume ordinal as an official supported object remains a
diagnostic and does not create a completed-artifact collision. The public
reader MUST reconstruct the official final snapshot using the supported
`.gz` object and MUST NOT open the unsupported `.p7m` sibling. This acceptance
covers one exact supported/unsupported identity pair in the official plain
Duplicity 3.0.7 fixture; it makes no general policy claim for other names or
envelopes.

## Fixture setup

Start with the unchanged twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Add exactly one zero-byte object:

```text
backup/duplicity-full.20231114T221320Z.vol1.difftar.p7m
```

The official supported sibling remains unchanged:

```text
backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
```

The added `.p7m` key parses as `FullVolume` at timestamp `1700000000`, volume
ordinal 1, with envelope `Pkcs7` and no partial marker. The two parsed names
therefore have the same kind/date/ordinal but different envelopes. Discovery
MUST retain the new object with `DiscoveryStatus::UnsupportedEnvelope`; it
MUST NOT reject the catalog as `DuplicateArtifact`. The source identity for
the added object is synthetic and this test makes no S3 version/ETag claim.
No official fixture object is renamed or mutated; the listing contains
exactly thirteen objects.

## Required public observations

1. Call public `Catalog::discover` and assert the added `.p7m` object is
   visible with parsed kind `FullVolume`, timestamp `1700000000`, volume
   `Some(1)`, envelope `Envelope::Pkcs7`, `partial == false`, and status
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert the original `.gz` sibling remains visible with the same parsed
   kind/date/ordinal, envelope `Envelope::Gzip`, and
   `DiscoveryStatus::Completed`.
3. Assert `Catalog::manifests()` has exactly the official three completed
   descriptors, independent of iterator order: Full
   `(1700000000,1700000000)`, first Incremental
   `(1700000000,1700086400)`, and final Incremental
   `(1700086400,1700172800)`.
4. Derive the Full descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete snapshot with `mode == "plain", index == 2` in
   `tests/fixtures/duplicity-3.0.7/expected.json`.
5. Assert source opens equal exactly the three manifests and six supported
   data volumes. In particular, the original `.gz` volume 1 MUST be opened
   and the added `.p7m` object MUST NOT be opened:

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

- Require exact entry count, raw component-byte paths from `components_hex`,
  and iteration order equal to the oracle's sorted raw component sequences.
  Preserve component boundaries; do not compare rendered or slash-joined
  paths.
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

1. MUST retain the added key as a visible `FullVolume` diagnostic with
   envelope `Pkcs7` and status `UnsupportedEnvelope`, while the same-semantic
   `.gz` sibling remains completed. Link parsed classification and diagnostic
   behavior to `PARSE-NAME-001`.
2. MUST retain exactly the three official descriptors and successfully read
   the final target through the uniquely contiguous Full-plus-Incremental
   chain. The unsupported sibling MUST NOT cause `DuplicateArtifact` or
   participate as a completed required volume. Link this restored-chain
   behavior to `PARSE-CHAIN-001`.
3. MUST compare the complete snapshot with the documented `expected.json`
   fields, including raw component paths, kind/mode, supported timestamps,
   file size/hash, and symlink target; omit symlink mtime as documented. Link
   these checks to `PARSE-PATH-001` and `ASSURE-TEST-001`.
4. MUST require `SnapshotState::entries()` iteration order to equal the
   oracle's paths sorted lexicographically by raw component-byte sequences.
   Compare each component as unsigned bytes, preserve component boundaries,
   and order a proper component-sequence prefix before its extension. This
   assertion covers the observed fixture snapshot and links to
   `PARSE-STATE-003`; it does not claim proof of standard-library map internals.
5. MUST assert the exact nine-key open set above; the unsupported `.p7m`
   sibling MUST NOT be opened. This result is limited to the named fixture
   pair and plain chain.
6. MUST NOT make a universal claim about collision handling for other
   unsupported variants, CMS decoding, cryptography, S3 identity, or
   filesystem restoration.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`. Preserve
  all twelve official objects and add only the zero-byte `.p7m` sibling.
- Add Duvet links for unsupported/supported same-semantic name diagnostics to
  `PARSE-NAME-001`, restored-chain success to `PARSE-CHAIN-001`, raw byte paths
  to `PARSE-PATH-001`, the raw-component iteration-order assertion to
  `PARSE-STATE-003`, and the official snapshot oracle to `ASSURE-TEST-001`.
  Run targeted Duvet queries for all five linked requirements and the full
  trace. Do not add `PARSE-NAME-005` or crypto-authentication links.
- Run focused/full core tests, targeted Duvet queries and full trace,
  formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the exact pair of keys, parsed identity
  equality except envelope, unsupported diagnostic visibility, absence of a
  duplicate-artifact error, oracle match, and exact source-open set.

## Exclusions

This acceptance covers only the FullVolume volume-1 `.gz`/`.p7m` pair in the
official plain fixture. It does not test duplicate supported artifacts,
other volume ordinals or object kinds, partial variants, other unsupported
envelopes, signed/encrypted profiles, S3 identity semantics, filesystem
restoration, complete reader/controller refinement, or formal reader proof.

## Checklist

- [x] Independent specification review accepts the same-semantic pair,
      diagnostic treatment, snapshot oracle, and open set.
- [x] Independent review accepts the added `PARSE-STATE-003` ordering scope.
- [x] Discovery preserves both objects with distinct envelope/status values
      and does not reject the catalog as a duplicate artifact.
- [x] The final snapshot matches all documented oracle fields and opens
      exactly the three manifests plus six supported data volumes.
- [x] Focused/full core tests, formatting, strict Clippy, and diff checks pass
      locally.
- [x] Targeted Duvet queries for `PARSE-NAME-001`, `PARSE-CHAIN-001`,
      `PARSE-PATH-001`, `PARSE-STATE-003`, and `ASSURE-TEST-001`, plus the full
      trace, pass after adding the `PARSE-STATE-003` test link.
- [x] Independent implementation/test review passes.
- [x] Final-tree full pinned local `cargo xtask ci` passes after the added
      `PARSE-STATE-003` mapping and documentation update.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- Focused reader acceptance test passed (1/1).
- `cargo test -p duplicity-core` passed.
- Targeted Duvet queries for `PARSE-NAME-001`, `PARSE-CHAIN-001`,
  `PARSE-PATH-001`, `PARSE-STATE-003`, and `ASSURE-TEST-001`, followed by
  `cargo xtask trace-update` and `cargo xtask trace`, passed after the
  `PARSE-STATE-003` test annotation and assurance registration were added.
- `cargo fmt --all -- --check`, strict core Clippy, and `git diff --check`
  passed.
- Independent implementation review accepts the added STATE-003 scope.
- Final-tree full pinned local `cargo xtask ci` passed with Verus partitions
  2,047/3,183/1,329 and zero errors. The generated receipt is
  `target/assurance/ci.json`. Hosted Linux assurance/Pages remain pending.
