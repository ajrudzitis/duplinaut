# Official Plain Reader PKCS#7 Full-Volume Exclusion Acceptance

**Status:** Specification and implementation/test independently reviewed.
Focused/full tests, Duvet, strict Clippy, formatting, and local full
`cargo xtask ci` pass. Hosted Linux assurance and Duvet Pages publication
remain pending.

## Goal and scope

Verify that a recognized full data-volume name with the unsupported `.p7m`
envelope remains visible as an `UnsupportedEnvelope` catalog diagnostic but
does not satisfy the required volume ordinal for a snapshot. Exercise only
full volume 2 from the official plain 3.0.7 fixture, while retaining completed
full volume 3 as a non-vacuity control. This covers name classification and
required-volume completeness; it makes no CMS/PKCS#7 decoding or cryptographic
support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only full volume 2:

```text
backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
    -> backup/duplicity-full.20231114T221320Z.vol2.difftar.p7m
```

The renamed object MUST retain the original volume-2 stored bytes, size, and
fixture source identity. The local source uses the existing synthetic identity
for each fixture object. No object is added or dropped, the listing remains
exactly twelve objects, and the only key-set difference is this rename. The
full manifest, full volume 3, both incremental manifests and volumes, and both
signature archives MUST remain present with their original bytes, sizes, and
identities.

The original bytes are the fixture's `.difftar.gz` payload. Renaming the key
does not make those bytes CMS content; the acceptance MUST NOT claim that they
are valid PKCS#7 data. Discovery classifies the suffix, and the public reader
must report the absent required completed ordinal without attempting to decode
the renamed payload.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()`. Its parsed name MUST have kind `FullVolume`, volume
   `Some(2)`, and envelope `Envelope::Pkcs7`; its status MUST be
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert the retained full volume 3 is still visible as a completed
   `FullVolume` with ordinal `Some(3)`. This control proves the exact required
   ordinal 2 is absent while a later volume remains available.
3. Call public `Catalog::read_snapshot` using the official full-set anchor and
   `Timestamp(1700000000)`. It MUST return exactly
   `Err(ReaderError::MissingVolume { number: 2 })`; no snapshot may be
   returned. The result MUST be about the missing required volume, not an
   unsupported-envelope error or a successful but incomplete snapshot.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, bytes, sizes, and
   source identities. The sole mutation is the specified full-volume-2 key
   rename to `.difftar.p7m`.
2. MUST classify the renamed basename as `FullVolume`, ordinal 2, envelope
   `Pkcs7`, with `DiscoveryStatus::UnsupportedEnvelope`. Link this public
   classification to `PARSE-NAME-001`.
3. MUST retain and assert full volume 3 as a completed ordinal-3 control.
4. MUST call public `Catalog::read_snapshot` for the full anchor at
   `Timestamp(1700000000)` and assert exactly
   `ReaderError::MissingVolume { number: 2 }`, with no snapshot. Link this
   required-volume result to `PARSE-CHAIN-002`.
5. MUST NOT link this case to `PARSE-NAME-005`; `.p7m` is an unsupported
   envelope, not a partial marker. MUST NOT claim CMS decoding, authentication,
   signature validation, or cryptographic support.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the partial-volume and unsupported-manifest cases.
- Add Duvet links for the unsupported-envelope classification and its test to
  `PARSE-NAME-001`, and for the exact missing-volume assertion and its test to
  `PARSE-CHAIN-002`. Do not add a `PARSE-NAME-005` link.
- Run focused queries for `PARSE-NAME-001` and `PARSE-CHAIN-002`, then the full
  trace, focused/full core tests, formatting, strict Clippy, and
  `cargo xtask ci`. Require hosted Linux assurance and Duvet Pages publication
  before marking acceptance complete.
- Independent implementation review MUST verify the sole-key mutation,
  twelve-object inventory and byte/identity preservation, parsed kind/ordinal/
  envelope/status, retained completed volume 3, and exact public
  `MissingVolume { number: 2 }` result.

## Exclusions

This acceptance covers only the full volume-2 `.p7m` name in the official
plain fixture. It does not test CMS decoding, signatures, cryptographic
integrity, validity of the gzip bytes as CMS, partial-artifact policy, other
unsupported envelopes, arbitrary missing-volume behavior, S3 identity
semantics, filesystem restoration, complete `Catalog` controller
refinement, or a formal reader proof.

## Checklist

- [x] Independent specification review accepts scope, mutation, and oracle.
- [x] Exact inventory rename relation and byte/size/identity preservation
      pass; the renamed volume is visible as unsupported PKCS#7 volume 2 and
      original completed volume 3 remains visible.
- [x] Public `read_snapshot` at the full anchor and target time returns exact
      `MissingVolume { number: 2 }` with no snapshot.
- [x] Targeted `PARSE-NAME-001` and `PARSE-CHAIN-002` Duvet queries, full trace,
      focused/full core tests, formatting, strict Clippy, and diff check pass.
- [x] Independent implementation/test review passes.
- [x] Full local `cargo xtask ci` passes with 2,047, 3,183, and 1,329 verified
      declarations and zero errors; Duvet, strict Clippy, formatting, and
      workspace tests pass.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- `cargo test -p duplicity-core --test reader
  official_pkcs7_full_volume_is_visible_but_cannot_satisfy_volume_two`:
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
- Full pinned CI wrote `target/assurance/ci.json` with source digest
  `21cf3c4f4736a381d1edbd294b21d4aa3b2838922feb316e1f114950aeb32aa0`.
- This fixture retains gzip archive bytes under a `.p7m` key and makes no CMS,
  crypto, or PNAME-005 claim.
