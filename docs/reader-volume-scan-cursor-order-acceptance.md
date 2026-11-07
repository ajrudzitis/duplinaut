# Row-major volume-scan cursor acceptance

Status: specification accepted after independent review. Implementation,
proof, tests, and hosted gates remain pending.

## Goal and scope

Strengthen the existing `reader-volume-scan-cursor` assurance slice so its
formal model establishes the row-major position sequence used by the required
volume scan. The production loop in `Catalog::read_snapshot_inner` nests
`volumes.iter().enumerate()` outside `self.objects.iter().enumerate()` and
calls `advance_volume_scan_cursor(cursor, volume_index, object_index)` for each
pair.

This acceptance adds a finite cursor-order refinement for bounded dimensions.
It does not prove Rust slice or iterator internals, catalog ordering, candidate
identity, cancellation timing, or complete reader orchestration.

## Normative requirements

### 1. Bounded rectangular model

The formal model MUST take nonnegative `volume_count = V` and
`object_count = O`, with the product `V * O` representable in `usize` and
equal to the cursor reservation's `planned_positions`. Cursor initialization
MUST start at position zero. If the checked product is not representable or
does not match the reservation, production initialization MUST reject the
cursor, preserving the existing checked-arithmetic/`ReaderError::Limit`
behavior.

The proof MUST cover zero dimensions. If `V == 0` or `O == 0`, the pair domain
is empty, `planned_positions == 0`, and the initialized cursor is complete
without any advance. Zero dimensions MUST NOT be treated as a missing advance
or an arithmetic error.

### 2. Exact row-major correspondence

For all in-range coordinates `0 <= v < V` and `0 <= o < O`, the model MUST
establish that the nested-loop visit `(v, o)` corresponds to the unique
row-major position `v * O + o`, and that this position lies in
`[0, V * O)`. The proof MUST establish that lexicographic traversal of the
pair domain visits exactly positions `0, 1, ..., V*O - 1` in order, with no
duplicate, skipped, or extra position.

The proof MUST connect this sequence property to the current transition
contract: each accepted `advance_volume_scan_cursor` call consumes exactly the
next expected pair and increments `next_position` by one; out-of-range,
reordered, repeated, skipped, and post-completion pairs are rejected. After
the exact nested traversal, `volume_scan_cursor_complete` MUST be true; before
the last required pair it MUST be false. No statement is required about
candidate matching or charge accounting.

### 3. Proof and native test matrix

Strict Verus MUST prove the row-major relation for arbitrary dimensions under
the stated checked-product precondition, rather than only the following
examples. The focused native matrix MUST exercise:

- `(V, O) = (0, 0)`, `(0, 3)`, and `(2, 0)`: initialization is complete and
  every attempted coordinate advance is rejected.
- `(1, 1)`: the sole pair completes the cursor.
- `(1, 3)` and `(3, 1)`: single-row and single-column order.
- `(2, 3)` and `(3, 2)`: multi-row/multi-column row-major traversal.
- A mismatched reservation and a product-overflow case: cursor startup fails
  closed without wrapping or producing a partial cursor.

The existing exact-position, incomplete, and reject-skips/reordering tests in
`verified::reader_volume_scan::tests` SHOULD be strengthened or reused so
test expectations are computed independently of the production transition.
The tests MUST assert the exact accepted coordinate sequence, each resulting
`next_position`, and completion state. They MUST NOT derive the expected
sequence by repeatedly calling `advance_volume_scan_cursor`.

### 4. Production adapter evidence

The acceptance MUST inspect and retain native evidence that the production
nested loops pass `enumerate()` indices in the same outer-volume/inner-object
order to the proved transition before candidate inspection. Reuse the
official reversed-object-order reader test's exact 4/7/9 opens for its three
snapshot dates, extending it only if needed to assert a required-volume scan
adapter observable. Duvet MUST link the test to `PARSE-GEN-005`; retain its
existing `PARSE-CHAIN-001`/`PARSE-CHAIN-002` links for the behaviors it
currently asserts. Do not relabel the manifest-projection test as proof of
volume iterator behavior.

Native adapter evidence MUST be described as implementation/test evidence;
it does not expand the formal model to Rust's standard-library iterator
implementation or prove whole-reader refinement.

## Traceability and assurance ledger

The new model lemmas and cursor tests MUST link only to
[PARSE-GEN-005](parsing-requirements.md#parse-gen-005), the configured resource
bound requirement. Keep the existing `reader-volume-scan-cursor` milestone
and its proof IDs (`reader-volume-scan-cursor-start`,
`reader-volume-scan-cursor-order`, and `reader-volume-scan-cursor-complete`);
extend that milestone's proof/test mappings rather than adding a broad
`reader-orchestration` proof claim. Update `docs/assurance.toml` only as part
of implementation, when concrete proof symbols and tests exist.

## Exclusions

This slice does not prove Rust iterator/slice behavior, source object
enumeration completeness, preservation of catalog order, required-volume
identity matching, candidate uniqueness, budget charge ordering, cancellation
or syscall timing, source identity, digest/authentication, snapshot
reconstruction, or complete repository-reader/resource-accounting
refinement. Existing production and integration behavior remains separately
tested.

## Acceptance evidence

- [x] Independent specification review accepts the bounded model and scope.
- [ ] Strict Verus proves the arbitrary-dimension row-major sequence property,
      including empty dimensions and checked-product boundaries.
- [ ] Focused cursor tests cover the required dimension matrix, exact
      positions/completion, invalid transitions, reservation mismatch, and
      overflow.
- [ ] Production nested-loop adapter evidence is checked against the source;
      the official 4/7/9 reverse-order opens remain accurately described.
- [ ] Duvet targeted `PARSE-GEN-005` queries, trace update, and full trace pass.
- [ ] Formatting, strict Clippy, focused and workspace tests, and full
      `cargo xtask ci` pass.
- [ ] Independent implementation/proof review accepts the model-to-transition
      linkage and native adapter claims.
- [ ] Hosted Linux assurance and Duvet Pages pass.
