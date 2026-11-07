# Official Plain Reader Partial PKCS#7 First-Incremental Volume 2 Acceptance

**Status:** Implementation and independent implementation review passed.
Focused/full core tests, Duvet queries/full trace, formatting, and strict Clippy
passed locally. Full pinned CI and hosted Linux/Pages assurance are pending.

## Goal and scope

Verify the public diagnostic precedence for a supported name that is both
partial and uses the unsupported PKCS#7 envelope. Discovery MUST preserve
both parsed facts and report `Partial` as the object's status. The partial
object MUST NOT satisfy the required first-incremental volume, so the public
reader returns the exact missing-volume error. This acceptance covers one
combined-profile mutation of the official plain Duplicity 3.0.7 fixture; it
does not establish global diagnostic ordering for every artifact class.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only first-incremental volume 2:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.part.p7m
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. Its local identity is synthetic; this test makes no S3
version/ETag claim. No object is added or removed, the listing remains exactly
twelve objects, and this rename is the only key-set difference. All three
manifests, first-incremental volume 1, the Full set, final-incremental volume
1, and all three signatures archives MUST remain unchanged in key, bytes,
size, and identity.

The renamed key is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as
`IncrementalVolume(1700000000, 1700086400, 2)`, with `partial == true` and
`Envelope::Pkcs7`. Discovery MUST classify it as `Partial`, preserving the
partial diagnostic even though its parsed envelope is unsupported.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with kind `IncrementalVolume`, interval
   `(1700000000, 1700086400)`, volume ordinal `Some(2)`, `partial == true`,
   envelope `Envelope::Pkcs7`, and status `DiscoveryStatus::Partial`.
   Specifically reject `DiscoveryStatus::UnsupportedEnvelope` for this
   combined case.
2. Assert first-incremental volume 1 remains under its original key, with
   parsed tuple `IncrementalVolume(1700000000,1700086400,1)`, no partial marker,
   and `DiscoveryStatus::Completed`.
3. Assert `Catalog::manifests()` retains exactly the three official completed
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
4. Derive the Full index from the public iterator, clear the source-open log,
   and call `Catalog::read_snapshot` at `Timestamp(1700086400)`. Require exactly
   `ReaderError::MissingVolume { number: 2 }` and no snapshot. The combined
   partial/PKCS#7 object MUST NOT satisfy the required volume.
5. Assert the read opens exactly the Full manifest and volumes 1–3, the first
   Incremental manifest, and first-Incremental volume 1. It MUST exclude the
   combined-status object and all final-Incremental objects:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   ```

## Requirements

1. MUST preserve the exact twelve-object inventory and original object bytes,
   sizes, and fixture identities except for the specified key rename. MUST
   classify the key as `IncrementalVolume(1700000000,1700086400,2)` with
   `partial == true` and `Envelope::Pkcs7`. Link parsed classification to
   `PARSE-NAME-001`.
2. MUST report `DiscoveryStatus::Partial` for the combined partial/unsupported
   object and MUST retain that object as a visible interrupted artifact. Link
   the direct partial-exclusion behavior to `PARSE-NAME-005`. This is one
   observed precedence case, not a universal status-ordering rule.
3. MUST refuse to treat the partial object as the required completed volume 2;
   the public read MUST return exactly `ReaderError::MissingVolume { number: 2 }`
   and no snapshot. Link this fail-closed result to `PARSE-CHAIN-002`.
4. MUST assert the exact six-key open set above and MUST NOT open the combined
   partial/PKCS#7 object. Scope the observation to this fixture and target.
5. MUST NOT claim CMS decoding, cryptographic support, OpenPGP authorization,
   or diagnostic precedence for other artifact classes. Do not link to
   `PARSE-NAME-005` beyond this partial-object exclusion assertion.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the partial-only volume 2 and unsupported-only `.p7m` volume 2 cases.
  The test MUST change only the key and MUST preserve the payload.
- Add Duvet links for exact name fields, direct partial classification and
  exclusion, and exact missing-volume behavior to `PARSE-NAME-001`,
  `PARSE-NAME-005`, and `PARSE-CHAIN-002`. Do not add path or
  crypto-authentication traces.
- Run the focused test, targeted Duvet queries/full trace, focused/full core
  tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the key mutation, preservation of both
  parsed facts, `Partial` status precedence, exclusion from completed restore
  data, exact result, and six-key open set.

## Exclusions

This acceptance covers only the combined partial/PKCS#7 first-incremental
volume 2 case in the official plain fixture. It does not establish general
diagnostic ordering, other combined suffix profiles, other volume ordinals,
CMS or OpenPGP behavior, S3 identity, filesystem restoration, complete
reader/controller refinement, or formal reader proof.

## Checklist

- [x] Independent specification review accepts the combined profile, public
      status precedence, required-volume result, and open set.
- [x] Discovery retains the renamed object with both parsed facts and status
      `Partial`, while volume 1 remains completed.
- [x] The first-incremental read returns exact `MissingVolume { number: 2 }`,
      no snapshot, and opens only the six listed objects.
- [x] Focused/full core tests, targeted Duvet queries/full trace, formatting,
      strict Clippy, and diff checks pass locally.
- [x] Independent implementation/test review passes.
- [ ] Full pinned local `cargo xtask ci` passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- Focused reader acceptance test passed (1/1).
- `cargo test -p duplicity-core` passed.
- Targeted Duvet queries for `PARSE-NAME-001`, `PARSE-NAME-005`, and
  `PARSE-CHAIN-002`, `cargo xtask trace-update`, and `cargo xtask trace` passed.
- `cargo fmt --all -- --check`, strict core Clippy, and `git diff --check`
  passed.
- Independent implementation review passed. Full pinned CI and hosted Linux
  assurance/Pages remain pending.
