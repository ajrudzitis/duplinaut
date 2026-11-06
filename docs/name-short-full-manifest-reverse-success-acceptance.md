# PARSE-NAME-001 Short FullManifest Reverse-Success Acceptance

**Status:** Specification and independent implementation/proof reviews, strict proof, tests, local full `cargo xtask ci`, hosted Linux assurance, and Duvet Pages publication passed in [GitHub Actions run 42212428925](https://github.com/ajrudzitis/duplinaut/actions/runs/42212428925). Broader filename-parser/controller work remains open.

## Goal and scope

Prove the positive reverse-success implication for the existing short `FullManifest` raw-body model:

> For any raw body and modeled fields, if `short_full_manifest_raw_body_model(input, start, end, envelope, partial)` holds, then production `short_full_manifest_body(input, separator)` returns exactly an `ObjectName` with `kind = FullManifest`, `short = true`, those start/end timestamps, `volume = None`, the modeled envelope, and partial flag.

The short route does not consume the time separator. This is positive success refinement only and is traced to `PARSE-NAME-001` classification semantics.

## Existing model and witness-alignment risk

Reuse the existing definitions in `crates/core/src/names.rs`:

- `short_full_manifest_raw_body_model` is the antecedent. It existentially chooses the timestamp/trailer dot-field split, suffix witnesses, and the semantic field tuple.
- `short_full_manifest_fields_model` models the short FullManifest timestamp and trailer/suffix profile.
- `body_success_result_model` states the forward direction: an `Ok(name)` is consistent with the raw-body model.
- `short_full_manifest_raw_body_from_fields` projects already-separated field/suffix witnesses into the raw-body model; it is not the required converse from the model to production parsing.
- `short_full_manifest_body` is the deterministic production parser under proof.
- Keep `short_full_manifest_body_raw_result_matrix` and `short_full_manifest_body_error_and_success_edge_matrix` as regression coverage. Do not weaken or repurpose the `PARSE-NAME-010` error-domain evidence.

The raw-body predicate existentially chooses splits, while production uses deterministic `take_dot_field` and `take_body_suffix_fields`. The proof must establish that every valid model witness aligns with those production splits and therefore yields the same exact semantic fields. This is a proof obligation, not an assumption. Acceptance is conditional on strict Verus proof of the implication; examples or a test-only implication do not suffice.

## Requirements

1. MUST state and prove the implication from `short_full_manifest_raw_body_model` to the exact successful production result. The result must match all modeled fields: `ObjectKind::FullManifest`, `short == true`, `start`, `end`, `volume == None`, `envelope`, and `partial`.
2. MUST prove alignment between the existential raw-body split witnesses and the deterministic dot-field/suffix extraction in `short_full_manifest_body`; the proof must derive this alignment rather than presume the witnesses equal production slices.
3. MUST separately establish a concrete, non-vacuous antecedent witness in strict Verus: `short_full_manifest_raw_body_model(b"1.m", 1, 1, Envelope::Plain, false)` (using the appropriate Verus byte-sequence form). This is the existing positive matrix case with base36 timestamp `1`, manifest trailer `m`, and no suffix.
4. MUST run the focused Verus proof with the repository's strict no-cheating configuration. Both the implication and witness must be discharged without `assume`, `external_body`, proof bypasses, or weakened obligations.
5. MUST preserve existing production behavior unless a proof-driven implementation adjustment is needed. Such an adjustment must preserve exact successful results and current `Malformed` behavior; error completeness is outside this acceptance.
6. MUST retain the existing 18-case native raw-body regression matrix: timestamp fields `1`, `z`, and `10` crossed with the six supported suffix profiles, with exact `ObjectName` assertions. Any new theorem-facing positive test MUST be linked specifically to positive `PARSE-NAME-001` semantics.
7. MUST map the proof and positive test evidence to `PARSE-NAME-001` in Duvet. Keep `PARSE-NAME-010` associated only with its existing error-domain contract/test; this slice adds no error-domain claim.

## Verification and acceptance evidence

- Focused strict Verus proof of the reverse-success implication and concrete antecedent witness.
- Existing `names::tests::short_full_manifest_body_raw_result_matrix` and relevant edge tests pass without weakened assertions.
- Focused Duvet queries and complete trace report show proof and positive-result test evidence under `PARSE-NAME-001`; `PARSE-NAME-010` remains unchanged.
- Formatting, strict Clippy, full core tests, full `cargo xtask ci`, hosted Linux assurance, and Duvet report publication pass.
- Independent proof review examines witness alignment, non-vacuity, exact field correspondence, and absence of assumptions or weakened conditions. Independent implementation/test review confirms the tests exercise production parsing.

## Exclusions

This acceptance does not prove malformed-input/error completeness, scanner/raw split-model correctness, the full filename grammar, reverse success for other body routes, parse-name controller refinement, or a raw-model iff theorem. It does not expand `PARSE-NAME-010`; that requirement remains supported only by existing error-domain evidence.

## Checklist

- [x] Independent specification review accepts the theorem scope and witness-alignment obligation.
- [x] Implementation, concrete antecedent witness, and strict Verus implication pass independent review.
- [x] Strict names-module verification passes with 133 declarations verified and zero errors; the focused `short_full_manifest_body_` tests pass (two unit tests and the integration composition test), and the targeted PARSE-NAME-001 Duvet query passes implementation and test coverage (53/53 tests). Formatting and strict core Clippy pass.
- [x] Full local `cargo xtask ci` passes, including Duvet, strict workspace Verus, formatting, Clippy, and tests.
- [x] Hosted Linux assurance and Duvet Pages publication pass in [run 42212428925](https://github.com/ajrudzitis/duplinaut/actions/runs/42212428925).
- [x] Scoped implementation and local proof/test evidence are recorded without closing broader filename-parser/controller work.
