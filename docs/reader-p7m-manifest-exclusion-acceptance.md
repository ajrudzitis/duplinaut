# Official Plain Reader PKCS#7 Full-Manifest Exclusion Acceptance

**Status:** Specification accepted after independent review. Implementation,
Duvet evidence, and CI remain pending.

## Goal and scope

Verify that a recognized full-manifest basename with the unsupported `.p7m`
envelope remains visible as an `UnsupportedEnvelope` catalog diagnostic but
does not become a completed manifest descriptor or chain anchor. Exercise one
full-manifest name from the official plain fixture. This acceptance is about
name classification and anchor exclusion only; it makes no CMS/PKCS#7 decode
or cryptographic support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the full-manifest key:

```text
backup/duplicity-full.20231114T221320Z.manifest
    -> backup/duplicity-full.20231114T221320Z.manifest.p7m
```

The renamed object MUST retain the original manifest bytes, stored size, and
content-derived ETag; the local source harness continues to use its synthetic
version ID. No object is added or dropped, the listing remains exactly twelve
objects, and the only key-set difference is this rename. Both incremental
manifests, all full/incremental data volumes, and both signature archives MUST
remain present with their original bytes, sizes, and fixture-derived source
identities.

The renamed basename is below the default 4096-byte name limit, contains
neither NUL nor `/`, and uses valid default `NameConfig`. The fixture bytes
remain the original plain-manifest bytes; they are not claimed to be CMS
content. The expected invalid-anchor result occurs before object reads, so no
envelope decoding is performed by this acceptance. The fixture-local identity
does not make an S3 version/ETag claim.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed full-manifest object
   remains in `Catalog::objects()` with a parsed name whose envelope is
   `Envelope::Pkcs7` and whose status is `DiscoveryStatus::UnsupportedEnvelope`.
2. Collect `Catalog::manifests()` and assert it contains exactly the two
   retained Incremental descriptors. There MUST be no Full descriptor; assert
   the incremental starts/ends are `(1700000000, 1700086400)` and
   `(1700086400, 1700172800)`. These descriptors establish that the catalog
   still contains chain metadata while excluding the unsupported full anchor.
3. Obtain the first iterator item's index and call public
   `Catalog::read_snapshot` with that index and `Timestamp(1700000000)`. The
   call MUST return exactly
   `Err(ReaderError::Chain(ChainError::InvalidAnchor))`; no snapshot may be
   returned because the chosen descriptor is incremental rather than full.
   Derive the index from the iterator instead of assuming a particular index.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory and bytes; the sole
   mutation is the full-manifest key rename to `.manifest.p7m`.
2. MUST keep the renamed object visible as a recognized PKCS#7-envelope
   `CatalogObject` with `DiscoveryStatus::UnsupportedEnvelope`. Annotate this
   classification/diagnostic result to `PARSE-NAME-001`.
3. MUST exclude the unsupported full manifest from `Catalog::manifests()` and
   assert the exact two Incremental descriptors and their intervals. Link the
   unsupported-name diagnostic and these public classification assertions to
   `PARSE-NAME-001`.
4. MUST derive the descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700000000)`. It MUST return exactly
   `ReaderError::Chain(ChainError::InvalidAnchor)` with no snapshot. Annotate
   this no-full-anchor result to `PARSE-CHAIN-001`.
5. MUST make no claim that `.p7m` content is decoded, authenticated, or
   cryptographically supported. The test MUST NOT invoke a crypto adapter.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from existing discovery-status checks that include a valid full manifest.
- Add Duvet links for the unsupported envelope classification and its test to
  `PARSE-NAME-001`, and for the public invalid-anchor assertion and test to
  `PARSE-CHAIN-001`. Do not link this acceptance to `PARSE-NAME-005` or
  `PARSE-CHAIN-002`.
- Run focused queries for `PARSE-NAME-001` and `PARSE-CHAIN-001`, then the full
  trace, focused/full core tests, formatting, strict Clippy, and
  `cargo xtask ci`. Require hosted Linux assurance and Duvet Pages publication
  before acceptance is complete.
- Independent review MUST verify the single-key mutation, original payload,
  visible unsupported diagnostic, exactly two incremental descriptors,
  iterator-derived index, exact target, and exact `Chain(InvalidAnchor)` result.

## Exclusions

This acceptance covers only a PKCS#7 full-manifest name in the official plain
fixture. It does not test CMS decoding, signatures, cryptographic integrity,
partial artifacts, other unsupported envelopes, arbitrary chain behavior,
S3 identity semantics, filesystem restoration, complete `Catalog` controller
refinement, or a formal reader proof. It adds no `PARSE-NAME-005` or
`PARSE-CHAIN-002` claim.

## Checklist

- [x] Independent specification review accepts scope, mutation, and oracle.
- [ ] Exact inventory, visible unsupported diagnostic, descriptor exclusions,
      and public invalid-anchor result pass.
- [ ] Duvet links, focused/full tests, formatting, strict Clippy, and local
      `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
