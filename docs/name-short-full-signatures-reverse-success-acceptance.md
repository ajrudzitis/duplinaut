# PARSE-NAME-001 Short FullSignatures Reverse-Success Acceptance

**Status:** Proposed; pending independent specification review. No implementation or proof has been accepted for this slice.

## Goal and scope

Prove the positive reverse-success implication for the existing short `FullSignatures` raw-body model:

> For any raw body and modeled fields, if `short_full_signatures_raw_body_model(input, start, end, envelope, partial)` holds, then production `short_full_signatures_body(input, separator)` returns exactly an `ObjectName` with `kind = FullSignatures`, `short = true`, those start/end timestamps, `volume = None`, the modeled envelope, and partial flag.

The short route does not consume the time separator. This is positive success refinement only, traced to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `short_full_signatures_raw_body_model` is the antecedent. It existentially chooses the timestamp/trailer dot-field split, suffix witnesses, and semantic field tuple.
- `short_full_signatures_fields_model` models the short FullSignatures timestamp and signature-trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `short_full_signatures_raw_body_from_fields` projects already-separated field/suffix witnesses into the raw-body model; it does not establish the converse to production parsing.
- `short_full_signatures_body` is the deterministic production parser under proof.
- Keep `names::tests::short_full_signatures_body_raw_result_matrix` and `short_full_signatures_body_error_and_success_edge_matrix` as regression coverage. Do not weaken or repurpose the existing `PARSE-NAME-010` error-domain evidence.

The raw-body predicate existentially chooses splits, while production uses deterministic `take_dot_field` and `take_body_suffix_fields`. The proof must establish that every valid model witness aligns with these production splits and yields the same exact fields. This is an explicit proof obligation, not an assumption. Acceptance is conditional on strict Verus proof of the implication; examples or a test-only implication do not suffice.

## Requirements

1. MUST state and prove the implication from `short_full_signatures_raw_body_model` to the exact successful production result. The result must match all modeled fields: `ObjectKind::FullSignatures`, `short == true`, `start`, `end`, `volume == None`, `envelope`, and `partial`.
2. MUST prove alignment between existential raw-body split witnesses and deterministic dot-field/suffix extraction in `short_full_signatures_body`; derive the alignment rather than presuming witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `short_full_signatures_raw_body_model(b"1.st", 1, 1, Envelope::Plain, false)` (using the appropriate Verus byte-sequence form). This is the existing matrix case with base36 timestamp `1`, signatures trailer `st`, and no suffix.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve exact successful results and current `Malformed` behavior; error completeness is outside this acceptance.
6. MUST retain the existing 18-case native raw-body regression matrix: timestamp fields `1`, `z`, and `10` crossed with six supported suffix profiles, asserting exact `ObjectName` results. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST map the proof and positive-result test evidence to `PARSE-NAME-001` in Duvet. Existing `PARSE-NAME-003` coverage for short timestamp decoding remains separate; do not attribute numeric-decoder proof to this reverse-success theorem. Keep `PARSE-NAME-010` associated only with its existing error-domain evidence.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::short_full_signatures_body_raw_result_matrix` and relevant edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result test under `PARSE-NAME-001`, with no change to separate `PARSE-NAME-003` or `PARSE-NAME-010` evidence.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact field correspondence, and absence of assumptions or weakened conditions. Independent implementation/test review confirms the tests exercise production parsing.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for any other body route, parse-name controller refinement, or a raw-model iff theorem. It does not expand `PARSE-NAME-010`; that requirement remains supported only by existing error-domain evidence.

## Checklist

- [ ] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
