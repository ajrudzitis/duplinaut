# Selected chain-set adapter acceptance plan

Status: specification accepted after independent review; implementation is
pending.

## Goal and scope

Specify and validate the narrow adapter that exposes a validated `ChainPlan`
as its selected descriptors in chain order. The production adapter is
`ChainPlan::sets()` in `crates/core/src/chain.rs`; today it maps the plan's
private index vector over the borrowed original descriptor slice. The
acceptance connects the controller's verified selected indices to that
observable adapter result.

The plan is limited to plans successfully constructed by `ChainPlan::new` and
left immutable through the public API. It does not change which chain is
selected, reinterpret timestamps, or alter missing/ambiguous-chain behavior.

## Requirements and model

Trace this slice to
[PARSE-CHAIN-001](parsing-requirements.md#parse-chain-001), requiring the
selected chain to retain full-then-incremental order, and
[PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002), requiring that
missing or ambiguous metadata is not treated as a complete chain. The latter
applies only to the already-constructed plan's selected descriptor sequence;
this acceptance does not independently prove chain selection or reader
completeness.

For a valid plan `P` borrowing source descriptor sequence `S` and containing
selected index sequence `I`, define the independent model:

```text
selected(P)[j] = S[I[j]]                 for 0 <= j < len(I)
len(selected(P)) = len(I)
```

The adapter MUST yield exactly `selected(P)` in order, with no omitted,
duplicated, added, or reordered descriptor. Each yielded reference MUST point
to the corresponding descriptor in the original borrowed slice. The plan's
construction contract already establishes that every selected index is within
`S`; the adapter acceptance MUST rely on and retain that invariant, and MUST
not claim bounds for caller-forged or mutated plans.

The formal evidence MUST independently state the index-to-source projection
and refine the project-level selected-position accessor/model to it. If Verus
cannot model the opaque `impl Iterator` return type, the proof MUST stop at the
project helper/accessor contract and clearly identify the iterator adapter as
a native-tested boundary. In that case, do not claim a Verus proof of
`Iterator`, `Map`, `collect`, or Rust standard-library iterator internals.

## Acceptance requirements

1. The verified contract/model MUST define the selected descriptor sequence
   from the plan's original borrowed slice and stored index sequence. For each
   in-range position it MUST establish the exact source index and descriptor;
   for an out-of-range position it MUST return no descriptor. This may
   strengthen or reuse `selected_set_at`, but the model MUST be independent of
   the production `sets()` implementation.
2. The production `sets()` adapter MUST preserve the selected sequence's exact
   length and order and yield references to the original source entries. Its
   index safety MUST be justified by the private plan construction invariant;
   implementation review MUST check that no mutation path can invalidate the
   stored indices or source slice.
3. Native tests MUST compare actual `sets()` output with an independently
   assembled expected sequence and check pointer identity for each yielded
   reference. Cover at least:
   - A full-only plan whose target equals the anchor's effective end.
   - A multi-hop plan whose selected descriptors occur in non-chain order in
     the input slice and whose unused full/incremental descriptors are
     omitted. Preserve the existing `[1, 2, 0]` selected-index fixture or an
     equivalent case, and assert the exact output count and references.
   - The valid plan's selected-position boundary: last valid position maps to
     its exact source descriptor, and positions at/after the sequence length
     return none through the checked accessor.
4. Existing missing, ambiguous, invalid-anchor, and target-before-anchor
   tests MUST remain unchanged and pass. This adapter milestone MUST NOT
   reinterpret those outcomes or assert that an invalid chain has an
   iterable plan.

## Duvet and verification evidence

- Link the selected-position model/helper to PARSE-CHAIN-001 and
  PARSE-CHAIN-002, as applicable to the exact contract clauses.
- Link the native `sets()` adapter tests to PARSE-CHAIN-001/-002 only when
  their assertions cover the selected sequence for a successfully planned
  chain. Do not link these tests to volume existence, snapshot reconstruction,
  or source provenance requirements.
- Add/update targeted Duvet queries and run `cargo xtask trace-update` and
  `cargo xtask trace`; review the resulting requirement-to-evidence links.
- Run the focused Verus proof/test, focused `duplicity-core` chain tests,
  formatting, strict Clippy, and full `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before marking the acceptance complete.
- Independent verifier review MUST check model independence and index bounds.
  Independent implementation review MUST check the adapter preserves exact
  source references/order and that private construction remains the only way
  to obtain a `ChainPlan`.

## Exclusions

This acceptance does not prove Rust standard-library iterator internals,
`ChainPlan::new`'s chain-selection algorithm beyond its existing accepted
contract, descriptor parsing/provenance, required data-volume identity or
existence, catalog enumeration, reader-controller ordering, cancellation,
resource accounting, archive integrity, snapshot state, encryption, or
filesystem restoration. It does not complete PARSE-CHAIN-001/-002 or same-
source complete-core acceptance.

## Checklist

- [ ] Specify and verify the independent selected-index-to-descriptor model
      and checked position boundary.
- [ ] Test `sets()` exact count, order, and original-reference identity for
      full-only and nontrivial multi-hop plans.
- [ ] Preserve existing chain-selection failure tests and confirm invalid
      plans do not expose a selected-set adapter.
- [ ] Duvet links/queries, trace, formatting, strict Clippy, focused/full CI,
      hosted Linux assurance, and Duvet Pages pass.
- [x] Independent specification review accepts the scoped plan.
- [ ] Independent verifier and implementation reviews accept the scoped
      evidence.
