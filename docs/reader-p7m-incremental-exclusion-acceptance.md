# Official Plain Reader PKCS#7 First-Incremental Manifest Exclusion Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a recognized first-incremental manifest with the unsupported
`.p7m` envelope remains visible as an `UnsupportedEnvelope` catalog diagnostic
but does not participate in the completed chain. The catalog must retain the
full anchor and later incremental descriptor, while public snapshot reading
reports the missing transition. This acceptance covers one mutation of the
official plain 3.0.7 fixture. It makes no CMS/PKCS#7 decoding or cryptographic
support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the first incremental
manifest key:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.p7m
```

The renamed object MUST retain the original manifest bytes, stored size, and
content-derived ETag; the local source retains the fixture's synthetic
version ID. No object is added or removed, the listing remains exactly twelve
objects, and the only key-set difference is this rename. The full manifest,
later incremental manifest, all data volumes, and both signature archives
MUST remain present with their original bytes, sizes, and fixture source
identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. Its parsed incremental interval is
`(1700000000, 1700086400)`. The stored bytes remain the original plain-manifest
payload; the `.p7m` suffix does not establish that they are CMS content. The
acceptance exercises name classification and completed-chain selection only,
not decoding or authenticating the payload.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()`. Its parsed name MUST have kind
   `IncrementalManifest`, interval start/end `1700000000`/`1700086400`, and
   envelope `Envelope::Pkcs7`; its status MUST be
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Collect `Catalog::manifests()` and assert exact membership, without
   assuming iteration order: one Full descriptor with start/end
   `1700000000`/`1700000000`, and one retained later Incremental descriptor
   with start/end `1700086400`/`1700172800`. The unsupported first incremental
   MUST not appear as a completed descriptor.
3. Derive the Full descriptor's index from `Catalog::manifests()` and call
   public `Catalog::read_snapshot` at `Timestamp(1700172800)`. It MUST return
   exactly `Err(ReaderError::Chain(ChainError::Missing))`; no snapshot may be
   returned. The retained later incremental is a non-vacuity control: the
   failed read demonstrates the missing first transition in the chain.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, original bytes,
   sizes, and fixture identities. The only mutation is the specified first
   incremental manifest key rename to `.manifest.p7m`.
2. MUST keep the renamed object visible as a parsed
   `IncrementalManifest(1700000000, 1700086400)` with `Envelope::Pkcs7` and
   `DiscoveryStatus::UnsupportedEnvelope`. Link the classification and test to
   `PARSE-NAME-001`.
3. MUST assert the exact completed descriptor membership: Full
   `(1700000000, 1700000000)` and later Incremental
   `(1700086400, 1700172800)`, with no first-incremental descriptor. Membership
   assertions MUST be independent of iterator order.
4. MUST derive the Full index from the public manifests iterator, read target
   `Timestamp(1700172800)`, and require exactly
   `ReaderError::Chain(ChainError::Missing)` with no snapshot. Link this
   missing-chain result and its test to `PARSE-CHAIN-002`.
5. MUST NOT link the case to `PARSE-NAME-005`: `.p7m` is an unsupported
   envelope, not a partial marker. MUST NOT claim CMS decoding, authentication,
   signature validation, or cryptographic support.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the partial first-incremental and unsupported full-manifest cases.
- Add Duvet links for the unsupported-envelope classification and test to
  `PARSE-NAME-001`, and for the missing-chain result and test to
  `PARSE-CHAIN-002`. Do not add a `PARSE-NAME-005` link.
- Run focused `PARSE-NAME-001` and `PARSE-CHAIN-002` queries, full Duvet trace,
  focused/full core tests, formatting, strict Clippy, and `cargo xtask ci`.
  Require hosted Linux assurance and Duvet Pages publication before acceptance
  is complete.
- Independent review MUST verify the sole-key mutation, exact twelve-object
  inventory and byte/size/identity preservation, parsed name and unsupported
  status, order-independent descriptor membership, iterator-derived Full
  index, target timestamp, and exact `Chain(Missing)` result.

## Exclusions

This acceptance covers only the first incremental `.p7m` name in the official
plain fixture. It does not test partial artifacts, other unsupported
envelopes, arbitrary missing-chain behavior, CMS decoding, signatures,
cryptographic integrity, S3 identity semantics, filesystem restoration,
complete reader/chain-controller refinement, or a formal reader proof. It
adds no `PARSE-NAME-005` claim.

## Checklist

- [x] Independent specification review accepts the scope, fixture mutation,
      and exact public oracle.
- [ ] Exact inventory relation and byte/size/identity preservation pass; the
      renamed object is visible with the specified parsed fields and
      `UnsupportedEnvelope` status.
- [ ] The manifests iterator contains exactly the specified Full and later
      Incremental descriptors, independent of order; the iterator-derived
      Full index returns exact `Chain(Missing)` at target `1700172800`.
- [ ] Targeted Duvet queries and full trace, focused/full core tests,
      formatting, strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
