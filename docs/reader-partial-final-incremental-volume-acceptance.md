# Official Plain Reader Partial Final-Incremental Volume Acceptance

**Status:** Implementation and independent review passed. Full local
`cargo xtask ci` passed. Hosted Linux assurance and Duvet Pages remain pending.

## Goal and scope

Verify that a partial final-incremental data volume remains visible in
discovery but cannot satisfy the completed restore set. The completed history
through the preceding date MUST remain restorable, while a request for the
final date MUST report missing volume 1. This is one in-memory key mutation of
the official plain duplicity 3.0.7 fixture, scoped to partial-volume
exclusion and two public snapshot outcomes.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final-incremental
volume 1 key:

```text
backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz
    -> backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.part.gz
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. The local source identity is synthetic; this is not an S3
identity claim. No object is added or removed, the listing remains exactly
twelve objects, and the only key-set difference is this rename. The full and
first-incremental manifests, final-incremental manifest, all full and
first-incremental volumes, and signature archives MUST remain present with
original bytes, sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as an
`IncrementalVolume` for interval `(1700086400, 1700172800)`, ordinal 1, with
the partial marker.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with parsed kind `IncrementalVolume`, start/end
   `1700086400`/`1700172800`, volume `Some(1)`, `name().partial == true`, and
   `DiscoveryStatus::Partial`.
2. Assert `Catalog::manifests()` retains exactly the three official completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`. The final manifest is complete; its data volume
   is not.
3. Derive the Full descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700086400)`. Require success and
   compare the complete state with the `mode == "plain", index == 1` entry
   set in `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Using the same iterator-derived Full index, call `read_snapshot` at
   `Timestamp(1700172800)`. It MUST return exactly
   `Err(ReaderError::MissingVolume { number: 1 })`; no final snapshot may be
   returned.

## Prior-snapshot oracle

For the successful snapshot at `1700086400`, compare every retained entry
against the official `expected.json` index 1 using only the recorded fields:

- Require exact entry count and path identity as ordered raw component bytes
  from `components_hex`; compare component sequences directly, preserving
  boundaries and proper-prefix ordering. Require `SnapshotState::entries()`
  order to match the oracle's bytewise component-sequence order.
- Require exact kind and `mode & 0o777`. For regular files and directories,
  convert `mtime_ns` to checked signed `i64`, then compare seconds with
  `div_euclid(1_000_000_000)` and nanoseconds with
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `expected.json` lists `symlink_mtime_ns` in
  `official_comparison_exclusions`, and the producer skips symlink `utime`.
- For regular files, require exact `size` and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match path,
  kind, mode, and timestamp. No entry may be skipped for absent content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, stored bytes,
   sizes, and fixture source identities. The sole mutation is renaming
   final-incremental volume 1 to `.difftar.part.gz`.
2. MUST keep the renamed volume visible with `name().partial == true` and
   `DiscoveryStatus::Partial`, and exclude it from satisfying the final
   completed restore set. Link partial-volume exclusion to `PARSE-NAME-005`.
3. MUST assert the exact three completed manifest descriptors, independent
   of iterator order. The final manifest remains complete even though its
   required volume is partial.
4. MUST derive the Full index from the public iterator and require the prior
   target `1700086400` to succeed with the exact official snapshot fields.
   Link contiguous-chain restoration to `PARSE-CHAIN-001`; link the raw-path
   oracle to `PARSE-PATH-001` and official comparison to `ASSURE-TEST-001`.
5. MUST require exact `ReaderError::MissingVolume { number: 1 }` and no
   snapshot at final target `1700172800`. Link this missing-required-volume
   result to `PARSE-CHAIN-002`.
6. MUST make no S3 identity, filesystem-publication, cryptographic, or
   complete reader/controller claim.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from partial first-incremental-volume and partial final-incremental-manifest
  tests. It MUST exercise both target dates on the same mutated catalog.
- Add Duvet links for partial-volume exclusion, prior-date contiguous-chain
  success, required-volume failure, byte-path preservation, and official
  source-oracle comparison to `PARSE-NAME-005`, `PARSE-CHAIN-001`,
  `PARSE-CHAIN-002`, `PARSE-PATH-001`, and `ASSURE-TEST-001` respectively.
- Run targeted Duvet queries, full trace, focused/full core tests, formatting,
  strict Clippy, and `cargo xtask ci`. Require hosted Linux assurance and
  Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the exact inventory mutation, bytes/size/
  identity preservation, visible Partial entry, completed descriptor set,
  prior-date oracle, final-date exact error, and documented symlink-mtime
  exclusion.

## Exclusions

This acceptance covers only final-incremental volume 1 marked partial in the
official plain fixture. It does not test partial manifests, other volume
ordinals, other envelopes or crypto profiles, arbitrary missing-volume
behavior, S3 semantics, filesystem restoration, authentication, complete
reader/controller refinement, or a formal reader proof.

## Checklist

- [x] Independent specification review accepts the mutation and both target
      outcomes.
- [x] Exact twelve-object mutation and bytes/size/identity preservation pass;
      the renamed final volume remains visible as Partial and all three
      manifests remain available.
- [x] Prior target `1700086400` matches the official oracle; final target
      `1700172800` returns exact `MissingVolume { number: 1 }`.
- [x] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and diff checks pass.
- [x] Independent implementation/test review passes.
- [x] Full local `cargo xtask ci` passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
