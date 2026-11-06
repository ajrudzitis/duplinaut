# PARSE-NAME-001 Short IncrementalSignatures Reverse-Success Acceptance

**Status:** Proposed; pending independent specification review. No implementation or proof has been accepted for this slice.

## Goal and scope

Prove the positive reverse-success implication for the existing short `IncrementalSignatures` raw-body model:

> For any raw body and modeled fields, if `short_incremental_signatures_raw_body_model(input, start, end, envelope, partial)` holds, then production `short_incremental_signatures_body(input, separator)` returns exactly an `ObjectName` with `kind = IncrementalSignatures`, `short = true`, those start/end timestamps, `volume = None`, the modeled envelope, and partial flag.

The short route does not consume the time separator. This is positive success refinement only, mapped to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `short_incremental_signatures_raw_body_model` is the antecedent. It existentially chooses the start/end/trailer dot-field splits, suffix witnesses, and semantic field tuple.
- `short_incremental_signatures_fields_model` models the short increasing interval and signatures trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `short_incremental_signatures_raw_body_from_fields` projects already-separated field/suffix witnesses into the raw-body model; it does not establish the converse to production parsing.
- `short_incremental_signatures_body` is the deterministic production parser under proof.
- Keep `names::tests::short_incremental_signatures_body_raw_result_matrix` and `short_incremental_signatures_body_error_and_success_edge_matrix` as regression coverage. The raw-result matrix already asserts exact `ObjectName` values for 18 valid raw bodies.

The model existentially chooses three dot-field boundaries, while production uses deterministic `take_dot_field` calls followed by `take_body_suffix_fields`. The proof must derive that every valid model witness aligns with those production splits, including suffix presence and absence, and yields the same exact fields. Acceptance is conditional on a strict Verus proof; tests alone or assumed witness alignment do not suffice.

## Requirements

1. MUST prove the implication from `short_incremental_signatures_raw_body_model` to exact successful production result. The result must match every modeled field: `ObjectKind::IncrementalSignatures`, `short == true`, `start`, `end`, `volume == None`, `envelope`, and `partial`.
2. MUST prove alignment between the model's existential start/end/trailer/suffix witnesses and deterministic extraction in `short_incremental_signatures_body`; derive the alignment rather than presume the witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `short_incremental_signatures_raw_body_model(b"1.2.st", 1, 2, Envelope::Plain, false)` (using the appropriate Verus byte-sequence form). This is the existing positive matrix case with base36 timestamps `1` and `2`, signatures trailer `st`, and no suffix.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve exact successful results and current `Malformed` behavior; error completeness is outside this acceptance.
6. MUST retain the existing 18-case native raw-body regression matrix: increasing base36 intervals `1→2`, `z→10`, and `10→11`, crossed with six supported suffix profiles, asserting exact `ObjectName` values. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST add reverse-success proof and positive-result evidence under `PARSE-NAME-001` in Duvet. The existing matrix also exercises base36 timestamp decoding and strict interval ordering; keep its current separate `PARSE-NAME-003` and `PARSE-NAME-006` annotations without attributing those standalone requirement proofs to this converse.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::short_incremental_signatures_body_raw_result_matrix` and relevant edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result evidence under `PARSE-NAME-001`; existing `PARSE-NAME-003` and `PARSE-NAME-006` trace links remain distinct.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact field correspondence, and absence of assumptions or weakened conditions. Independent implementation/test review confirms tests exercise production parsing.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for another body route, parse-name controller refinement, or a raw-model iff theorem. It does not re-prove base36 decoding or interval-order policy as standalone requirements.

## Checklist

- [ ] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
