# PARSE-NAME-001 Long FullManifest Reverse-Success Acceptance

**Status:** Proposed; pending independent specification review. No implementation or proof has been accepted for this slice.

## Goal and scope

Prove the positive reverse-success implication for the existing long `FullManifest` raw-body model:

> For any body, configured timestamp separator, and modeled fields, if `long_full_manifest_raw_body_model(input, separator, start, end, envelope, partial)` holds, then production `long_full_manifest_body(input, separator)` returns exactly an `ObjectName` with `kind = FullManifest`, `short = false`, those start/end timestamps, `volume = None`, the modeled envelope, and partial flag.

This is positive success refinement only, mapped to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `long_full_manifest_raw_body_model` is the antecedent. It existentially chooses the timestamp and trailer dot-field splits, suffix witnesses, and semantic field tuple.
- `long_full_manifest_fields_model` models the configured canonical/legacy timestamp and manifest trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `long_full_manifest_body_fields` already refines a valid field model to an exact successful `ObjectName`; it does not establish the raw-model converse to production parsing.
- `long_full_manifest_body` is the deterministic production parser under proof.
- Keep `names::tests::long_full_manifest_body_raw_result_matrix` and the existing body error/success tests as regression coverage.

The raw-body predicate existentially chooses field splits, while production uses deterministic `take_dot_field` calls followed by `take_body_suffix_fields`. The proof must derive that every valid model witness aligns with those production splits and yields the exact timestamp and trailer/suffix fields. Acceptance is conditional on strict Verus proof; test examples or assumed witness alignment do not suffice.

## Requirements

1. MUST prove the implication from `long_full_manifest_raw_body_model` to the exact successful production result. The result must match `ObjectKind::FullManifest`, `short == false`, `start`, `end`, `volume == None`, `envelope`, and `partial`.
2. MUST prove alignment between the model's existential timestamp/trailer/suffix witnesses and deterministic extraction in `long_full_manifest_body`; derive the alignment rather than presume model witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `long_full_manifest_raw_body_model(b"19700101T000000Z.manifest", b':', 0, 0, Envelope::Plain, false)` (using the appropriate Verus sequence form). This is the existing matrix's canonical UTC, epoch-zero, plain-manifest case.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve exact successful results and current error behavior; malformed-input/error completeness is outside this acceptance.
6. MUST retain the existing 24-case native body-parser regression matrix: canonical UTC, legacy UTC, and legacy explicit-zone timestamp spellings crossed with eight supported suffix profiles, asserting exact `ObjectName` results. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST add the reverse-success proof and positive-result evidence under `PARSE-NAME-001` in Duvet. Preserve existing `PARSE-NAME-004` canonical timestamp and `PARSE-NAME-007` configured separator evidence as distinct requirement trace; do not claim this converse proves those parser requirements independently.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::long_full_manifest_body_raw_result_matrix` and relevant edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result evidence under `PARSE-NAME-001`; existing `PARSE-NAME-004` and `PARSE-NAME-007` evidence remains distinct.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact result fields, and absence of assumptions or weakened conditions. Independent implementation/test review confirms tests exercise the production long-body route.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for another body route, parse-name controller refinement, or a raw-model iff theorem. It does not expand `PARSE-NAME-004` or `PARSE-NAME-007` beyond their separate existing proof/test evidence.

## Checklist

- [ ] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
