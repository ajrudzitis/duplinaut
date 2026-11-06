# Official Plain Reader Partial-Full-Manifest Exclusion Acceptance

**Status:** The bounded fixture test and local scoped verification pass.
Independent implementation review, full local `cargo xtask ci`, and hosted
Linux assurance/Pages remain pending.

## Goal and scope

Verify that the public catalog keeps a listed partial full manifest visible as
a diagnostic but does not admit it to the completed manifest sequence or use
it as a chain anchor. Use the official plain 3.0.7 fixture and preserve every
stored byte; change only the full manifest's basename to its partial form.
This acceptance covers one partial full manifest and the public anchor
rejection it causes. It does not claim broad chain/controller refinement.

## Fixture mutation and inventory oracle

Start with all twelve objects from `tests/fixtures/duplicity-3.0.7/plain/`.
Rename only this key:

```text
backup/duplicity-full.20231114T221320Z.manifest
    -> backup/duplicity-full.20231114T221320Z.manifest.part
```

The renamed object MUST retain the original full-manifest stored bytes, size,
and content-derived ETag; the local source harness continues to use its
synthetic version ID. All other eleven keys, payloads, sizes, and
fixture-derived source identities MUST remain unchanged. The resulting
listing MUST still contain exactly twelve objects. In particular, both
incremental manifests and all full/incremental volumes MUST remain present.

The new basename is below the default 4096-byte bound, contains neither NUL nor
`/`, and uses valid default `NameConfig`. The local source MUST return a stable
listed `StoredObject` identity for the renamed key; its version ID is
synthetic and its ETag is derived from the unchanged fixture bytes. The
expected invalid-anchor result occurs before object reads; this fixture-local
identity does not make an S3 version/ETag claim.

## Required public observations

1. Call `Catalog::discover` and assert the renamed full-manifest object remains
   in `Catalog::objects()` with `name().partial == true` and
   `DiscoveryStatus::Partial`.
2. Collect `Catalog::manifests()` and assert it contains exactly two
   descriptors. Both MUST be `SetKind::Incremental`; there MUST be no
   `SetKind::Full` descriptor. Retained incremental manifests make the
   assertion non-vacuous.
3. Take the index from the first item returned by that iterator and pass that
   index to public `Catalog::read_snapshot` for
   `Timestamp(1700000000)`. The call MUST return exactly
   `Err(ReaderError::Chain(ChainError::InvalidAnchor))` and no snapshot,
   because that selected descriptor is incremental rather than full. The test
   MUST derive the index from the iterator; it MUST NOT assume a guessed anchor
   index.

## Requirements

1. MUST preserve the twelve-object inventory and original bytes as specified;
   the full-manifest change is one key rename to `.manifest.part`, with no
   object added, dropped, or rewritten.
2. MUST retain the partial full-manifest entry as visible discovery evidence
   while excluding it from `Catalog::manifests()`. Annotate the exclusion
   behavior to `PARSE-NAME-005`.
3. MUST establish that every descriptor exposed by `Catalog::manifests()` is
   incremental and that there is no full anchor. Annotate the public read's
   `Chain(InvalidAnchor)` result to `PARSE-CHAIN-001`, which requires a
   restorable snapshot to start with a full set.
4. MUST invoke public `Catalog::read_snapshot` using the first iterator item’s
   own index and exact target timestamp `Timestamp(1700000000)`. Any snapshot,
   successful result, or error other than `Chain(InvalidAnchor)` fails this
   acceptance.
5. MUST keep this acceptance fixture-local, with no live S3, external
   credentials, or filesystem publication.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs` distinct
  from existing partial-discovery and descriptor-construction tests.
- Add Duvet links for the `Catalog` partial-manifest exclusion and its test to
  `PARSE-NAME-005`; link the `read_snapshot` invalid-anchor assertion and test
  to `PARSE-CHAIN-001`. Do not link this acceptance to `PARSE-CHAIN-002`.
- Run focused queries for `PARSE-NAME-005` and `PARSE-CHAIN-001`, then the full
  trace, focused/full core tests, formatting, strict Clippy, and
  `cargo xtask ci`. Require hosted Linux assurance and Duvet Pages publication
  before acceptance is complete.
- Independent review MUST check the exact inventory/byte preservation, visible
  partial status, precisely two incremental descriptors, iterator-derived
  index, exact target, and exact public `Chain(InvalidAnchor)` result.

## Exclusions

This acceptance covers only the official plain full-manifest partial rename.
It does not test partial incremental manifests or data volumes, missing-volume
behavior, arbitrary malformed input, general chain selection, S3 identity
semantics, filesystem restoration, complete `Catalog` controller refinement,
or a formal reader proof. It claims no `PARSE-CHAIN-002` result.

## Checklist

- [x] Independent specification review accepts scope, mutation, and oracle.
- [x] Fixture inventory assertions, visible Partial status, exactly two
      incremental descriptors, iterator-derived index, and exact public
      invalid-anchor result pass.
- [x] Targeted `PARSE-NAME-005` and `PARSE-CHAIN-001` Duvet queries, full
      `cargo xtask trace`, focused test, full core suite, formatting, strict
      Clippy, and diff check pass locally.
- [x] Independent implementation/test review passes, including the zero
      object-open check before `Chain(InvalidAnchor)`.
- [x] Full local `cargo xtask ci` passes with 2,047, 3,183, and 1,329 verified
      declarations and zero errors; Duvet, strict Clippy, formatting, and
      workspace tests pass.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- `cargo test -p duplicity-core --test reader
  official_partial_full_manifest_remains_visible_but_is_not_a_chain_anchor`:
  passed.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-name-005`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-chain-001`: passed.
- `DUVET="$PWD/.tools/bin/duvet" cargo xtask trace-update` and `cargo xtask
  trace`: passed.
- `cargo fmt --all -- --check`, `cargo clippy -p duplicity-core --all-targets
  -- -D warnings`, and `git diff --check`: passed.
- Full pinned CI wrote `target/assurance/ci.json` with source digest
  `5f77df1719baa59be0a828410d602df98f42c2a93cd6a72064b6ddcb7443a63a`.
