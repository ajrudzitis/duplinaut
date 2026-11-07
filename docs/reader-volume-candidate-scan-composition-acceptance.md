# Required-volume candidate scan composition acceptance

Status: implementation, strict focused composition proof, focused reader tests,
Duvet trace, formatting, strict Clippy, and independent implementation review
pass. The candidate-scan milestone is now enforced by the assurance gate;
full `cargo xtask ci` passes on `aarch64-macos` (Verus 2,047/3,183/1,371,
zero errors). Hosted verification remains pending.

## Goal and scope

Close one bounded reader assurance gap by composing the existing per-step
cursor, work-charge, parsed-identity, and candidate-accumulation transitions
for a single required-volume scan. The current production loop in
`Catalog::read_snapshot_inner` performs these steps inline over the catalog
objects for each required manifest volume. The existing
`reader-volume-candidate-scan` assurance milestone has separate
contracts for scalar charge-before-inspection and candidate accumulation, but
does not yet establish their composition.

The acceptance MUST refine one scan's ordered candidate fold and its
fail-closed result. It MUST remain a helper/one-volume-scan proof and MUST NOT
claim complete reader orchestration or Rust iterator refinement.

## Normative requirements

### 1. Shared production transition

The implementation MUST introduce or identify one small production
transition, called by `Catalog::read_snapshot_inner` for each catalog entry,
that composes the existing verified steps in this order:

1. Advance the row-major `VolumeScanCursor` for the current
   `(volume_index, object_index)` pair.
2. Charge exactly one object check using `charge_volume_object`.
3. Evaluate the already-parsed optional `ObjectName` with the existing
   `required_volume_identity_matches` predicate (or treat `None` as no match).
4. Record a unique candidate with `record_volume_candidate`, returning
   `Duplicate` if this is a second matching object.

The production transition MUST preserve the existing error mapping:
invalid cursor/order and exhausted charge return `ReaderError::Limit`; a
second matching candidate returns `ReaderError::DuplicateArtifact`. The
identity predicate MUST run only after the object charge succeeds. The test
MUST call this same production transition; a test-only reimplementation of
the four steps is insufficient.

### 2. Composed contract

The Verus-visible model MUST use the project's `VolumeScanCursor`,
`VolumeScan`, `SetDescriptor`, and `ObjectName` types. For one valid cursor
position and an in-range object index:

- A successful no-match step advances the cursor exactly once and increments
  `checked_objects` exactly once while preserving the prior candidate index.
- A successful match with no previous candidate advances and charges once,
  then records exactly the current object index.
- A matching step when a prior candidate exists returns `Duplicate`; it MUST
  not produce a successful next scan state.
- Invalid cursor coordinates/order or an exhausted work charge fail with
  `Limit` before candidate identity evaluation.

For a sequence of successful ordered steps within one volume row, the proof
MUST show the final check count equals the initial count plus the number of
visited objects. The candidate result MUST be `None` if no visited name
matches, `Some(i)` if exactly one visited name at object index `i` matches,
and `Duplicate` upon the second match. The existing row-major cursor proof
supplies coordinate-order facts; the composition proof MUST state its
preconditions and MUST NOT re-prove or overstate Rust iterator behavior.

### 3. Focused proof and tests

Strict Verus MUST prove the composed transition contract for arbitrary valid
project-type inputs under the cursor/charge preconditions. Native helper tests
MUST independently cover:

- A no-match step preserving `None`, and a no-match step preserving an
  existing candidate.
- A first match at the first, middle, and last object positions.
- A second match at an index later than the first returning `Duplicate`.
- An invalid/skipped/repeated cursor position returning `Limit`.
- An exhausted object-check charge returning `Limit` before identity matching.
- Exact check-count and cursor advancement for every successful step.

Expected states MUST be constructed independently of the production
transition. Tests MUST verify the production helper, not only the four
constituent helpers already under test.

### 4. Public reader adapter evidence

The existing `missing_corrupt_and_ambiguous_objects_fail_explicitly` public
reader test MUST continue to demonstrate exact missing-volume failure without
publishing a snapshot and its separate discovery-time `DuplicateArtifact`
case. That duplicate is rejected by `Catalog::discover` before a reader scan;
it MUST NOT be described as exercising the second-match branch of the
candidate-scan transition. The helper-level composition tests MUST exercise
that second-match branch directly. The official reversed-object-order test
MUST continue to demonstrate its exact 4/7/9 required open logs for the three
dates. Any strengthened source assertion MUST keep catalog order and standard
iterator internals outside the formal claim.

