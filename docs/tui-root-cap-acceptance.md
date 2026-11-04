# TUI selected-root byte-cap acceptance plan

Status: proposed; specification only. This is a bounded test of selected-root
admission and does not change backlog status.

## Goal and scope

Test the byte-cap check applied before the TUI clones selected raw-byte roots
for a restore preview. Keep the production `ROOT_BYTES` limit fixed at 4 MiB;
use an internal test-local limit derived from a small official-fixture root set
to exercise the exact boundary without large allocations.

This acceptance traces to
[RESTORE-UI-011](parsing-requirements.md#restore-ui-011), which requires bounds
on owned restoration metadata before allocation. If the test compares the
returned original component-byte sequences, also link it to
[RESTORE-UI-002](parsing-requirements.md#restore-ui-002). It does not establish
a global allocator or process-memory bound.

## Acceptance requirements

1. Preserve the production `copy_roots(snapshot, marked)` behavior and fixed
   `ROOT_BYTES == 4 * 1024 * 1024` cap. Factor a private helper that accepts a
   byte limit for deterministic unit testing; do not expose a runtime or
   user-configurable limit.
2. Build a snapshot from the checked-in official duplicity 3.0.7 plain
   fixture. Select one or more valid roots using their actual entry indices,
   including a root with a non-UTF-8 component (the fixture contains a raw
   `0xff` component). Do not modify producer data or convert displayed escapes
   back into root identity.
3. Independently compute the selected-root charge for the test case as the
   owned root-vector element storage (`root_count * size_of::<BackupPath>()`)
   plus, for each selected path, its outer component-vector allocation
   (`path.components().capacity() * size_of::<Vec<u8>>()`) and the capacity of
   every inner component byte vector (`component.capacity()`). Set the helper's
   test-local cap to this exact charge and require success with the same root
   count and exact raw component-byte sequences as the selected snapshot
   paths.
4. Rerun the identical selection with a test-local cap one byte below the
   computed charge and require exactly `Err(Error::Limit)`. The call returns
   no root vector, so no partial selection can proceed to planning. Do not
   assert allocator-call counts or infer a process-wide allocation bound.
5. Require the production wrapper to use `ROOT_BYTES`, and require its Duvet
   implementation/test links to RESTORE-UI-011. Link the test to
   RESTORE-UI-002 only for asserted preservation of the selected raw-byte root
   identities.

## Verification and evidence

- Run the focused selected-root byte-cap test and `cargo test -p
  duplicity-tui` locally. The test is portable and does not require Linux
  filesystem publication, PTYs, AWS credentials, or network access.
- Run targeted Duvet queries for RESTORE-UI-011 and, if used, RESTORE-UI-002;
  then run `cargo xtask trace-update` and `cargo xtask trace`.
- Run formatting and strict TUI Clippy locally. Run the hosted Linux assurance
  workflow (`cargo xtask ci`) and require Duvet report publication before
  acceptance.
- Independent review must confirm the production cap is unchanged, the test
  charge accounts for root-vector element storage, each selected path's outer
  component-vector capacity, and every inner component byte-vector capacity;
  boundary results are exact, root identities are checked as raw components,
  and no allocation or global-memory claim is made.

## Exclusions

This acceptance does not test the separate root-count or candidate-comparison
limits, complete `RestorePlan` traversal, preview text or diagnostic caps,
terminal interaction, cancellation, filesystem publication, S3, cryptographic
behavior, allocator internals, or a global heap bound. It does not claim that
the caller's marks are an authenticated or durable selection record.

## Checklist

- [ ] Add the private test-local cap seam while retaining the fixed 4 MiB
      production limit.
- [ ] Add exact-cap and cap-minus-one tests over official plain-snapshot roots;
      link the test and implementation to the applicable Duvet requirements.
- [ ] Focused/full TUI tests, Duvet trace, formatting, and strict Clippy pass
      locally; hosted Linux assurance and Duvet Pages pass.
- [ ] Independent review accepts the test and its limited claim.
