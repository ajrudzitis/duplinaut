# PARSE-NAME-001 Long FullVolume Reverse-Success Acceptance

**Status:** Specification accepted after independent review; implementation and proof remain pending.

## Goal and scope

Prove the positive reverse-success implication for the existing long `FullVolume` raw-body model:

> For any body, configured timestamp separator, and modeled fields, if `long_full_volume_raw_body_model(input, separator, start, end, volume, envelope, partial)` holds, then production `body(input, ObjectKind::FullVolume, false, separator)` returns exactly an `ObjectName` with `kind = FullVolume`, `short = false`, those start/end timestamps, that volume, the modeled envelope, and partial flag.

This is positive success refinement only, mapped to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `long_full_volume_raw_body_model` is the antecedent. It existentially chooses timestamp, volume, and trailer dot-field splits, suffix witnesses, and the semantic field tuple.
- `long_full_volume_fields_model` models the configured canonical/legacy timestamp, positive decimal volume, and volume trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `long_full_volume_body_fields` already refines a valid field model to an exact successful `ObjectName`; it does not establish the converse through production body dispatch.
- Production parsing is `body(input, ObjectKind::FullVolume, false, separator)`, which deterministically extracts and dispatches the timestamp, volume, trailer, and suffix fields.
- Keep `names::tests::long_full_volume_body_raw_result_matrix` and relevant field/error tests as regression evidence.

The raw-body model existentially chooses field boundaries, while production extracts them deterministically. The proof must derive alignment for timestamp, volume, trailer, and suffix witnesses so the exact semantic tuple reaches the same production parser path. Acceptance is conditional on strict Verus proof; examples or assumed witness alignment do not suffice.

## Requirements

1. MUST prove the implication from `long_full_volume_raw_body_model` to the exact successful production `body` result. The result must match `ObjectKind::FullVolume`, `short == false`, `start`, `end`, `volume`, `envelope`, and `partial`.
2. MUST prove alignment between the model's existential timestamp/volume/trailer/suffix witnesses and deterministic production extraction; derive the alignment rather than presume model witnesses equal production slices.
3. MUST separately establish a concrete non-vacuous antecedent witness in strict Verus: `long_full_volume_raw_body_model(b"19700101T000000Z.vol42.difftar", b':', 0, 0, 42, Envelope::Plain, false)` (using the appropriate Verus sequence form). This is the existing canonical UTC, epoch-zero, decimal-volume-42, plain-volume matrix case.
4. MUST run focused Verus proof under the repository's strict no-cheating configuration. Both implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve existing successful results and error behavior; malformed-input/error completeness is outside this acceptance.
6. MUST retain the existing 24-case native body-parser regression matrix: canonical UTC, legacy UTC, and legacy explicit-zone timestamp spellings crossed with eight supported suffix profiles, using `vol42.difftar` and asserting exact `ObjectName` results with volume 42. Any new theorem-facing positive test MUST link specifically to positive `PARSE-NAME-001` semantics.
7. MUST add the reverse-success proof and positive-result evidence under `PARSE-NAME-001` in Duvet. Preserve existing `PARSE-NAME-004` canonical timestamp, `PARSE-NAME-007` configured-separator, and `PARSE-GEN-003` checked-conversion evidence separately; do not claim this converse independently proves those requirements.
8. MUST leave existing `PARSE-NAME-010` error-domain refinement and its tests unchanged; this slice adds no error claim.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::long_full_volume_body_raw_result_matrix` and relevant field/error tests pass without weakened assertions.
- Focused Duvet queries and complete trace show the new proof and positive-result evidence under `PARSE-NAME-001`; existing `PARSE-NAME-004`, `PARSE-NAME-007`, `PARSE-GEN-003`, and `PARSE-NAME-010` evidence remains distinct.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact result fields, and absence of assumptions or weakened conditions. Independent implementation/test review confirms the test exercises the production dispatch route.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for another body route, parse-name controller refinement, or a raw-model iff theorem. It does not expand timestamp, separator, checked-conversion, or error-domain requirements.

## Checklist

- [x] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [ ] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [ ] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass.
- [ ] Local full assurance and hosted Linux assurance plus Duvet publication pass.
- [ ] Scoped acceptance is recorded without closing broader filename-parser/controller work.
