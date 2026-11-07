# Reversed-Listing Volume Scan Acceptance

**Status:** Specification and independent implementation review, implementation,
scoped local checks, and full local CI passed. Hosted Linux assurance and Duvet
Pages remain pending.

## Goal and scope

Strengthen the existing official plain reversed-listing reader test to observe
which required data-volume objects the production scan selects when
`ObjectSource::list` returns the catalog in reverse key order. The current test
already preserves that descending catalog order, derives the Full anchor from
the public manifest iterator, reads all three official dates, and checks the
manifest-only open projection. This acceptance adds an exact ordered
manifest-plus-volume open oracle for each date. It uses the existing fixture
and test rather than adding another fixture mutation.

The acceptance covers the native composition of manifest volume records,
ordered catalog scanning, selected object indices, and source opens for this
official plain listing. It does not prove Rust slice/iterator semantics, the
formal volume-scan arithmetic helpers, arbitrary listing permutations, all
reader behavior, or any global resource bound.

## Requirement and production path

Trace the new observable volume-selection claim to
[PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002): the reader must not
silently report a complete snapshot without its required volumes. Use the
existing `Catalog::read_snapshot` path in `crates/core/src/reader.rs`, whose
per-volume scan advances the bounded cursor, examines each `Catalog.objects()`
entry, applies `required_volume_identity_matches`, records the candidate
index, and opens the selected `StoredObject`.

Extend the existing `reader-volume-candidate-scan` milestone in
`docs/assurance.toml` to include this production integration assertion. That
milestone already links `PARSE-CHAIN-002`; the test adds native evidence that
the verified candidate-selection helpers compose with the catalog scan when
the source listing order is reversed. It does not add a proof or close complete
reader refinement.

## Acceptance requirements

1. Reuse the unchanged official plain 3.0.7 fixture in
   `official_reversed_object_order_preserves_manifest_descriptor_projection`.
   Sort the source listing by raw key bytes and reverse it as the test already
   does. Require discovery to perform no opens and require
   `Catalog::objects()` to retain exactly that descending listing.
2. Preserve the existing public manifest projection checks and derive the
   Full anchor from `Catalog::manifests()`. Read targets `1700000000`,
   `1700086400`, and `1700172800` with separate cleared open logs. Each read
   MUST succeed and retain the test's existing snapshot assertions.
3. For target `1700000000`, require the complete source open log to equal, in
   order:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   ```

4. For target `1700086400`, require the complete source open log to equal the
   preceding four keys followed by exactly:

   ```text
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol2.difftar.gz
   ```

5. For target `1700172800`, require the preceding seven keys followed by exactly:

   ```text
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest
   backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz
   ```

   The exact sequence lengths MUST be 4, 7, and 9. No source open may target
   either unselected later-set objects or any of the three official signature
   archives: `backup/duplicity-full-signatures.20231114T221320Z.sigtar.gz`,
   `backup/duplicity-new-signatures.20231114T221320Z.to.20231115T221320Z.sigtar.gz`,
   and `backup/duplicity-new-signatures.20231115T221320Z.to.20231116T221320Z.sigtar.gz`.
   No duplicate open is permitted. This proves only the selected keys and
   observed order for this fixture and descending listing.
6. Add the exact open-log assertion to the existing test's Duvet links for
   `PARSE-CHAIN-002`, and register the test under
   `reader-volume-candidate-scan` in `docs/assurance.toml`. Keep the separate
   `reader-volume-scan-cursor` `PARSE-GEN-005` requirement and claims
   unchanged.

## Verification and boundaries

- Run the focused reversed-listing test and full core tests; query Duvet for
  `PARSE-CHAIN-002`, then run `cargo xtask trace-update` and `cargo xtask
  trace`.
- Run formatting, strict core Clippy, and full `cargo xtask ci`. Require
  independent implementation/test review of the descending source order, the
  three exact open logs, and absence of unselected opens. Require hosted Linux
  assurance and Duvet Pages before acceptance.
- This test does not claim the iterator itself has a formal guarantee, that
  every possible listing permutation works, that omitted objects are always
  detected under adversarial sources, or that the reader is fully refined.
  It makes no filesystem-publication, crypto, live-S3, or global-memory claim.

## Checklist

- [x] Independent specification review accepts the exact 4/7/9 open oracle.
- [x] Strengthen the existing reversed-listing official test without adding a
      new fixture mutation.
- [x] Link the test to `PARSE-CHAIN-002` and the existing
      `reader-volume-candidate-scan` assurance milestone.
- [x] Focused/full core tests, targeted Duvet/full trace, formatting, and
      strict Clippy pass.
- [x] Full local `cargo xtask ci` passed for the implementation tree recorded
      below.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages pass.

## Current evidence

- `cargo test -p duplicity-core --test reader official_reversed_object_order_preserves_manifest_descriptor_projection -- --exact`: passed with exact complete open logs of 4, 7, and 9 fixture keys for the three dates.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check implementation,test --section docs/parsing-requirements.md#parse-chain-002`: passed.
- `cargo xtask trace-update` and `cargo xtask trace`: passed.
- `cargo clippy -p duplicity-core --tests -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check`: passed.
- Full local `cargo xtask ci` passed at source SHA-256 `4e687f5c4a75818d6ef7f924c846eae95b9d1f93f74a4047ba7d906062357cc3`; Verus partitions 2,047/3,183/1,332, zero errors.
- Independent implementation review passed. Hosted Linux assurance/Duvet Pages remain pending.
