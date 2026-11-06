# PARSE-NAME-001 Short FullVolume Reverse-Success Acceptance

**Status:** Implementation, strict proof, test evidence, and independent specification/implementation review are complete. Local full `cargo xtask ci` passed; its machine-readable source digest is recorded in generated `target/assurance/ci.json`. Hosted Linux assurance and Duvet Pages publication remain pending. The broader filename parser/controller remains open.

## Goal and scope

Prove the positive reverse-success implication for the short `FullVolume` raw-body model already used by the parser contract:

> For any raw body and modeled fields, if `short_full_volume_raw_body_model(input, start, end, volume, envelope, partial)` holds, then production `short_full_volume_body(input, separator)` returns exactly the `ObjectName` with `kind = FullVolume`, `short = true`, those start/end timestamps, that volume, envelope, and partial flag.

The separator is irrelevant on this short route, as in current production behavior. This is a positive success refinement only, traced to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `short_full_volume_raw_body_model` is the antecedent. It existentially chooses dot-field split witnesses, suffix witnesses, and the field-model tuple.
- `short_full_volume_fields_model` models a valid short interval, positive base36 volume, and FullVolume trailer/suffix profile.
- `body_success_result_model` currently states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `short_full_volume_body` is the deterministic production parser under proof.
- Keep the existing `short_full_volume_body_raw_result_matrix` and `short_full_volume_body_error_and_success_edge_matrix` tests as regression coverage; do not weaken or repurpose the `PARSE-NAME-010` error-domain evidence.

The raw-body model uses existential split witnesses, while production calls deterministic `take_dot_field` and `take_body_suffix_fields`. The proof must establish that a witness satisfying the model aligns with those production splits and yields the same exact fields. This is a material proof risk. Acceptance is conditional on a strict Verus proof of the implication; examples, a test-only implication, an `assume`, weakened postconditions, or a proof that only restates the existing forward direction do not satisfy this plan.

## Requirements

1. MUST state and prove the implication from `short_full_volume_raw_body_model` to exact successful production result. The resulting `ObjectName` must match every modeled field: `ObjectKind::FullVolume`, `short == true`, `start`, `end`, `volume`, `envelope`, and `partial`.
2. MUST prove the raw-model witness alignment with the actual deterministic field and suffix extraction used by `short_full_volume_body`; the proof must not presume the existential witnesses equal production slices without deriving that fact.
3. MUST separately prove at least one concrete antecedent witness so the reverse implication cannot pass vacuously: `short_full_volume_raw_body_model(b"1.10.dt", 1, 1, 36, Envelope::Plain, false)` (with the appropriate Verus byte-sequence form) must be established by strict Verus. This corresponds to the existing raw-body matrix case with timestamp `1`, volume token `10` (base36 value 36), and plain trailer `dt`.
4. MUST run the focused Verus check with the repository's strict no-cheating assurance configuration and demonstrate that both the concrete witness and implication are discharged without `assume`, `external_body`, proof bypasses, or reduced obligations that evade these claims.
5. MUST preserve the existing parser implementation and error-domain contract unless a proof-driven implementation adjustment is needed. Any adjustment must preserve the current exact success results and `Malformed` behavior; error completeness is outside this acceptance.
6. MUST retain native positive regression coverage using the existing 18-case raw-body matrix (three timestamp tokens crossed with six supported suffix profiles), with exact `ObjectName` assertions. If a new test is added to make the theorem-facing model relation observable, it MUST be linked specifically to the positive `PARSE-NAME-001` claim.
7. MUST trace the proof and positive test evidence to `PARSE-NAME-001` in Duvet. Keep `PARSE-NAME-010` associated only with the already-existing error-domain contract/test; this work adds no new error-domain claim.

## Verification and acceptance evidence

- Focused Verus proof of the reverse-success implication and the concrete non-vacuity witness, under strict no-cheating mode.
- Core test for `short_full_volume_body_raw_result_matrix` and relevant existing edge tests.
- Duvet focused queries and complete trace report showing positive proof/test links under `PARSE-NAME-001` and no change to the scoped `PARSE-NAME-010` evidence.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, and hosted Linux assurance with Duvet report publication.
- Independent proof review must inspect the witness-alignment argument, exact `ObjectName` field correspondence, and absence of assumptions or weakened conditions. A separate implementation/test review must check that regression tests exercise production parsing.

## Exclusions

This acceptance does not prove malformed-input or error completeness, correctness or completeness of the scanner/raw split model, the full filename grammar, reverse-success for any other body route, parse-name controller refinement, or a raw-model iff theorem. It does not expand `PARSE-NAME-010`; that requirement remains supported only by its existing error-domain evidence.

## Checklist

- [x] Independent spec review accepts this bounded theorem and witness-alignment obligation.
- [x] Implementation, concrete antecedent witness, and strict Verus implication accepted by independent review.
- [x] Existing exact-result regression tests, Duvet trace, formatting, strict Clippy, and full core tests pass. The `names` module passed strict verification with 131 verified declarations and zero errors.
- [x] Local full assurance passed: `cargo xtask ci` under the pinned Nix shell produced generated `target/assurance/ci.json`, which records the source digest. The full run reported Verus crates at 2047, 3183, and 1291 verified declarations, each with zero errors; workspace tests, formatting, strict Clippy, and Duvet passed.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
- [x] Scoped implementation and local evidence are recorded without closing broader filename-parser/controller work.
