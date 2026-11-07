# Late Data-Volume Source-Identity Failure Acceptance

**Status:** Independently reviewed and accepted as a specification;
implementation, tests, and verification pending.

## Goal and scope

Add a distinct public-reader failure case where the source reports a
conditional-read identity change on the final Incremental data volume. The
official Full set, first Incremental set, final Incremental manifest, and
volume selection must all succeed before the final volume open reports
`SourceError::Changed`. The public read must fail without returning a
`ValidatedSnapshot`.

The test source simulates a provider rejecting the conditional read for this
one object. It verifies typed error propagation and fail-closed behavior after
the earlier chain work. It does not prove that the fake source independently
detects identity changes, verify backend conditional-read implementation, or
prove atomic publication for every failure location.

## Requirement and production path

Trace this public boundary to
[PARSE-SOURCE-001](parsing-requirements.md#parse-source-001): source identity
changes between discovery and use must be detected whenever the source offers
conditional-read metadata. The source reports `Changed` for the final data
volume; `Catalog::read_snapshot` propagates that error and returns no
snapshot.

Use the production `Catalog::read_snapshot` path and the existing
`reader-orchestration` assurance milestone. Add a distinct native test to that
milestone. No formal proof is required: the scope is a source callback/error
observation, not a proof of source identity comparison or whole-reader
refinement.

## Acceptance requirements

1. Use the unchanged official plain 3.0.7 fixture. Complete discovery before
   the read, derive the Full anchor from `Catalog::manifests()`, and request
   `Timestamp(1700172800)`.
2. Add a test-local source mode that returns the unchanged discovered object
   metadata for every object but returns `SourceError::Changed` only when
   opening exactly
   `backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gz`.
   It MUST NOT fail on the final manifest or any earlier object. Do not alter
   fixture bytes or inventory.
3. Require exactly
   `Err(ReaderError::Source(SourceError::Changed))`, a clear supplied
   cancellation token, and no `ValidatedSnapshot`.
4. Require the ordered object-open log to equal exactly these nine keys:

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

   The final key MUST be the only open that returns `Changed`. No signature
   archive or other object may be opened. This records that the preceding
   chain sets and final manifest were read before the final required-volume
   source failure.
5. Map the test to `PARSE-SOURCE-001` and register it under
   `reader-orchestration` in `docs/assurance.toml`. Extend that milestone's
   rationale to describe both separately tested late conditional-source
   failures: the existing final-manifest open and this final-volume open after
   its manifest succeeds. Keep the claim limited to those two source-reported
   observations; do not claim backend identity checking, retry behavior, or
   general reader refinement.

## Verification and boundaries

- Run the focused test and full core tests; run the targeted
  `PARSE-SOURCE-001` Duvet query, `cargo xtask trace-update`, and
  `cargo xtask trace`.
- Run formatting, strict core Clippy, and full `cargo xtask ci`. Require
  independent implementation/test review of the exact trigger, error, clear
  token, and nine-key order. Require hosted Linux assurance and Duvet Pages
  before acceptance.
- This test does not establish backend identity detection, detect changes
  during a body read, test cancellation or I/O races, alter fixture bytes,
  cover other objects or failure positions, validate crypto, perform
  filesystem publication, or prove complete reader-controller refinement.

## Checklist

- [x] Independent specification review accepts the trigger and nine-key log.
- [ ] Add the official-fixture source-change test with an exact trigger and no
      production changes.
- [ ] Map the test to `PARSE-SOURCE-001` and `reader-orchestration`; extend
      the assurance rationale for both separate final-manifest and
      final-volume-open observations without claiming backend identity proof
      or complete reader refinement.
- [ ] Focused/full core tests, targeted Duvet/full trace, formatting, strict
      Clippy, and full local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages pass.
