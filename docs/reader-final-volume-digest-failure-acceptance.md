# Final Incremental Volume Digest Failure Acceptance

**Status:** Independently reviewed and accepted as a specification;
implementation and scoped local checks pass; independent implementation
review, full local CI, and hosted gates pending.

## Goal and scope

Add one late stored-object integrity failure case to the official plain public
reader test set. Mutate only the in-memory bytes of final-Incremental volume 1,
update that test object's local source identity to match the mutated body, and
leave the manifest's recorded digest unchanged. After the Full and first
Incremental sets, the final Incremental manifest, and the final data-volume
open all succeed, the reader must report the stored-volume digest mismatch and
return no snapshot.

This tests the integrity check on the last required volume after earlier
selected sets have been reconstructed. It does not prove SHA1/MD5 primitives,
manifest authenticity, source identity comparison, or complete reader
refinement. No fixture file, S3 object, or external backup is changed.

## Requirement and production path

Trace the test to
[PARSE-ENV-005](parsing-requirements.md#parse-env-005): manifest volume digests
must be checked against the original stored object bytes before a restoration
using those volumes is published. Exercise the production
`Catalog::read_snapshot` path and require the digest-specific typed error
before any final snapshot is returned.

Register a scoped `reader-final-volume-digest-failure-observation` milestone
in `docs/assurance.toml`, linked to `PARSE-ENV-005` and the new native test.
This is a tested observation of one late failure path, not a formal proof of
the digest algorithm or complete PARSE-ENV-005 behavior. Keep the trusted
boundary for native digest primitives explicit.

## Acceptance requirements

1. Use the unchanged official plain 3.0.7 fixture and preserve its twelve
   object keys and count. Target exactly
   `backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz`.
2. Clone the original test-local inventory before mutation. Flip exactly byte
   zero of only the final Incremental volume's stored body, preserve its
   length, and update only that `StoredObject` identity to a distinct test
   version ID and an ETag computed from the mutated bytes. Keep every other
   key, size, identity, and body byte-for-byte equal to the original fixture.
   Do not alter either final-manifest digest declaration.
3. Complete metadata-only discovery before the read; require zero discovery
   opens. Derive the Full anchor from `Catalog::manifests()` and read target
   `Timestamp(1700172800)`. The catalog must retain the same three completed
   manifest descriptors and intervals as the official fixture.
4. Require exactly
   `Err(ReaderError::Assembly(AssemblyError::Envelope(EnvelopeError::DigestMismatch)))`.
   The cancellation token must remain clear; no `ValidatedSnapshot` may be
   returned.
5. Require the ordered source open log to equal exactly these nine keys:

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

   The final key is the mutated object and its unchanged manifest digest is the
   failure trigger. No signature archive, later object, or duplicate open is
   permitted.
6. Link the test to `PARSE-ENV-005` in Duvet and register it under the new
   scoped assurance milestone. The milestone must say this is one late
   fixture-backed failure observation; it does not establish native hash
   primitive correctness or complete volume-digest enforcement.

## Verification and boundaries

- Run the focused final-volume digest test and full core tests; run the
  targeted `PARSE-ENV-005` Duvet query, `cargo xtask trace-update`, and
  `cargo xtask trace`.
- Run formatting, strict core Clippy, and full `cargo xtask ci`. Require
  independent implementation/test review of the single-byte mutation,
  identity refresh, unchanged manifest digest, typed mismatch, and exact
  nine-key order. Require hosted Linux assurance and Duvet Pages before
  acceptance.
- The mutation is test-local and changes only one in-memory body and its source
  identity. No upstream fixture regeneration or external service is involved.
  The source identity is made consistent with the changed body, so this case
  does not test `PARSE-SOURCE-001`. The test does not claim hash primitive
  correctness, signature/authenticity, failure behavior at other volumes,
  cancellation races, filesystem publication, or whole-reader refinement.

## Current evidence

- The focused `official_final_incremental_volume_digest_mismatch_returns_no_snapshot`
  test and full `cargo test -p duplicity-core` pass.
- The targeted `PARSE-ENV-005` implementation/test query, `cargo xtask
  trace-update`, and full `cargo xtask trace` pass.
- `cargo clippy -p duplicity-core --tests -- -D warnings`, `cargo fmt --all
  -- --check`, and `git diff --check` pass. Clippy reports pre-existing
  warnings in vendored `vest_lib`; the core test target is clean.
- Independent implementation review passed. The reviewer confirmed the
  mutation is limited to final Incremental vol1's byte zero and test-local
  identity, the manifest digest remains unchanged, discovery performs zero
  opens, the exact typed mismatch and clear token are asserted, and the
  ordered nine-key read log ends at the mutated volume. The scoped
  PARSE-ENV-005 mapping and native-digest trust boundary are appropriate.
- Full `nix-shell tools/shell.nix --run 'source tools/env.sh && cargo xtask ci'`
  passes at source SHA-256
  `a2cadb1dba44d7423fdcb410d99d9c851ef7ae4b90970fc0fe6195bc84e8177c` on
  `aarch64-macos`. Verus partitions report 2,047, 3,183, and 1,332 verified
  obligations with zero errors; workspace formatting, strict Clippy, tests,
  and assurance-mapped test validation pass. Receipt: generated
  `target/assurance/ci.json`.
- Hosted Linux assurance and Duvet Pages remain pending.

## Checklist

- [x] Independent specification review accepts the mutation and nine-key
      failure oracle.
- [x] Add a native test that mutates only final Incremental volume 1 and
      updates only its test-local identity.
- [x] Map the test to `PARSE-ENV-005` and a separate scoped assurance
      milestone; retain the trusted native-digest boundary.
- [x] Focused/full core tests, targeted Duvet/full trace, formatting, and
      strict Clippy pass.
- [x] Full local `cargo xtask ci` passes.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages pass.
