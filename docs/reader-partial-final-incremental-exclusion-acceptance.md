# Official Plain Reader Partial Final-Incremental Manifest Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a partial final-incremental manifest remains visible in catalog
diagnostics but is excluded from completed-chain selection. The completed
chain through the prior date MUST remain restorable, while a request for the
later date MUST report the missing transition. This is a single key mutation
of the official plain duplicity 3.0.7 fixture, scoped to partial-artifact
exclusion and the two public snapshot outcomes. It does not establish general
reader or chain-controller refinement.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final incremental
manifest key:

```text
backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
    -> backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.part
```

The renamed object MUST retain its original manifest bytes, stored size, and
fixture-derived source identity. The local source identity is synthetic; this
test makes no S3 identity claim. No object is added or removed, the inventory
remains exactly twelve objects, and the only key-set difference is this
rename. The full manifest, first incremental manifest, all data volumes, and
all signature archives MUST remain unchanged.

The renamed object parses as the final IncrementalManifest interval
`(1700086400, 1700172800)` with its partial marker. Its name is below the
default 4096-byte limit, contains neither NUL nor `/`, and uses default
`NameConfig`.

## Required public observations

1. Call public `Catalog::discover`. The renamed object MUST remain visible in
   `Catalog::objects()` with `name().partial == true` and
   `DiscoveryStatus::Partial`.
2. Collect `Catalog::manifests()` and assert exact membership independently
   of iterator order: Full `(1700000000, 1700000000)` and first Incremental
   `(1700000000, 1700086400)`. The partial final incremental MUST NOT appear
   as a completed descriptor.
3. Derive the Full descriptor index from the public manifests iterator and
   call `Catalog::read_snapshot` at `Timestamp(1700086400)`. Require success
   and compare the complete state to the `mode == "plain", index == 1` entry
   set in `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Using that same iterator-derived Full index, call `read_snapshot` at
   `Timestamp(1700172800)`. It MUST return exactly
   `Err(ReaderError::Chain(ChainError::Missing))`; no final snapshot may be
   returned because the partial final manifest cannot complete the chain.

## Successful-snapshot oracle

For the 1700086400 result, compare every retained snapshot entry against the
official `expected.json` index 1 using only its recorded fields:

- Require exact entry count and backup-relative path components as raw bytes
  from `components_hex`. Compare component sequences directly; do not use
  rendered or slash-joined path strings. Require `SnapshotState::entries()`
  ordering to match the oracle's bytewise component-sequence order, preserving
  boundaries and proper-prefix ordering.
- Require exact entry kind and `mode & 0o777`. For files and directories,
  convert `mtime_ns` to checked signed `i64`, then compare seconds using
  `div_euclid(1_000_000_000)` and nanoseconds using
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `expected.json` records `symlink_mtime_ns` in
  `official_comparison_exclusions`, and the producer skips symlink `utime`.
- For regular files, require exact `size` and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match path,
  kind, mode, and timestamp. No entry may be skipped for absent content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, bytes, sizes, and
   source identities. The sole mutation is the final-incremental manifest key
   rename to `.manifest.part`.
2. MUST retain the renamed manifest in discovery as partial and exclude it
   from `Catalog::manifests()`. Link this partial classification/exclusion to
   `PARSE-NAME-005`.
3. MUST assert exactly the Full and first Incremental descriptors with the
   specified intervals, independent of iterator order.
4. MUST derive the Full index from the public iterator and assert that the
   completed prior target `1700086400` succeeds with the exact official
   expected snapshot fields. Link the contiguous successful chain to
   `PARSE-CHAIN-001`, and the source-oracle comparison to `PARSE-PATH-001` and
   `ASSURE-TEST-001`.
5. MUST assert the later target `1700172800` returns exactly
   `ReaderError::Chain(ChainError::Missing)` with no snapshot. Link this
   missing-chain result to `PARSE-CHAIN-002`.
6. MUST make no S3 identity, filesystem-publication, cryptographic, or
   complete-controller claim.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the partial-first-incremental test. It MUST exercise both the
  restorable-prior-date success and later missing-chain result from the same
  mutated catalog.
- Add Duvet links for partial exclusion, valid contiguous-chain selection,
  raw path preservation, official source-oracle compatibility, and missing
  chain to `PARSE-NAME-005`, `PARSE-CHAIN-001`, `PARSE-PATH-001`,
  `ASSURE-TEST-001`, and `PARSE-CHAIN-002` respectively.
- Run targeted queries for those requirements, full Duvet trace, focused/full
  core tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted
  Linux assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the exact inventory mutation, stable
  payload/size/identity, visible Partial status, exact descriptor membership,
  both target outcomes, and every oracle field/exclusion.

## Exclusions

This acceptance covers only the partial final IncrementalManifest in the
official plain fixture. It does not test partial volumes or other manifests,
other envelopes/profiles, arbitrary chain failures, S3 identity semantics,
filesystem restoration, authentication, complete reader/chain-controller
refinement, or a formal reader proof.

## Checklist

- [x] Independent specification review accepts scope, mutation, and both
      public outcomes.
- [ ] Exact twelve-object inventory relation and byte/size/identity
      preservation pass; the renamed object remains visibly Partial.
- [ ] Completed descriptor membership is exactly Full plus first
      Incremental; prior-date snapshot matches expected.json and later target
      returns exact `Chain(Missing)`.
- [ ] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
