# Official Plain Reader PKCS#7 Final-Incremental Manifest Exclusion Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a recognized final-incremental manifest with the unsupported
`.p7m` envelope remains visible as an `UnsupportedEnvelope` catalog diagnostic
but is excluded from the completed chain. The catalog must retain the full
anchor and first incremental descriptor, while a request for the final
snapshot reports the missing transition. This acceptance covers one mutation
of the official plain 3.0.7 fixture. It makes no CMS/PKCS#7 decoding or
cryptographic support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final incremental
manifest key:

```text
backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
    -> backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.p7m
```

The renamed object MUST retain its original manifest bytes, stored size, and
fixture-derived source identity. The local source harness uses its existing
synthetic version ID; this makes no S3 version/ETag claim. No object is added
or removed, the listing remains exactly twelve objects, and the only key-set
difference is this rename. The full manifest, first incremental manifest,
all data volumes, and both signature archives MUST remain present with their
original bytes, sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. Its parsed incremental interval is
`(1700086400, 1700172800)`. The stored bytes remain the original plain
manifest payload; the `.p7m` suffix does not establish that they are CMS
content. This test covers classification and completed-chain selection only.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()`. Its parsed name MUST have kind
   `IncrementalManifest`, interval start/end `1700086400`/`1700172800`, and
   envelope `Envelope::Pkcs7`; its status MUST be
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Collect `Catalog::manifests()` and assert exact membership, independent of
   iteration order: a Full descriptor with start/end
   `1700000000`/`1700000000` and the first Incremental descriptor with start/end
   `1700000000`/`1700086400`. The unsupported final Incremental MUST not appear
   as a completed descriptor.
3. Derive the Full descriptor index from the public manifests iterator and
   call `Catalog::read_snapshot` at `Timestamp(1700172800)`. It MUST return
   exactly `Err(ReaderError::Chain(ChainError::Missing))`; no snapshot may be
   returned. The retained first incremental is a non-vacuity control: a valid
   prior transition exists, but the requested final transition is missing.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, original bytes,
   sizes, and fixture identities. The only mutation is the final-incremental
   manifest key rename to `.manifest.p7m`.
2. MUST keep the renamed object visible as a parsed
   `IncrementalManifest(1700086400, 1700172800)` with `Envelope::Pkcs7` and
   `DiscoveryStatus::UnsupportedEnvelope`. Link its public classification and
   test to `PARSE-NAME-001`.
3. MUST assert exact completed descriptor membership: Full
   `(1700000000, 1700000000)` and first Incremental
   `(1700000000, 1700086400)`, with no final-incremental descriptor. The
   membership assertion MUST not assume iterator order.
4. MUST derive the Full index from the public manifests iterator, read target
   `Timestamp(1700172800)`, and require exactly
   `ReaderError::Chain(ChainError::Missing)` with no snapshot. Link this
   missing-chain result and its test to `PARSE-CHAIN-002`.
5. MUST NOT link this case to `PARSE-NAME-005`: `.p7m` is an unsupported
   envelope, not a partial marker. MUST NOT claim CMS decoding, authentication,
   signature validation, or cryptographic support.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the unsupported first-incremental manifest and partial-manifest cases.
- Add Duvet links for the unsupported-envelope classification and test to
  `PARSE-NAME-001`, and for the exact missing-chain result and test to
  `PARSE-CHAIN-002`. Do not add a `PARSE-NAME-005` link.
- Run focused Duvet queries for `PARSE-NAME-001` and `PARSE-CHAIN-002`, full
  trace, focused/full core tests, formatting, strict Clippy, and
  `cargo xtask ci`. Require hosted Linux assurance and Duvet Pages publication
  before acceptance is complete.
- Independent review MUST verify the single-key mutation, twelve-object
  inventory and byte/size/identity preservation, parsed name/status, exact
  order-independent descriptor membership, iterator-derived Full index,
  target timestamp, and exact `Chain(Missing)` result.

## Exclusions

This acceptance covers only the final-incremental `.p7m` name in the official
plain fixture. It does not test partial artifacts, other unsupported
envelopes, arbitrary missing-chain behavior, CMS decoding, signatures,
cryptographic integrity, S3 identity semantics, filesystem restoration,
complete reader/chain-controller refinement, or a formal reader proof. It
adds no `PARSE-NAME-005` claim.

## Checklist

- [x] Independent specification review accepts the scope, mutation, and
      public oracle.
- [ ] The exact inventory relation and bytes/size/identity preservation pass;
      the renamed object is visible as the specified unsupported incremental
      manifest.
- [ ] The manifests iterator contains exactly the Full and first Incremental
      descriptors independent of order; the iterator-derived Full index
      returns exact `Chain(Missing)` at `1700172800`.
- [ ] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
