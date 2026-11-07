# Official Plain Reader PKCS#7 Final-Incremental Volume Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a recognized final-incremental data volume with the unsupported
`.p7m` envelope remains visible in discovery, cannot satisfy the required
volume, and does not invalidate the earlier completed snapshot. The public
reader MUST restore the snapshot at the previous date and MUST return the
exact missing-volume error at the final date. This is one renamed object in the
official plain Duplicity 3.0.7 fixture. It makes no CMS, decryption, or
cryptographic-support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the final-incremental
volume 1 key:

```text
backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz
    -> backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.p7m
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. Its local identity is synthetic; this test makes no S3
version/ETag claim. No object is added or removed, the listing remains exactly
twelve objects, and this rename is the only key-set difference. All three
manifests, the other five data volumes, and all three signatures archives MUST
remain unchanged in key, bytes, size, and identity.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as
`IncrementalVolume(1700086400, 1700172800, 1)` with envelope `Pkcs7` and no
partial marker. Discovery reports `UnsupportedEnvelope`; the stored bytes
remain the original gzip volume payload and are not claimed to be CMS data.

## Required public observations

1. Call public `Catalog::discover` and assert that the renamed object remains
   visible in `Catalog::objects()` with kind `IncrementalVolume`, interval
   `(1700086400, 1700172800)`, volume ordinal `Some(1)`, `partial == false`,
   envelope `Envelope::Pkcs7`, and status
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert `Catalog::manifests()` retains exactly the official three completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator. Read
   `Timestamp(1700086400)` and compare the successful snapshot with
   `mode == "plain", index == 1` in
   `tests/fixtures/duplicity-3.0.7/expected.json`.
4. Reset the test source's open log. Read `Timestamp(1700172800)` and require
   exactly `ReaderError::MissingVolume { number: 1 }`. The final read MUST NOT
   treat the unsupported object as satisfying volume 1 or return a snapshot.
   Its exact opened-key set MUST contain these three manifests and five
   supported volumes, and MUST exclude the renamed `.p7m` key:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
   ```

## Prior-snapshot oracle

Compare the complete successful snapshot at `1700086400` with
`expected.json`, plain index 1:

- Require exact entry count and raw path-component bytes from `components_hex`;
  compare `SnapshotState::entries()` order to the oracle's lexicographically
  sorted raw component sequences and preserve component boundaries.
- Require exact entry kind and `mode & 0o777`. For regular files and
  directories, convert `mtime_ns` to checked signed `i64`; compare seconds via
  `div_euclid(1_000_000_000)` and nanoseconds via
  `rem_euclid(1_000_000_000)`. Do not assert symlink mtime:
  `official_comparison_exclusions` lists `symlink_mtime_ns`, and the producer
  skips symlink `utime`.
- For regular files, require exact size and SHA-256. For symlinks, require
  exact target bytes from `symlink_target_hex`. Directories MUST match their
  recorded path, kind, mode, and timestamp. Do not skip entries without
  content.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, stored bytes,
   sizes, and fixture identities except for the specified key rename. Link
   parsed kind, interval, ordinal, envelope, and unsupported diagnostic to
   `PARSE-NAME-001`.
2. MUST retain the three completed descriptors and restore the previous
   snapshot through the Full set and first Incremental set. Link the successful
   contiguous read to `PARSE-CHAIN-001`.
3. MUST compare the previous snapshot against every documented `expected.json`
   field, including raw component paths/order, kinds, modes, supported
   timestamps, regular-file size/hash, and symlink target; omit symlink mtime
   as documented. Link this oracle to `PARSE-PATH-001` and `ASSURE-TEST-001`.
4. MUST return exactly `MissingVolume { number: 1 }` for the final target and
   MUST NOT use the `.p7m` object as its required data volume. Link this
   missing-volume outcome to `PARSE-CHAIN-002`.
5. MUST assert the exact eight-key open set for the final read above; the
   unsupported `.p7m` key MUST NOT be opened. Scope this observation to this
   official plain fixture.
6. MUST make no claim about CMS decoding, crypto, or other unsupported
   envelope types. Do not link to `PARSE-NAME-005` or OpenPGP authentication
   requirements.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the full-volume `.p7m`, first-incremental-volume `.p7m`, and partial
  final-incremental-volume cases. The test MUST rename only the key and retain
  its bytes, size, and identity.
- Add Duvet links for parsed name/diagnostic classification, previous-snapshot
  chain success, raw paths and metadata, and exact missing-volume rejection
  to `PARSE-NAME-001`, `PARSE-CHAIN-001`, `PARSE-PATH-001`, `ASSURE-TEST-001`,
  and `PARSE-CHAIN-002`, respectively. Do not add `PARSE-NAME-005` or
  crypto-authentication traces.
- Run the focused test, targeted Duvet queries and full trace, focused/full
  core tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted
  Linux assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the fixture mutation, retained payload
  identity, unsupported object diagnostic, descriptor membership, previous
  oracle comparison, exact final error, and final-read open set.

## Exclusions

This acceptance covers only final-incremental volume 1 in the official plain
fixture. It does not test other volumes, partial artifacts, other
unsupported-envelope classes, CMS, OpenPGP, encryption, S3 identity semantics,
filesystem restoration, complete reader/controller refinement, or a formal
reader proof.

## Checklist

- [x] Independent specification review accepts the mutation, outcomes, open
      set, and previous-snapshot oracle.
- [ ] Discovery reports the renamed object as
      `IncrementalVolume(1700086400,1700172800,1)`/`Pkcs7`/
      `UnsupportedEnvelope`, preserving exact inventory and payload identity.
- [ ] The previous snapshot matches every documented oracle field; the final
      target returns exact `MissingVolume { number: 1 }` and opens exactly the
      eight listed objects.
- [ ] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
