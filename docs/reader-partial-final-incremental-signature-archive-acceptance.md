# Official Plain Reader Partial Final-Incremental Signature Archive Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a partial final-incremental signatures archive remains visible in
catalog diagnostics but does not prevent reconstruction of the official plain
chain. The public reader MUST restore the final snapshot using its required
manifests and data volumes without opening the signatures archive. This is one
key mutation of the official plain Duplicity 3.0.7 fixture. It makes no
signature-verification, cryptographic, or general auxiliary-artifact claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final-incremental
signatures archive key:

```text
backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz
    -> backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.part.gz
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. The local identity is synthetic; this test makes no S3
version/ETag claim. No object is added or removed, the listing remains exactly
twelve objects, and this rename is the only key-set difference. All three
manifests, all six data volumes, the full-signatures archive, and the
first-incremental signatures archive MUST remain present with their original
bytes, sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as
`IncrementalSignatures` for the interval `1700086400` to `1700172800`, with no
volume ordinal and the partial marker.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()` with parsed kind `IncrementalSignatures`, start
   `1700086400`, end `1700172800`, `volume == None`, `name().partial == true`,
   and `DiscoveryStatus::Partial`.
2. Assert `Catalog::manifests()` retains exactly the official three completed
   descriptors, independently of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator and call public
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete state with `mode == "plain", index == 2` in
   `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Record source-opened keys. The exact set MUST contain the three required
   manifests and six data volumes, and MUST exclude the renamed partial
   final-incremental signatures archive and both retained signatures archives.
   This assertion is limited to the specified plain fixture reconstruction.

## Snapshot oracle

Compare every final snapshot entry with the official `expected.json` index 2:

- Require exact entry count and raw backup-relative path-component bytes from
  `components_hex`. Do not compare rendered or slash-joined paths; preserve
  component boundaries when matching path identities.
- Require exact entry kind and `mode & 0o777`. For regular files and
  directories, convert `mtime_ns` to checked signed `i64`; compare seconds
  with `div_euclid(1_000_000_000)` and nanoseconds with
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `expected.json` lists `symlink_mtime_ns` in
  `official_comparison_exclusions`, and the producer skips symlink `utime`.
- For regular files, require exact `size` and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match the
  recorded path, kind, mode, and timestamp. Do not skip entries without
  content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, bytes, sizes, and
   fixture source identities. The only mutation is renaming the final-
   incremental signatures key to `.sigtar.part.gz`.
2. MUST retain the renamed object in discovery with parsed kind
   `IncrementalSignatures`, interval `(1700086400, 1700172800)`, no volume
   ordinal, `name().partial == true`, and `DiscoveryStatus::Partial`. Link kind,
   interval, and partial-field classification to `PARSE-NAME-001`.
3. MUST assert the exact three completed manifest descriptors, independent of
   iterator order, and successfully read the final target through the
   contiguous full-plus-incremental chain. Link this restoration to
   `PARSE-CHAIN-001`.
4. MUST compare the complete final snapshot to the recorded `expected.json`
   fields, including raw component paths, kind/mode, supported timestamps,
   file size/hash, and symlink target; omit symlink mtime as documented. Link
   these assertions to `PARSE-PATH-001` and `ASSURE-TEST-001`.
5. MUST assert that source opens are exactly the three manifests and six data
   volumes; no signature archive is opened by this read. Scope this result to
   the official plain fixture.
6. MUST make no claim that signatures are verified, signature archives are
   unnecessary in other operations, or that any crypto, S3, filesystem
   publication, or complete reader/controller behavior is established.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from partial full-signature, partial first-incremental-signature, and
  unsupported PKCS#7 cases. The test MUST rename only the key; it MUST NOT
  modify stored bytes.
- Add Duvet test links for kind/interval/partial classification, successful
  chain restoration, raw path preservation, and official snapshot comparison
  to `PARSE-NAME-001`, `PARSE-CHAIN-001`, `PARSE-PATH-001`, and
  `ASSURE-TEST-001` respectively. Do not link `PARSE-NAME-005`, because
  signature archives are not members of a completed restore set here.
- Run targeted Duvet queries, full trace, focused/full core tests, formatting,
  strict Clippy, and `cargo xtask ci`. Require hosted Linux assurance and Duvet
  Pages publication before acceptance is complete.
- Independent review MUST verify the exact key mutation and payload
  preservation, parsed partial diagnostic, descriptor membership, final
  expected snapshot fields/exclusions, and exact opened-key set.

## Exclusions

This acceptance covers only the final-incremental signatures archive in the
official plain fixture. It does not test full or first-incremental signature
archives, other signature/profile combinations, signature verification,
OpenPGP/CMS, other partial artifacts, S3 identity, filesystem restoration,
complete reader/controller refinement, or a formal reader proof.

## Checklist

- [x] Independent specification review accepts the scope, fixture mutation,
      opened-key set, and snapshot oracle.
- [ ] The renamed archive remains visible as partial `IncrementalSignatures`
      for `(1700086400, 1700172800)`, with exact inventory and byte/size/
      identity preservation.
- [ ] Final state matches all documented oracle fields and the opened-key set
      contains only the three manifests plus six data volumes.
- [ ] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
