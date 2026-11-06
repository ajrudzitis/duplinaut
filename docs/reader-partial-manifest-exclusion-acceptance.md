# Official Plain Reader Partial-Full-Manifest Exclusion Acceptance

**Status:** Specification accepted after independent review. Implementation,
Duvet evidence, and CI remain pending.

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
- [ ] Fixture inventory assertions, visible Partial status, two incremental
      descriptors, and the public invalid-anchor result pass.
- [ ] Duvet links, focused/full tests, formatting, strict Clippy, and local
      `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
