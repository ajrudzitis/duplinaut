# Official Plain Reader PKCS#7 First-Incremental Volume 2 Acceptance

**Status:** Implementation and independent review passed. Focused/full core
tests, targeted Duvet queries/full trace, formatting, and strict Clippy passed.
Local pinned `cargo xtask ci` passed. Hosted Linux assurance and Duvet Pages
are pending.

## Goal and scope

Verify that a recognized first-incremental volume 2 with the unsupported
`.p7m` envelope remains visible in discovery but cannot satisfy the required
volume. The public reader MUST report `MissingVolume { number: 2 }` for the
first incremental target even though volume 1 remains available. This is one
renamed object in the official plain Duplicity 3.0.7 fixture. It makes no CMS,
decryption, or cryptographic-support claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only first-incremental volume 2:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.p7m
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. Its local identity is synthetic; this test makes no S3
version/ETag claim. No object is added or removed, the listing remains exactly
twelve objects, and this rename is the only key-set difference. The full set,
all three manifests, first-incremental volume 1, final-incremental volume 1,
and all three signatures archives MUST remain at their original keys with
their original bytes, sizes, and identities. The unsupported object retains
the original gzip payload; it is not claimed to be CMS data.

The renamed key is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as
`IncrementalVolume(1700000000, 1700086400, 2)` with envelope `Pkcs7` and no
partial marker. Discovery classifies it as `UnsupportedEnvelope`.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with kind `IncrementalVolume`, interval
   `(1700000000, 1700086400)`, volume ordinal `Some(2)`, `partial == false`,
   envelope `Envelope::Pkcs7`, and status
   `DiscoveryStatus::UnsupportedEnvelope`.
2. Assert that first-incremental volume 1 remains present under its original
   key, is parsed as `IncrementalVolume(1700000000, 1700086400, 1)`, and has
   `DiscoveryStatus::Completed`.
3. Assert `Catalog::manifests()` retains exactly the three official completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
4. Derive the Full descriptor index from the public iterator, clear the
   source-open log, and call public `Catalog::read_snapshot` at
   `Timestamp(1700086400)`. Require exactly
   `ReaderError::MissingVolume { number: 2 }`; the reader MUST NOT return a
   snapshot or treat the unsupported volume as satisfying the set.
5. Assert that this read opened exactly these six objects: the Full manifest
   and three Full volumes, the first Incremental manifest, and first
   Incremental volume 1. It MUST exclude the renamed `.p7m` object, first
   Incremental volume 2 under any key, and all final-Incremental objects.

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   ```

## Requirements

1. MUST preserve the exact twelve-object inventory, stored bytes, sizes, and
   fixture identities except for the specified key rename. MUST classify the
   renamed key as `IncrementalVolume(1700000000,1700086400,2)`/
   `Pkcs7`/`UnsupportedEnvelope`. Link these name and envelope observations to
   `PARSE-NAME-001`.
2. MUST retain the three completed manifest descriptors and the completed
   first-incremental volume 1 while refusing the unsupported volume 2 as a
   required member. The public read MUST return exactly
   `ReaderError::MissingVolume { number: 2 }` and no snapshot. Link this
   fail-closed missing-volume behavior to `PARSE-CHAIN-002`.
3. MUST assert the exact six-key open set for the attempted read and MUST NOT
   open the unsupported object. This is scoped to this official plain fixture
   and target date.
4. MUST make no claim about CMS decoding, crypto, other unsupported-envelope
   classes, S3 identity, or filesystem restoration. Do not link this case to
   `PARSE-NAME-005` or OpenPGP authentication requirements.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the existing first-incremental-volume-1 `.p7m`, full-volume-2 `.p7m`,
  and partial-volume cases. The test MUST change only this key and MUST retain
  its stored payload bytes.
- Add Duvet links for parsed name/envelope classification and exact required
  volume rejection to `PARSE-NAME-001` and `PARSE-CHAIN-002`. Do not add
  `PARSE-NAME-005`, snapshot-path, or crypto-authentication traces.
- Run the focused test, targeted Duvet queries and full trace, focused/full
  core tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted
  Linux assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the key mutation, payload preservation,
  recognized unsupported diagnostic, retained volume 1, exact descriptor
  membership, exact `MissingVolume` value, and six-key opened set.

## Exclusions

This acceptance covers only first-incremental volume 2 in the official plain
fixture. It does not test other volume ordinals, data profiles, partial
artifacts, other unsupported-envelope classes, CMS, OpenPGP, encryption, S3
identity semantics, filesystem restoration, complete reader/controller
refinement, or formal reader proof.

## Checklist

- [x] Independent specification review accepts the mutation, public outcomes,
      six-key open set, and scope.
- [x] Discovery retains the renamed object as
      `IncrementalVolume(1700000000,1700086400,2)`/`Pkcs7`/
      `UnsupportedEnvelope`, and volume 1 remains completed.
- [x] The first-incremental read returns exactly
      `MissingVolume { number: 2 }`, no snapshot, and opens exactly the six
      listed objects.
- [x] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      and strict Clippy pass.
- [x] Local pinned `cargo xtask ci` passes.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
