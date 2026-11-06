# Official Plain Reader PKCS#7 Incremental-Volume Exclusion Acceptance

**Status:** The bounded fixture test, targeted Duvet checks, and local core
verification pass. Independent implementation review and full local
`cargo xtask ci` pass; hosted Linux assurance/Pages remain pending.

## Goal and scope

Verify that a recognized first-incremental data-volume name with the
unsupported `.p7m` envelope remains visible as an `UnsupportedEnvelope`
catalog diagnostic but cannot satisfy the required first volume of that
incremental set. This acceptance covers one key mutation in the official
plain 3.0.7 fixture. It checks public name classification and required-volume
completeness only; it makes no CMS/PKCS#7 decoding or cryptographic support
claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only first-incremental volume
1:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.p7m
```

The renamed object MUST retain its original stored bytes, size, and
fixture-derived source identity. The local source harness uses the existing
synthetic version ID; no S3 version or ETag behavior is inferred. No object is
added or removed, the listing remains exactly twelve objects, and the only
key-set difference is this rename. The full manifest and all full volumes,
both incremental manifests, first-incremental volume 2, the later
incremental volumes, and both signature archives MUST remain present with
their original bytes, sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. Its parsed interval is
`(1700000000, 1700086400)`, and its volume ordinal is 1. The stored bytes remain
the original gzip fixture payload; the `.p7m` suffix does not establish that
they are CMS content. The acceptance does not decode or authenticate the
renamed payload.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()`. Its parsed name MUST have kind `IncrementalVolume`,
   interval start/end `1700000000`/`1700086400`, volume `Some(1)`, and envelope
   `Envelope::Pkcs7`; its status MUST be
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert that the retained first-incremental volume 2 remains visible as a
   completed `IncrementalVolume` with the same interval and ordinal
   `Some(2)`. This proves the exact missing ordinal is 1 while a later volume
   in the set remains available.
3. Collect `Catalog::manifests()` and assert exact membership independent of
   iteration order: the Full descriptor `(1700000000, 1700000000)`, first
   Incremental descriptor `(1700000000, 1700086400)`, and later Incremental
   descriptor `(1700086400, 1700172800)`.
4. Derive the Full descriptor index from the public manifests iterator and
   call `Catalog::read_snapshot` at `Timestamp(1700086400)`. It MUST return
   exactly `Err(ReaderError::MissingVolume { number: 1 })`; no snapshot may be
   returned.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, stored bytes,
   sizes, and fixture identities. The sole mutation is the specified first
   incremental volume-1 key rename to `.difftar.p7m`.
2. MUST keep the renamed object visible with parsed kind `IncrementalVolume`,
   interval `(1700000000, 1700086400)`, ordinal 1, envelope `Pkcs7`, and
   `DiscoveryStatus::UnsupportedEnvelope`. Link this name classification and
   its test to `PARSE-NAME-001`.
3. MUST retain and assert the completed incremental volume-2 control and the
   exact three completed manifest descriptors. The descriptor membership
   assertion MUST not assume iterator order.
4. MUST derive the Full descriptor index from the public iterator, read target
   `Timestamp(1700086400)`, and require exactly
   `ReaderError::MissingVolume { number: 1 }` with no snapshot. Link this
   missing-required-volume result and its test to `PARSE-CHAIN-002`.
5. MUST NOT link this case to `PARSE-NAME-005`: `.p7m` is an unsupported
   envelope, not a partial marker. MUST NOT claim CMS decoding, authentication,
   signature validation, or cryptographic support.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from full-volume and incremental-manifest `.p7m` cases.
- Add Duvet links for unsupported-envelope classification to
  `PARSE-NAME-001` and for exact missing-volume behavior to `PARSE-CHAIN-002`.
  Do not add a `PARSE-NAME-005` link.
- Run focused Duvet queries for both requirements, full trace, focused/full
  core tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted
  Linux assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the one-key mutation, exact fixture
  inventory/payload/size/identity preservation, parsed object fields and
  status, retained volume 2, exact manifest membership, iterator-derived Full
  index, target timestamp, and exact `MissingVolume { number: 1 }` result.

## Exclusions

This acceptance covers only first-incremental volume 1 with a `.p7m` suffix
in the official plain fixture. It does not test partial artifacts, other
unsupported envelopes, arbitrary missing-volume cases, CMS decoding,
signatures, cryptographic integrity, S3 identity semantics, filesystem
restoration, complete reader/chain-controller refinement, or a formal reader
proof. It adds no `PARSE-NAME-005` claim.

## Checklist

- [x] Independent specification review accepts the scope, mutation, and
      public oracle.
- [x] Exact twelve-object mutation and byte/size/identity preservation pass;
      the renamed volume 1 is visible as unsupported and completed volume 2
      remains visible.
- [x] All three expected descriptors are present independent of order; the
      iterator-derived Full index returns exact `MissingVolume { number: 1 }`
      at target `1700086400`.
- [x] Targeted `PARSE-NAME-001`/`PARSE-CHAIN-002` Duvet queries, full trace,
      focused/full core tests, formatting, strict Clippy, and diff check pass.
- [x] Independent implementation/test review passes.
- [x] Full local `cargo xtask ci` passes (2,047/0, 3,183/0, and 1,329/0
      verified; source digest is recorded in generated
      `target/assurance/ci.json`).
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- `cargo test -p duplicity-core --test reader
  official_pkcs7_first_incremental_volume_is_visible_but_cannot_satisfy_volume_one`:
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
- The original gzip volume bytes remain under a `.p7m` key; this test makes no
  CMS, crypto, or PNAME-005 claim.
