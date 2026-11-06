# Official Plain Reader PKCS#7 Final-Incremental Manifest Exclusion Acceptance

**Status:** The bounded fixture test, targeted Duvet checks, full core suite,
formatting, strict Clippy, independent implementation review, and full local
`cargo xtask ci` pass. Hosted Linux assurance/Pages remain pending.

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
- [x] The exact inventory relation and bytes/size/identity preservation pass;
      the renamed object is visible as the specified unsupported incremental
      manifest.
- [x] The manifests iterator contains exactly the Full and first Incremental
      descriptors independent of order; the iterator-derived Full index
      returns exact `Chain(Missing)` at `1700172800`.
- [x] Targeted `PARSE-NAME-001`/`PARSE-CHAIN-002` Duvet queries, full trace,
      focused/full core tests, formatting, strict Clippy, and diff check pass.
- [x] Independent implementation/test review passes.
- [x] Full local `cargo xtask ci` passes (2,047/0, 3,183/0, and 1,329/0
      verified; the source digest is recorded in generated
      `target/assurance/ci.json`).
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- `cargo test -p duplicity-core --test reader
  official_pkcs7_final_incremental_manifest_is_visible_but_leaves_missing_transition`:
  passed.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-name-001`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-chain-002`: passed.
- `DUVET="$PWD/.tools/bin/duvet" cargo xtask trace-update` and `cargo xtask
  trace`: passed.
- `cargo fmt --all -- --check`, `cargo clippy -p duplicity-core --all-targets
  -- -D warnings`, and `git diff --check`: passed.
- `nix-shell tools/shell.nix --run 'source tools/env.sh && cargo xtask ci'`:
  passed with fresh Verus, Duvet, formatting, strict Clippy, and workspace
  tests. The generated receipt records the source digest in
  `target/assurance/ci.json`.
- The original plain manifest bytes remain under a `.p7m` key; this test makes
  no CMS, crypto, or PNAME-005 claim.
