# Reader Selected-Set Boundary Cancellation Acceptance

**Status:** Accepted. Implementation, independent review, full local CI,
hosted Linux assurance, and Duvet Pages pass in
[run 42370397916](https://github.com/ajrudzitis/duplinaut/actions/runs/42370397916).

## Goal and scope

Add one deterministic integration case for cancellation at a selected-set
boundary in the public `Catalog::read_snapshot` path. It MUST cancel when the
reader requests the first incremental manifest, after the official Full set
has been read and applied. The public result MUST be
`ReaderError::Source(SourceError::Cancelled)`, with no snapshot returned and
no later incremental manifest or data volume opened.

This is a narrow cancellation-observation case for the synchronous
`ObjectSource::open` handoff. It does not establish a cancellation latency,
interrupt blocking I/O, hash/decompression/crypto interruption, thread-race
semantics, or complete reader-controller refinement. No filesystem operation
or external service is involved.

## Requirement and production path

Trace the test to
[PARSE-ENV-024](parsing-requirements.md#parse-env-024), scoped to its
requirement that cancellation be observed per required object/set and that a
cancellation failure return neither a completed snapshot nor successful
partial-chain evidence. Do not claim the test proves the complete ordering of
all cancellation checkpoints or a fixed response-time bound.

The relevant production path is `Catalog::read_snapshot` through
`read_snapshot_inner` and `read_stored` in `crates/core/src/reader.rs`. The
reader processes the selected Full set, reads its normalized required
volumes, finishes the archive, applies it to state, and then starts the first
incremental set. `read_stored` calls `ObjectSource::open`, constructs a
`CheckedReader`, and observes cancellation before consuming bytes. The test
MUST use these production functions without a test-only reader implementation
or cancellation-specific production hook.

## Acceptance requirements

1. The test MUST use the unchanged official plain 3.0.7 fixture and its
   existing local conditional-read source. Discovery MUST complete before the
   cancellation token is created or signaled, so the test isolates snapshot
   reading rather than discovery cancellation.
2. A test-local `ObjectSource` wrapper or equivalent deterministic source
   MUST log each opened key. When `open` is called for exactly
   `backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest`, it
   MUST record that trigger, cancel the exact token passed to
   `read_snapshot`, and return the fixture bytes without sleeping or spawning
   a thread. Other object opens MUST delegate unchanged to the official
   source.
3. Call public `read_snapshot` with the iterator-derived Full anchor and final
   target `Timestamp(1700172800)`. Require exactly
   `Err(ReaderError::Source(SourceError::Cancelled))`; the API MUST return no
   `ValidatedSnapshot` or successful partial-chain result.
4. Require the trigger manifest to be opened only after the Full manifest and
   all three completed Full data volumes have been opened. Require the
   complete opened-key set to contain exactly those five keys: the Full
   manifest, Full volumes 1–3, and the first-incremental manifest that
   triggers cancellation. No first-incremental volume, final-incremental
   manifest, or final-incremental volume may be opened after cancellation.
   Discovery/listing operations are not counted as object opens.
5. The test MUST assert that the exact token observed by the source is
   cancelled when `read_snapshot` returns. It MUST NOT rely on sleeps,
   scheduler timing, real network I/O, or cancellation during a blocking
   operation.
6. Add a Duvet test link to `PARSE-ENV-024` whose annotation describes this
   precise source-handoff cancellation result. Keep existing source identity,
   chain selection, and snapshot-state links separate unless the new test adds
   distinct assertions for them.

## Verification and boundaries

- Run the focused cancellation integration test and the full core test suite.
- Run the targeted `PARSE-ENV-024` Duvet query, `cargo xtask trace-update`,
  and `cargo xtask trace`; check formatting, strict core Clippy, and full
  `cargo xtask ci`.
- Require independent implementation/test review of the exact trigger key,
  observed token, no-snapshot error, and five-key open set. Hosted Linux
  assurance and Duvet Pages must pass before accepting the milestone.
- This case does not establish cancellation before discovery, at every
  manifest/volume boundary, during native crypto, during blocking source
  reads, or in a concurrent race. It does not prove iterator semantics,
  object matching correctness, resource accounting, complete chain
  completeness, or whole-reader formal refinement.

## Checklist

- [x] Independent specification review accepts the selected-set boundary
      trigger and exact open-set oracle.
- [x] Add the deterministic official-fixture test without production hooks.
- [x] Focused/full core tests, targeted Duvet query/full trace, formatting,
      strict Clippy, and diff checks pass.
- [x] Full local `cargo xtask ci` passes on the final tracked tree; receipt is recorded in generated `target/assurance/ci.json`.
- [x] Independent implementation/test review passes.
- [x] Hosted Linux assurance and Duvet Pages pass in
      [run 42370397916](https://github.com/ajrudzitis/duplinaut/actions/runs/42370397916).

## Current evidence

- `cargo test -p duplicity-core --test reader official_reader_cancellation_at_first_incremental_manifest_open_returns_no_snapshot -- --exact`: passed.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check test --section docs/parsing-requirements.md#parse-env-024`: passed.
- `cargo xtask trace-update` and `cargo xtask trace`: passed.
- `cargo fmt --all --check`, strict core Clippy, and `git diff --check`: passed.
- Independent implementation review and full local `cargo xtask ci` passed;
  the generated receipt is in `target/assurance/ci.json`. Hosted Linux
  assurance and Duvet Pages passed in
  [run 42370397916](https://github.com/ajrudzitis/duplinaut/actions/runs/42370397916).
