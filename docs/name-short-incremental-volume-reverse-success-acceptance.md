# PARSE-NAME-001 Short IncrementalVolume Reverse-Success Acceptance

**Status:** Proposed; pending independent specification review. No implementation or proof has been accepted for this slice.

## Goal and scope

Prove the positive reverse-success implication for the existing short `IncrementalVolume` raw-body model:

> For any raw body and modeled fields, if `short_incremental_volume_raw_body_model(input, start, end, volume, envelope, partial)` holds, then production `short_incremental_volume_body(input, separator)` returns exactly an `ObjectName` with `kind = IncrementalVolume`, `short = true`, those start/end timestamps, that volume, the modeled envelope, and partial flag.

The short route does not consume the time separator. This is positive success refinement only, mapped to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `short_incremental_volume_raw_body_model` is the antecedent. It existentially chooses start, end, volume, and trailer dot-field splits, suffix witnesses, and the semantic tuple.
- `short_incremental_volume_fields_model` models the short increasing interval, positive base36 volume, and volume trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `short_incremental_volume_raw_body_from_fields` projects already-separated field/suffix witnesses into the raw-body model; it does not establish the converse from the raw model to production parsing.
- `short_incremental_volume_body` is the deterministic production parser under proof.
- Keep `names::tests::short_incremental_volume_body_raw_result_matrix` and `short_incremental_volume_body_error_and_success_edge_matrix` as regression coverage. The raw-result matrix already asserts exact `ObjectName` values for 18 valid raw bodies.
- `name-short-incremental-volume-success-result-refinement` in `docs/assurance.toml` is the existing forward-success checkpoint and explicitly leaves the raw-model-to-parser converse open; this plan is distinct and must not replace or overstate it.

The raw-body predicate existentially chooses four dot-field boundaries, while production uses deterministic `take_dot_field` calls followed by `take_body_suffix_fields`. The proof must derive that every valid model witness aligns with those production splits, including suffix presence and absence, and yields the exact timestamp, volume, envelope, and partial values. Acceptance is conditional on a strict Verus proof; tests alone or assumed witness alignment do not suffice.

## Requirements

1. MUST prove the implication from `short_incremental_volume_raw_body_model` to the exact successful production result. The result must match all modeled fields: `ObjectKind::IncrementalVolume`, `short == true`, `start`, `end`, `volume`, `envelope`, and `partial`.
2. MUST prove alignment between the model's existential start/end/volume/trailer/suffix witnesses and deterministic extraction in `short_incremental_volume_body`; derive the alignment rather than presume the witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `short_incremental_volume_raw_body_model(b"1.2.10.dt", 1, 2, 36, Envelope::Plain, false)` (using the appropriate Verus byte-sequence form). This is the existing positive matrix case with interval `1→2`, base36 volume token `10` (36), trailer `dt`, and no suffix.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve exact successful results and current `Malformed` behavior; error completeness is outside this acceptance.
6. MUST retain the existing 18-case native raw-body regression matrix: increasing intervals `1→2`, `z→10`, and `10→11`, crossed with six supported suffix profiles, using volume field `10` and asserting exact `ObjectName` values with volume `Some(36)`. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST add the reverse-success proof and positive-result evidence under `PARSE-NAME-001` in Duvet. Preserve the existing matrix's separate `PARSE-NAME-003` base36 and `PARSE-NAME-006` interval-order annotations; do not attribute those standalone requirement proofs to this converse.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::short_incremental_volume_body_raw_result_matrix` and relevant edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result evidence under `PARSE-NAME-001`; existing `PARSE-NAME-003` and `PARSE-NAME-006` links remain distinct.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines split/suffix witness alignment, non-vacuity, exact field correspondence, and absence of assumptions or weakened conditions. Independent implementation/test review confirms tests exercise production parsing.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for another body route, parse-name controller refinement, or a raw-model iff theorem. It does not re-prove base36 decoding or strict interval policy as standalone requirements.

## Checklist

- [ ] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
