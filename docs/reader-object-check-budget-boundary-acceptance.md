# Public object-check budget boundary acceptance

Status: specification accepted after independent review. Implementation,
tests, trace, and hosted verification remain pending.

## Goal and scope

Make the public reader's required-volume object-check boundary observable with
an exact typed error and open log. Strengthen
`reader_budget_edges_accept_exact_and_reject_one_less` in
`crates/core/tests/reader.rs`, which already uses the official plain 3.0.7
fixture and is registered in the `reader-volume-work-reservation` assurance
milestone.

This acceptance covers only preflight reservation for the Full set's required
volume scan. It does not add a proof or claim complete reader resource
accounting.

## Normative requirements

### 1. Establish the exact work bound

The test MUST assert that the unchanged official plain source contains exactly
12 catalog objects and three completed Full data volumes. It MUST set
`max_object_checks` to the resulting exact work `12 * 3 = 36` for the accepted
case and `35` for the one-less case. The values MUST be explicit in the test
oracle; fixture drift MUST NOT silently change the intended boundary.

### 2. Exact-bound success

With `max_object_checks = 36`, discovery MUST remain metadata-only and the
public read of the first official snapshot at timestamp `1700000000` MUST
succeed. The ordered source-open log MUST contain exactly these four keys:

1. `backup/duplicity-full.20231114T221320Z.manifest`
2. `backup/duplicity-full.20231114T221320Z.vol1.difftar.gz`
3. `backup/duplicity-full.20231114T221320Z.vol2.difftar.gz`
4. `backup/duplicity-full.20231114T221320Z.vol3.difftar.gz`

The test MUST clear and inspect the source open log around this individual
read, and MUST require that discovery itself opened no object.

### 3. One-less preflight rejection

With `max_object_checks = 35`, discovery MUST still succeed without opening
objects. The read at timestamp `1700000000` MUST return exactly
`ReaderError::Limit`; it MUST NOT accept an `Envelope` error as an alternative.
The ordered source-open log MUST contain only
`backup/duplicity-full.20231114T221320Z.manifest`. In particular, no Full
volume may be opened: `reserve_volume_scan` MUST reject the required work
before entering the object-candidate scan.

This assertion demonstrates the public call path's failure point and open
prefix. It does not prove Rust iteration semantics or every reader budget.

### 4. Keep the other budget cases separate

The decoded-manifest-byte and normalization-metadata exact/one-less cases in
the existing test MUST remain as separate cases with independent catalogs and
open-log resets. They MUST NOT be merged into a shared assertion that permits
either an object-check `Limit` or an `Envelope` error for every bound. Their
distinct existing behavior MUST remain explicit: a one-byte-short decoded
manifest limit returns `ReaderError::Envelope(EnvelopeError::DecodedLimit)`;
a one-byte-short normalization limit returns `ReaderError::Limit` before any
volume is opened. This plan does not change the configured limits or those
cases' boundary values.

## Requirement and assurance mapping

The test MUST retain a Duvet link to
[PARSE-GEN-005](parsing-requirements.md#parse-gen-005), which requires
configured bounds to be enforced. Register the test under the existing
`reader-volume-work-reservation` assurance milestone and retain its current
test name. The production reservation path is
`Catalog::read_snapshot_inner` calling `reserve_volume_scan`; existing Verus
contracts cover checked reservation arithmetic and per-object charge
transitions. This acceptance adds native public-path evidence for the exact
36/35 outcome and preflight open prefix; it does not extend those formal
contracts.

The `reader-volume-candidate-scan` milestone and its existing tests remain
unchanged. The failure case returns before candidate enumeration, and this
test MUST NOT be described as a proof of candidate uniqueness or full scan
composition.

## Exclusions

This acceptance does not change reader code, limits, or proof models. It does
not cover decoded-manifest or normalization accounting beyond preserving
their separate current cases, other resource dimensions, cumulative work
across multiple selected sets, cancellation, allocation behavior, source
identity, S3, cryptographic behavior, or complete repository-reader
refinement.

## Acceptance evidence

- [x] Independent specification review accepts the 36/35 oracle and scope.
- [ ] The focused reader test asserts exact `Limit` at 35 and the one-key open
      prefix; exact 36 succeeds with the ordered four-key Full open log.
- [ ] Separate decoded-manifest and normalization exact/one-less cases remain
      present and assert their distinct typed failures.
- [ ] Targeted Duvet `PARSE-GEN-005` implementation/test queries, trace update,
      and full trace pass.
- [ ] Formatting, strict Clippy, focused/full core tests, and full
      `cargo xtask ci` pass.
- [ ] Independent implementation review confirms the preflight happens after
      the Full manifest is read but before any volume is opened.
- [ ] Hosted Linux assurance and Duvet Pages pass.
