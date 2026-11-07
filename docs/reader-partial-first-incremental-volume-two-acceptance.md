# Official Plain Reader Partial First-Incremental Volume 2 Acceptance

**Status:** Implementation and independent implementation review passed.
Focused/full core tests, Duvet queries/full trace, formatting, strict Clippy,
and full pinned local CI passed. Hosted Linux/Pages assurance is pending.

## Goal and scope

Verify that a partial first-incremental volume 2 remains visible as an
interrupted artifact but is excluded from completed restore data. The reader
MUST report the exact missing-volume error for the first incremental target
even when volume 1 is present. This is one key mutation of the official plain
Duplicity 3.0.7 fixture; it does not establish general partial-artifact or
reader-controller behavior.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only first-incremental volume 2:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.part.gz
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. Its local identity is synthetic; this test makes no S3
version/ETag claim. No object is added or removed, the listing remains exactly
twelve objects, and this rename is the only key-set difference. All three
manifests, all three Full volumes, first-incremental volume 1, final-
incremental volume 1, and all signature archives MUST remain unchanged in key,
bytes, size, and identity.

The renamed key is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as
`IncrementalVolume(1700000000, 1700086400, 2)` with envelope `Gzip` and
`partial == true`. Discovery classifies it as `Partial`.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with kind `IncrementalVolume`, interval
   `(1700000000, 1700086400)`, volume ordinal `Some(2)`, envelope
   `Envelope::Gzip`, `partial == true`, and status `DiscoveryStatus::Partial`.
2. Assert first-incremental volume 1 remains visible under its original key,
   parsed as `IncrementalVolume(1700000000,1700086400,1)`, with
   `partial == false` and `DiscoveryStatus::Completed`.
3. Assert `Catalog::manifests()` retains exactly the official three completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
4. Derive the Full index from the public iterator, clear the source-open log,
   and read `Timestamp(1700086400)`. Require exactly
   `ReaderError::MissingVolume { number: 2 }`; the partial object MUST NOT
   satisfy the required volume and no snapshot may be returned.
5. Assert that this read opens exactly the Full manifest and its three
   volumes, the first Incremental manifest, and first-incremental volume 1.
   It MUST exclude the renamed partial object and all final-Incremental
   objects:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   ```

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, stored bytes,
   sizes, and identities except for the specified key rename. MUST retain the
   parsed volume kind, interval, ordinal, gzip envelope, and partial status.
   Link these classification observations to `PARSE-NAME-001`.
2. MUST retain the partial object as a visible interrupted artifact but
   exclude it from completed restore data. Link this policy to
   `PARSE-NAME-005`.
3. MUST refuse to return a first-incremental snapshot while volume 2 is
   missing from the completed set; return exactly
   `ReaderError::MissingVolume { number: 2 }`. Link this fail-closed outcome
   to `PARSE-CHAIN-002`.
4. MUST assert the exact six-key open set above and MUST NOT open the partial
   object. Scope this observation to the named fixture and target date.
5. MUST make no broader claim about partial data-volume handling, chain
   completeness, S3 identity, filesystem restoration, or complete
   reader/controller refinement.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the existing partial first-incremental volume 1 and unsupported
  first-incremental volume 2 cases. Change only the object key; retain the
  fixture payload, size, and identity.
- Add Duvet test links for parsed partial classification, exclusion from
  completed restore data, and exact missing-volume behavior to
  `PARSE-NAME-001`, `PARSE-NAME-005`, and `PARSE-CHAIN-002`. Do not link
  `PARSE-NAME-005` beyond the direct partial-object exclusion assertion, or
  `PARSE-PATH-001` and crypto-authentication requirements.
- Run the focused test, targeted Duvet queries/full trace, focused/full core
  tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the exact key mutation, payload and identity
  preservation, partial status, retained completed volume 1, descriptor set,
  missing-volume result, and six-key open set.

## Exclusions

This acceptance covers only first-incremental volume 2 in the official plain
fixture. It does not test other partial objects, unsupported envelopes,
non-plain profiles, volume parsing grammar, signature verification, S3
identity semantics, filesystem restoration, or formal/complete reader
controller refinement.

## Checklist

- [x] Independent specification review accepts the fixture mutation,
      partial-exclusion policy, exact result, and open set.
- [x] Discovery retains the renamed object as partial
      `IncrementalVolume(1700000000,1700086400,2)`/`Gzip`, and volume 1
      remains completed.
- [x] The attempted first-incremental read returns exactly
      `MissingVolume { number: 2 }`, no snapshot, and opens only the six listed
      objects.
- [x] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      strict Clippy, and diff checks pass locally.
- [x] Independent implementation/test review passes.
- [x] Full pinned local `cargo xtask ci` passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- Focused reader acceptance test passed (1/1).
- `cargo test -p duplicity-core` passed.
- Targeted Duvet queries for `PARSE-NAME-001`, `PARSE-NAME-005`, and
  `PARSE-CHAIN-002`, `cargo xtask trace-update`, and `cargo xtask trace` passed.
- `cargo fmt --all -- --check`, strict core Clippy, and `git diff --check`
  passed.
- Independent implementation review and full pinned local `cargo xtask ci`
  passed. Hosted Linux assurance/Pages remain pending.
