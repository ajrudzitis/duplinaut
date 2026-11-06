# Official Plain Reader PKCS#7 Full-Signature Archive Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a recognized full-signatures archive with the unsupported `.p7m`
envelope remains visible as an `UnsupportedEnvelope` catalog diagnostic but
does not prevent reconstruction of the official plain chain. The public
reader MUST return the official final snapshot while reading only its required
manifests and data volumes. This is one renamed object in the official plain
3.0.7 fixture. It makes no CMS/PKCS#7 decoding, signature, or cryptographic
support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the full-signatures
archive key:

```text
backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz
    -> backup/duplicity-full-signatures.20231114T221320Z.sigtar.p7m
```

The renamed object MUST retain the original stored signature-archive bytes,
size, and fixture source identity. The local source's version ID remains
synthetic; this test makes no S3 version/ETag claim. No object is added or
removed, the listing remains exactly twelve objects, and this rename is the
only key-set change. All three manifests and all six full/incremental data
volumes MUST remain under their original keys with original bytes, sizes, and
identities. Both incremental signature archives also remain unchanged.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. Its parsed name is
`FullSignatures` with timestamp `1700000000` and envelope `Pkcs7`; discovery
classifies it as unsupported. Its stored bytes remain the original gzip
signature-archive payload and are not claimed to be CMS content. This test
does not decode the archive.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with parsed kind `FullSignatures`, timestamp
   `1700000000`, envelope `Envelope::Pkcs7`, and status
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert `Catalog::manifests()` has exactly the three completed descriptors,
   independent of iterator order: Full `(1700000000, 1700000000)`, first
   Incremental `(1700000000, 1700086400)`, and second Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete resulting state to the `mode == "plain", index == 2`
   entry set in `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Record opened object keys in the local source harness. The exact opened-key
   set MUST be the three manifests and six required data volumes below; it
   MUST exclude the renamed full-signatures archive and both retained
   incremental-signatures archives:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz
   ```

## Snapshot oracle

Compare every retained entry with the official `expected.json` final plain
snapshot oracle:

- Require exact entry count and path identity as ordered raw component bytes
  from `components_hex`; do not derive identity from rendered or slash-joined
  paths. Require the iteration order from `SnapshotState::entries()` to match
  the oracle's raw-component lexicographic order, preserving component
  boundaries and proper-prefix ordering.
- Require exact kind and `mode & 0o777`. For regular files and directories,
  convert oracle `mtime_ns` to checked signed `i64`, then compare seconds via
  `div_euclid(1_000_000_000)` and nanoseconds via
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `expected.json` records `symlink_mtime_ns` in
  `official_comparison_exclusions`, and the producer skips symlink `utime`.
- For regular files, require exact `size` and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST have exact
  path, kind, mode, and timestamp. Do not skip entries whose content is
  absent.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory and all stored
   bytes, sizes, and source identities except for the specified full-signature
   key rename. Link the parsed supported-name classification and unsupported
   diagnostic to `PARSE-NAME-001`.
2. MUST retain exactly the three expected completed descriptors and MUST
   successfully read the final target using a Full anchor followed by the
   uniquely contiguous Incremental sequence. Link the chain assertion to
   `PARSE-CHAIN-001`.
3. MUST match the complete final plain snapshot to the documented fields in
   `expected.json`, including raw component paths, kinds, modes, supported
   timestamps, file sizes/hashes, and symlink targets. Omit symlink mtime as
   documented by `official_comparison_exclusions`. Link this oracle to
   `PARSE-PATH-001` and `ASSURE-TEST-001`.
4. MUST record source opens and assert exactly the three manifest and six data
   volume keys listed above; the renamed unsupported full-signatures object
   MUST not be opened. This is a bounded observation for this plain fixture
   reconstruction, not a claim about all auxiliary artifacts or profiles.
5. MUST NOT claim CMS decoding, signature validation, or cryptographic
   support. Do not link this case to `PARSE-NAME-005` or OpenPGP authentication
   requirements.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from `reader-unused-signature-archive-acceptance.md`, which mutates
  signature-archive bytes rather than its key/envelope.
- Add Duvet test annotations for name classification, contiguous chain
  restoration, and the official snapshot/path oracle to `PARSE-NAME-001`,
  `PARSE-CHAIN-001`, `PARSE-PATH-001`, and `ASSURE-TEST-001` respectively.
  Do not add `PARSE-NAME-005` or crypto-authentication traces.
- Run the focused test, targeted Duvet queries, full trace, focused/full core
  tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify exact inventory mutation, unsupported
  diagnostic, all descriptor intervals, opened-key set, and every oracle
  field/exclusion without broadening the unsupported-envelope claim.

## Exclusions

This acceptance covers only the full-signatures archive in the official
plain fixture. It does not test other signature archives, signed or encrypted
profiles, CMS decoding, cryptographic verification, partial objects, other
unsupported-envelope classes, S3 identity semantics, filesystem restoration,
complete reader-controller refinement, or formal reader proof. It adds no
claim that duplicity signature archives are unnecessary in other operations.

## Checklist

- [x] Independent specification review accepts the scope, fixture mutation,
      opened-key set, and snapshot oracle.
- [ ] The renamed archive remains visible as parsed
      `FullSignatures`/`Pkcs7`/`UnsupportedEnvelope`, with exact inventory and
      byte/size/identity preservation.
- [ ] Public final snapshot exactly matches the documented oracle fields and
      the opened-key set equals the three manifests plus six data volumes.
- [ ] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
