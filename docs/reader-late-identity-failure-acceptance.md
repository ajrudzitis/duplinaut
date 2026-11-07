# Late Reader Source-Identity Failure Acceptance

**Status:** Proposed; independent specification review pending.

## Goal and scope

Strengthen the existing official plain reader failure test for a conditional
source change at the final Incremental manifest. The test source already
returns `SourceError::Changed` for that key, after the Full set and first
Incremental set have been read and reconstructed successfully. This acceptance
requires an exact source-open trace to show where the late failure occurs and
that no incomplete final chain is returned.

The test simulates the `ObjectSource` reporting that its conditional read no
longer matches the discovered identity. It verifies public error propagation
and fail-closed snapshot publication. It does not establish that the fake
source independently detects an identity change, prove backend conditional
read behavior, or prove publication atomicity for every failure point.

## Requirement and production path

Trace the public boundary to
[PARSE-SOURCE-001](parsing-requirements.md#parse-source-001): identity changes
between discovery and use must be detected when the source provides
conditional-read metadata. The source contract reports `Changed` from the
final manifest open; `Catalog::read_snapshot` must propagate that typed source
error and return no `ValidatedSnapshot`.

Use the existing test-local official `Source` mode that returns
`SourceError::Changed` when the final Incremental manifest is opened. Exercise
the production `Catalog::read_snapshot` path and the existing
`reader-orchestration` assurance milestone. Extend its rationale to identify
this as a late conditional-source failure after successful preceding sets.
This adds native integration evidence only; no formal proof is claimed.

## Acceptance requirements

1. Use the unchanged official plain 3.0.7 fixture and complete discovery
   before the read. Derive the Full anchor from `Catalog::manifests()` and
   request `Timestamp(1700172800)`.
2. Narrow the existing test-local mode-5 trigger to match exactly
   `backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest`; it
   MUST NOT trigger on other final-set keys. Return the unchanged stored
   object identity from discovery, and have the source return
   `SourceError::Changed` at this final manifest open, as if its conditional
   read failed. Do not change production code, fixture bytes, or source
   inventory.
3. Require exactly
   `Err(ReaderError::Source(SourceError::Changed))`. The supplied cancellation
   token MUST remain clear, and no `ValidatedSnapshot` may be returned.
4. Require the ordered open log to equal exactly these eight keys:

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

   The final key MUST be the only open that returns `Changed`. The log proves
   the previous two sets' required opens completed before the final-set
   failure. The final Incremental volume, all signature archives, and other
   unselected objects MUST NOT be opened.
5. Keep the test mapped to `PARSE-SOURCE-001` and registered under
   `reader-orchestration` in `docs/assurance.toml`. The observation covers
   propagation of one late source-reported conditional failure only; do not
   claim a formal identity-comparison proof or general reader refinement.

## Verification and boundaries

- Run the focused `identity_cancellation_and_exact_body_failures_publish_no_snapshot`
  test and full core tests; run the targeted `PARSE-SOURCE-001` Duvet query,
  `cargo xtask trace-update`, and `cargo xtask trace`.
- Run formatting, strict core Clippy, and full `cargo xtask ci`. Require
  independent implementation/test review of the exact trigger, typed error,
  clear cancellation token, and eight-key trace. Require hosted Linux
  assurance and Duvet Pages before acceptance.
- This case does not test backend identity comparison, retries, concurrent
  change races, cancellation, an altered manifest body, data-volume changes,
  arbitrary failure positions, cryptographic validation, filesystem
  publication, or complete reader-controller refinement.

## Checklist

- [ ] Independent specification review accepts the late-trigger/error scope.
- [ ] Strengthen the existing mode-5 official reader test with the exact
      eight-key log and clear-token assertion, and narrow its source failure
      trigger to the exact final-manifest key; make no fixture mutation or
      production change.
- [ ] Map the case to `PARSE-SOURCE-001` and the existing
      `reader-orchestration` assurance milestone.
- [ ] Focused/full core tests, targeted Duvet/full trace, formatting, strict
      Clippy, and full local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages pass.
