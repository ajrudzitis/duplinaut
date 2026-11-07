# Official Plain Reader PKCS#7 Final-Incremental Signature Archive Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a recognized final-incremental signatures archive with the
unsupported `.p7m` envelope remains visible as an `UnsupportedEnvelope`
diagnostic but does not prevent reconstruction of the official plain chain.
The public reader MUST produce the official final snapshot while opening only
its required manifests and data volumes. This is one renamed object in the
official plain Duplicity 3.0.7 fixture. It makes no CMS, signature-validation,
or cryptographic-support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final-incremental
signatures archive key:

```text
backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz
    -> backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.p7m
```

The renamed object MUST retain its original stored signature-archive bytes,
size, and fixture source identity. Its local source identity is synthetic; the
test makes no S3 version/ETag claim. No object is added or removed, the listing
remains exactly twelve objects, and the rename is the only key-set difference.
All three manifests and all six data volumes MUST retain their original keys,
bytes, sizes, and identities. The full-signatures and first-incremental
signatures archives also remain unchanged.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. Its parsed name is
`IncrementalSignatures` for `(1700086400, 1700172800)`, has no volume ordinal,
and has envelope `Pkcs7`. Discovery classifies the object as unsupported. Its
stored bytes remain the original gzip signature-archive payload; the test does
not decode it or claim it is CMS content.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with kind `IncrementalSignatures`, start `1700086400`,
   end `1700172800`, `volume == None`, envelope `Envelope::Pkcs7`, and status
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert `Catalog::manifests()` contains exactly the three completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700172800)`. Require success and
   compare the complete state with `mode == "plain", index == 2` in
   `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Record opened keys. The exact set MUST contain only these three manifests
   and six data volumes; it MUST exclude the renamed unsupported archive and
   both retained signatures archives:

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

Compare every final entry with the official `expected.json` final plain
snapshot:

- Require exact entry count and raw component-byte identity from
  `components_hex`; preserve component boundaries and compare
  `SnapshotState::entries()` order to the oracle's raw-component
  lexicographic order.
- Require exact entry kind and `mode & 0o777`. For regular files and
  directories, convert `mtime_ns` to checked signed `i64`; compare seconds via
  `div_euclid(1_000_000_000)` and nanoseconds via
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `official_comparison_exclusions` records `symlink_mtime_ns`, and the producer
  skips symlink `utime`.
- For regular files, require exact size and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match their
  recorded paths, kinds, modes, and timestamps. Do not skip entries without
  content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory and all object
   bytes, sizes, and source identities except for renaming the final-
   incremental signatures key to `.sigtar.p7m`. Link parsed kind, interval,
   envelope, and unsupported diagnostic classification to `PARSE-NAME-001`.
2. MUST retain all three completed manifest descriptors and successfully read
   the final target through the contiguous Full-plus-Incremental chain. Link
   this restoration to `PARSE-CHAIN-001`.
3. MUST compare the complete final snapshot with the recorded `expected.json`
   fields, including raw component paths/order, kind/mode, supported
   timestamps, regular-file size/hash, and symlink target; omit symlink mtime
   as documented. Link these assertions to `PARSE-PATH-001` and
   `ASSURE-TEST-001`.
4. MUST assert the exact nine-key open set above. No signature archive is
   opened by this fixture read. This is a bounded observation of this
   reconstruction, not a general statement about auxiliary artifacts.
5. MUST NOT claim CMS decoding, signature validation, cryptographic support,
   or that signature archives are unnecessary in other operations. Do not
   link this case to `PARSE-NAME-005` or OpenPGP authentication requirements.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the full-signatures `.p7m`, partial-signature, and unused-archive cases.
  The test MUST change only this key and MUST preserve stored bytes.
- Add Duvet links for name classification, contiguous-chain restoration,
  byte-path preservation, and the official snapshot oracle to
  `PARSE-NAME-001`, `PARSE-CHAIN-001`, `PARSE-PATH-001`, and `ASSURE-TEST-001`.
  Do not add `PARSE-NAME-005` or crypto-authentication traces.
- Run focused/full core tests, targeted Duvet queries and full trace,
  formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the exact inventory mutation, retained
  payload identity, unsupported diagnostic, descriptor membership, every
  oracle field/exclusion, and the exact opened-key set without broadening the
  unsupported-envelope claim.

## Exclusions

This acceptance covers only the final-incremental signatures archive in the
official plain fixture. It does not test the full or first-incremental
signatures archive, partial archives, other signature/profile combinations,
CMS decoding, cryptographic verification, other unsupported-envelope
classes, S3 identity semantics, filesystem restoration, complete
reader/controller refinement, or formal reader proof.

## Checklist

- [x] Independent specification review accepts the fixture mutation, public
      outcomes, open set, and oracle.
- [ ] Discovery retains the renamed archive as
      `IncrementalSignatures(1700086400,1700172800)`/
      `Pkcs7`/`UnsupportedEnvelope`, with exact inventory and payload identity.
- [ ] The public final snapshot matches every documented oracle field and
      opens exactly the three manifests plus six data volumes.
- [ ] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
