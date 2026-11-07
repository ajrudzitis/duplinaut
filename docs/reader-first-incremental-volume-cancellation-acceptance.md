# Reader First Incremental Volume Cancellation Acceptance

**Status:** Specification and independent implementation review,
implementation, focused/full core tests, scoped Duvet/format/Clippy checks, and
full local CI passed. Hosted Linux assurance and Duvet Pages passed in
[run 42376292497](https://github.com/ajrudzitis/duplinaut/actions/runs/42376292497).

## Goal and scope

Add one deterministic public-reader cancellation case at the first data-volume
open of the first Incremental set. The official Full set and the first
Incremental manifest MUST have been read successfully. A test-local source
MUST cancel the exact token supplied to `Catalog::read_snapshot` when the
reader requests first-incremental volume 1. The public call MUST return
`ReaderError::Source(SourceError::Cancelled)`, without a snapshot or any
later object open.

This adds an adjacent but distinct checkpoint from cancellation during the
first-incremental manifest open. It verifies one synchronous
`ObjectSource::open` handoff after manifest validation. It does not establish
blocking-I/O interruption, cancellation races, fixed cancellation latency, or
complete reader-controller refinement. No filesystem operation or external
service is involved.

## Requirement and production path

Trace the acceptance to
[PARSE-ENV-024](parsing-requirements.md#parse-env-024), scoped to observing
cancellation per required object/set and withholding the snapshot after a
cancellation failure. Do not claim a proof of every cancellation checkpoint
or cancellation ordering under concurrent races.

Use the production `Catalog::read_snapshot` → `read_snapshot_inner` →
`read_stored` path in `crates/core/src/reader.rs`. On the official first
incremental manifest, the reader parses the manifest, creates the assembler,
scans for the first required volume, and then calls `read_stored` for that
volume. `read_stored` calls `ObjectSource::open` and constructs a
`CheckedReader`, which observes the canceled token before consuming returned
bytes. No test-only reader or production cancellation hook is permitted.

## Acceptance requirements

1. Use the unchanged official plain 3.0.7 fixture. Complete discovery before
   creating or canceling the token used by `read_snapshot`; assert discovery
   performed no object opens. Derive the Full anchor from the public manifest
   iterator and request `Timestamp(1700172800)`.
2. A deterministic test-local source MUST log each object open. When it sees
   exactly
   `backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz`,
   it MUST assert the token is still clear, record the trigger, cancel that
   exact token, and return the fixture bytes. Other opens MUST return their
   unchanged official bytes and identities. The test MUST use no sleeps,
   threads, network requests, or blocking-read cancellation race.
3. Require exactly
   `Err(ReaderError::Source(SourceError::Cancelled))` and assert the same
   token is canceled when `read_snapshot` returns. No
   `ValidatedSnapshot` or partial-chain result may be returned.
4. Require the ordered object-open log to equal exactly these six keys:

   ```text
   backup/duplicity-full.20231114T221320Z.manifest
   backup/duplicity-full.20231114T221320Z.vol1.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
   backup/duplicity-full.20231114T221320Z.vol3.difftar.gz
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
   backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
   ```

   The final key is the cancellation trigger. No first-incremental volume 2,
   final-incremental manifest, or final-incremental data volume may be opened.
   Discovery/listing operations are not included in this log.
5. The point at which volume 1 is requested MUST be the observable evidence
   that the first-incremental manifest was read and accepted far enough to
   select its normalized required volume. Do not claim additional validation
   stages that the test cannot observe.
6. Extend the existing
   `reader-set-boundary-cancellation-observation` assurance milestone in
   `docs/assurance.toml` to cover two separately tested synchronous source-open
   handoffs: the existing first-incremental manifest trigger and this new
   first-incremental volume 1 trigger after manifest processing. Its rationale
   MUST describe both checkpoints, and its test mapping MUST retain the
   manifest-open test as a separate case while adding this volume-open test.
   Both tests map to `PARSE-ENV-024`. No new formal proof is required: these
   are native observations of distinct source callbacks and production
   cancellation results. This milestone does not claim broader controller
   refinement.

## Verification and boundaries

- Run the focused test and full core suite; run targeted
  `PARSE-ENV-024` Duvet query, `cargo xtask trace-update`, and `cargo xtask
  trace`; check formatting, strict core Clippy, and full `cargo xtask ci`.
- Require independent implementation/test review of the exact trigger key,
  clear-before-trigger observation, exact token, error, and six-key ordered
  log. Require hosted Linux assurance and Duvet Pages before acceptance.
- This case establishes only cancellation when first-incremental volume 1 is
  synchronously requested after its manifest. It does not prove interruption
  during a body read, cancellation during parsing/decompression/crypto,
  cross-thread race behavior, a response-time bound, other object boundaries,
  iterator semantics, resource accounting, or whole-reader refinement.

## Checklist

- [x] Independent specification review accepts the trigger and six-key order.
- [x] Add the deterministic official-fixture test without a production hook.
- [x] Extend the existing assurance milestone rationale and test mapping to
      cover the separate manifest-open and first-incremental volume 1
      handoffs; map both to `PARSE-ENV-024` without claiming broader controller
      proof.
- [x] Focused/full core tests, targeted Duvet/full trace, formatting, and
      strict Clippy pass.
- [x] Full local `cargo xtask ci` passed for the implementation tree recorded
      below.
- [x] Independent implementation/test review passes.
- [x] Hosted Linux assurance and Duvet Pages pass in [run 42376292497](https://github.com/ajrudzitis/duplinaut/actions/runs/42376292497).

## Current evidence

- `cargo test -p duplicity-core --test reader official_reader_cancellation_at_first_incremental_volume_open_returns_no_snapshot -- --exact`: passed; exact trigger, typed cancellation, and ordered six-key open log asserted.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check implementation,test --section docs/parsing-requirements.md#parse-env-024`: passed.
- `cargo xtask trace-update` and `cargo xtask trace`: passed.
- `cargo clippy -p duplicity-core --tests -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check`: passed.
- Full local `cargo xtask ci`: passed at source SHA-256 `e96efe562da72a8b7de132a3d9a51038ae92f0441227101454b222ed2d83ecb6`; Verus partitions 2,047/3,183/1,332, zero errors.
- Independent implementation/test review passed. Hosted Linux assurance and Duvet Pages passed in [run 42376292497](https://github.com/ajrudzitis/duplinaut/actions/runs/42376292497).
