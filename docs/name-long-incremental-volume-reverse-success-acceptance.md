# PARSE-NAME-001 Long IncrementalVolume Reverse-Success Acceptance

**Status:** Specification accepted after independent review; implementation and proof remain pending.

## Goal and scope

Prove the positive reverse-success implication for the existing long `IncrementalVolume` raw-body model:

> For any body, configured timestamp separator, and modeled fields, if `long_incremental_volume_raw_body_model(input, separator, start, end, volume, envelope, partial)` holds, then production `body(input, ObjectKind::IncrementalVolume, false, separator)` returns exactly an `ObjectName` with `kind = IncrementalVolume`, `short = false`, those start/end timestamps, that volume, the modeled envelope, and partial flag.

This is positive success refinement only, mapped to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `long_incremental_volume_raw_body_model` is the antecedent. It existentially chooses start, interval-marker, end, volume, trailer, and suffix splits, plus the semantic tuple.
- `long_incremental_volume_fields_model` models the configured increasing interval, exact `to` marker, positive decimal volume, and volume trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `long_incremental_volume_body_fields` already refines a valid field model to an exact successful `ObjectName`; it does not prove the raw-model converse through production body dispatch.
- Production parsing is `body(input, ObjectKind::IncrementalVolume, false, separator)`, which deterministically extracts and dispatches the start, marker, end, volume, trailer, and suffix fields.
- Keep `names::tests::long_incremental_volume_body_raw_result_matrix` and `long_incremental_volume_body_exact_error_edge_matrix` as regression evidence.

The raw-body model existentially chooses five dot-field boundaries, while production extracts them deterministically. The proof must derive alignment for timestamp, literal `to` marker, end timestamp, volume token, trailer and suffix witnesses, preserving the exact tuple. Acceptance is conditional on strict Verus proof; tests or assumed witness alignment do not suffice.

## Requirements

1. MUST prove the implication from `long_incremental_volume_raw_body_model` to the exact successful production `body` result. The result must match `ObjectKind::IncrementalVolume`, `short == false`, `start`, `end`, `volume`, `envelope`, and `partial`.
2. MUST prove alignment between the model's existential timestamp/marker/volume/trailer/suffix witnesses and deterministic production extraction; derive the alignment rather than presume model witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `long_incremental_volume_raw_body_model(b"19700101T000000Z.to.19700101T000001Z.vol42.difftar", b':', 0, 1, 42, Envelope::Plain, false)` (using the appropriate Verus sequence form). This is the existing canonical UTC, epoch-0-to-epoch-1, `to`, decimal-volume-42, plain-volume matrix case.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve existing successful results and error behavior; malformed-input/error completeness is outside this acceptance.
6. MUST retain the existing 24-case native body-parser regression matrix: three UTC-equivalent increasing interval spellings (canonical UTC, legacy UTC, and legacy explicit-zone) crossed with eight supported suffix profiles, using `vol42.difftar` and asserting exact `ObjectName` results with volume 42. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST add the reverse-success proof and positive-result evidence under `PARSE-NAME-001` in Duvet. Preserve existing `PARSE-NAME-004` canonical timestamp, `PARSE-NAME-006` increasing-interval, `PARSE-NAME-007` configured-separator, and `PARSE-GEN-003` checked-conversion evidence separately; do not claim this converse independently proves those requirements.
8. MUST leave existing `PARSE-NAME-010` field-error refinement and its exact error-domain tests unchanged; this slice adds no error claim.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::long_incremental_volume_body_raw_result_matrix` and relevant error/edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result evidence under `PARSE-NAME-001`; the existing `PARSE-NAME-004`, `PARSE-NAME-006`, `PARSE-NAME-007`, `PARSE-GEN-003`, and `PARSE-NAME-010` evidence remains distinct.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact result fields, and absence of assumptions or weakened conditions. Independent implementation/test review confirms the test exercises the production dispatch route.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for another body route, parse-name controller refinement, or a raw-model iff theorem. It does not expand timestamp, interval-order, separator, numeric-conversion, or error-domain requirements.

## Checklist

- [x] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
