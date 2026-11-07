# Official Plain Reader Partial Incremental-Signature Archive Acceptance

**Status:** Specification accepted after independent review. The implementation and independent test review pass. Local focused/full core tests, Duvet trace, formatting, and strict Clippy pass. Local full `cargo xtask ci` passes. Hosted Linux assurance and Duvet Pages remain pending.

## Goal and scope

Verify that a partial first-incremental signature archive remains visible as
a catalog diagnostic but does not prevent reconstruction of the official
plain chain. The reader MUST restore the final snapshot from its manifests
and data volumes without opening signature archives. This is one renamed
object in the official plain duplicity 3.0.7 fixture. It establishes no
signature-verification, cryptographic, or general auxiliary-artifact claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the first-incremental
signature archive:

```text
backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz
    -> backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.part.gz
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. The local source identity is synthetic and establishes no S3
version or ETag claim. No object is added or removed, the inventory remains
exactly twelve objects, and the sole key-set difference is this rename. All
three manifests, all six data volumes, the full-signatures archive, and the
later incremental-signatures archive MUST remain present with their original
bytes, sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as an
`IncrementalSignatures` object for interval `(1700000000, 1700086400)` with
the partial marker and no volume ordinal.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   in `Catalog::objects()` with parsed kind `IncrementalSignatures`, start/end
   `1700000000`/`1700086400`, `volume == None`, `name().partial == true`, and
   `DiscoveryStatus::Partial`.
2. Assert `Catalog::manifests()` retains exactly the official three completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator and call public
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete result to the `mode == "plain", index == 2` entry set
   in `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Record source-opened keys. The exact open set MUST contain the three
   required manifests and six data volumes; it MUST exclude the renamed
   partial signature archive and both retained signature archives. The test
   establishes this read set only for the specified plain fixture path.

## Snapshot oracle

Compare every final snapshot entry to the official `expected.json` index 2:

- Require exact entry count and path identity using raw ordered component
  bytes from `components_hex`; do not compare rendered or slash-joined path
  strings. Preserve component boundaries when matching paths.
- Require exact kind and `mode & 0o777`. For regular files and directories,
  convert `mtime_ns` to checked signed `i64`, then compare seconds with
  `div_euclid(1_000_000_000)` and nanoseconds with
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `expected.json` lists `symlink_mtime_ns` in
  `official_comparison_exclusions`, and the producer skips symlink `utime`.
- For regular files, require exact `size` and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match the
  recorded path, kind, mode, and timestamp. Do not skip entries without
  content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, original bytes,
   sizes, and fixture source identities. The only mutation is the specified
   first-incremental signature key rename to `.sigtar.part.gz`.
2. MUST retain the renamed object in discovery with parsed kind
   `IncrementalSignatures`, interval `(1700000000, 1700086400)`, no volume,
   `name().partial == true`, and `DiscoveryStatus::Partial`. Link this parsed
   classification, including its partial flag, to `PARSE-NAME-001`.
3. MUST assert exact completed descriptor membership: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`, independent of iterator order.
4. MUST derive the Full index from the public manifests iterator, read target
   `Timestamp(1700172800)`, and require successful contiguous-chain
   reconstruction. Link this assertion to `PARSE-CHAIN-001`.
5. MUST compare all final entries to the documented `expected.json` fields,
   including raw component paths, kind/mode, supported timestamps, regular
   file size/hash, and symlink target bytes. Omit symlink mtime as recorded in
   `official_comparison_exclusions`. Link the byte-path and official oracle
   assertions to `PARSE-PATH-001` and `ASSURE-TEST-001`.
6. MUST assert the exact source-open key set is the three manifests and six
   data volumes; no signature archive is opened during this read. This is
   limited to this plain fixture restoration path.
7. MUST make no signature-verification, CMS/cryptographic, S3, filesystem
   publication, or complete reader-controller claim.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the unsupported full-signatures and unused signed-signature-archive
  cases. The test MUST rename the key only; it MUST NOT mutate archive bytes.
- Add Duvet test links for kind classification/partial status, successful
  chain selection, raw path bytes, and official snapshot comparison to
  `PARSE-NAME-001`, `PARSE-CHAIN-001`, `PARSE-PATH-001`, and
  `ASSURE-TEST-001` respectively. Do not link `PARSE-NAME-005`; signature
  archives are not members of the completed restore sets in this scenario.
- Run targeted Duvet queries, full trace, focused/full core tests, formatting,
  strict Clippy, and `cargo xtask ci`. Require hosted Linux assurance and
  Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the sole-key mutation, exact inventory and
  payload preservation, partial diagnostic, descriptor set, final oracle
  fields/exclusions, and exact opened-key set.

## Exclusions

This acceptance covers only the first-incremental signature archive in the
official plain fixture. It does not test other signature archive objects,
signature verification, signed/encrypted profiles, CMS, other unsupported
envelopes, arbitrary irrelevant objects, S3 identity, filesystem restoration,
complete reader/controller refinement, or a formal reader proof. It does not
claim signature archives are unnecessary in backup creation or other reader
operations.

## Checklist

- [x] Independent specification review accepts the scope, mutation, opened
      key set, and final snapshot oracle.
- [x] Exact twelve-object key relation and original bytes/size/identity pass;
      the renamed object is visible as partial IncrementalSignatures.
- [x] The exact three descriptors remain present; final state matches the
      documented oracle and the read set contains only manifests/volumes.
- [x] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and diff checks pass.
- [x] Local full `cargo xtask ci` passes.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