The composed helper behavior is narrow: one required-volume scan records no
candidate, one candidate, or a second-match `Duplicate` according to the
already-parsed identity predicate. The second-match outcome is helper-level
composition evidence; the existing public duplicate-artifact regression is a
separate discovery-time check. This does not establish full snapshot
correctness or all reader error precedence.

## Requirement and assurance mapping

The composed charge/order step MUST map to
[PARSE-GEN-005](parsing-requirements.md#parse-gen-005). The unique/missing
candidate behavior MUST map to
[PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002). Duvet annotations
MUST distinguish these claims and MUST NOT add parser, source-identity,
envelope, or whole-reader requirements to this slice.

Extend the existing `reader-volume-candidate-scan` assurance milestone with
the concrete production transition's Verus contract and focused tests. Retain
the existing `reader-volume-work-reservation` mapping and row-major
`reader-volume-scan-cursor` milestone without expanding either claim. The
native public tests remain mapped to the reader candidate-scan milestone.

## Exclusions

This acceptance does not prove object-source enumeration completeness or
order, Rust slice/iterator internals, candidate identity parsing, manifest
normalization, reservation arithmetic beyond existing contracts, cumulative
multi-set budget composition, cancellation timing, source identity, digest or
crypto behavior, multipart assembly, snapshot reconstruction, allocator
behavior, or complete reader/resource-accounting refinement. The row-major
cursor, identity predicate, work reservation, and publication gates remain
their existing separate proofs.

## Acceptance evidence

- [x] Independent specification review accepts the one-scan composition
      boundary and its trust limits.
- [x] Production calls the shared transition in cursor, charge, identity,
      candidate-record order.
- [x] Strict Verus proves the composed per-step contract and ordered-fold
      candidate/check-count property.
- [x] Focused tests cover no match, one match at each position, duplicate,
      invalid cursor, charge exhaustion, and exact state advancement.
- [x] Existing public missing-volume, discovery-time duplicate-artifact, and
      reversed-order reader tests pass; the helper-level test exercises a
      second matching candidate. Duvet links the helper/tests to
      PARSE-GEN-005 and PARSE-CHAIN-002 only.
- [x] Assurance mapping extends `reader-volume-candidate-scan`; the
      row-major cursor and work-reservation mappings remain unchanged.
- [x] Targeted Duvet queries, trace update/full trace, formatting, strict
      Clippy, focused helper tests, and public reader tests pass.
- [x] Full `cargo xtask ci` passes on `aarch64-macos`, including all enabled
      Duvet, Verus, workspace-test, and assurance-mapped-test gates. The
      receipt records source SHA-256
      `22bc1734a7a6ce0c875945d498bf24ca702e9a026f4bb7032cdc8eeed73e6986`.
- [x] Independent implementation/proof review accepts the helper-to-callsite
      composition and scoped native evidence.
- [x] The `reader-volume-candidate-scan` milestone is enabled as a scoped
      assurance gate with the reviewed proof and test mappings.
- [ ] Hosted Linux assurance and Duvet Pages pass.

## Local implementation evidence

- Strict pinned Verus module verification passed: `cargo verus focus -p duplicity-core --target-dir target/verify/reader-volume-composition-final -- --no-cheating --verify-module verified::reader_volume_scan` (51 verified, 0 errors).
- `cargo test -p duplicity-core --lib verified::reader_volume_scan::tests::composed_volume_scan_step_covers_order_charge_and_candidate_outcomes` passed 1/1.
- `cargo test -p duplicity-core --test reader` passed all 39 reader integration tests, including the discovery-time duplicate, missing-volume, and reversed-order cases.
- Targeted `PARSE-GEN-005` and `PARSE-CHAIN-002` implementation/test queries passed; `cargo xtask trace-update` and `cargo xtask trace` passed.
- `cargo clippy -p duplicity-core --tests -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` passed.
- Full `nix-shell tools/shell.nix --run 'source tools/env.sh && cargo xtask ci'` passed with fresh isolated Verus verification (2,047/3,183/1,371, zero errors), workspace tests, and gated assurance-test replay. Receipt: `target/assurance/ci.json` (source SHA-256 above; `aarch64-macos`).
