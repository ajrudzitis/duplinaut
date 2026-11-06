# Public `parse_name` Malformed-Routing Acceptance

**Status:** Specification and implementation/test independently reviewed. The
12 malformed public results and unrelated control pass; Duvet trace links,
formatting, strict Clippy, full workspace tests, and local full `cargo xtask
ci` pass. Hosted Linux assurance and Duvet Pages publication remain pending.

## Goal and scope

Check that a recognized filename stem with no successful body candidate
returns `NameError::Malformed` through the public `parse_name` entry point,
while an unrelated basename is ignored as `Ok(None)`. The acceptance covers
one literal malformed case for each of the six object kinds in both short and
long profiles. It tests public classification of these exact inputs; it does
not characterize every malformed spelling or assert which overlapping body
candidate caused recognition.

## Inputs and independent expected results

Use `NameConfig::default()` for all cases: valid separator `:`, empty
configured prefixes, and a 4096-byte basename limit. Every listed input is
shorter than the limit and contains neither NUL nor `/`, so configuration and
basename preflight succeed. The expected results are literal and every
malformed input MUST be passed directly to public `parse_name`, not only to a
body helper.

| Kind | Profile | Malformed basename | Expected result | Malformation selected |
| --- | --- | --- | --- | --- |
| FullVolume | short | `df.1.0.dt` | `Err(NameError::Malformed)` | zero volume |
| FullVolume | long | `duplicity-full.19700101T000000Z.vol0.difftar` | `Err(NameError::Malformed)` | zero volume |
| IncrementalVolume | short | `di.1.1.1.dt` | `Err(NameError::Malformed)` | equal start/end |
| IncrementalVolume | long | `duplicity-inc.19700101T000000Z.to.19700101T000000Z.vol1.difftar` | `Err(NameError::Malformed)` | equal start/end |
| FullManifest | short | `df.1.bad` | `Err(NameError::Malformed)` | invalid manifest trailer |
| FullManifest | long | `duplicity-full.bad.manifest` | `Err(NameError::Malformed)` | invalid timestamp |
| IncrementalManifest | short | `di.1.1.m` | `Err(NameError::Malformed)` | equal start/end |
| IncrementalManifest | long | `duplicity-inc.19700101T000000Z.to.19700101T000000Z.manifest` | `Err(NameError::Malformed)` | equal start/end |
| FullSignatures | short | `dfs.1.bad` | `Err(NameError::Malformed)` | invalid signatures trailer |
| FullSignatures | long | `duplicity-full-signatures.bad.sigtar` | `Err(NameError::Malformed)` | invalid timestamp |
| IncrementalSignatures | short | `dns.1.1.st` | `Err(NameError::Malformed)` | equal start/end |
| IncrementalSignatures | long | `duplicity-new-signatures.19700101T000000Z.to.19700101T000000Z.sigtar` | `Err(NameError::Malformed)` | equal start/end |

Use `not-a-duplicity-object` as the unrelated control and assert exactly
`Ok(None)`.

## Requirements

1. MUST assert all twelve public results exactly as shown by calling
   `parse_name` with the same valid configuration. Annotate these assertions
   with `PARSE-NAME-001`: each input is classified as a recognized name with
   no successful body candidate, and the public result is `Malformed`.
2. MUST assert the unrelated control returns exactly `Ok(None)` through public
   `parse_name`, annotated with `PARSE-NAME-001` for unrelated-name
   classification/ignore behavior.
3. MUST demonstrate the inputs reach name parsing after valid preflight: use
   default configuration, inputs below 4096 bytes, and no NUL or `/` bytes.
   Each input MUST have a recognized kind/profile stem, with malformed body
   fields as identified in the table. Short `df`/`di` and long
   `duplicity-full`/`duplicity-inc` stems can match overlapping candidates;
   public assertions MUST NOT claim a unique route or identify which candidate
   caused recognition.
4. MUST use independent literal input/result pairs. The test MUST NOT derive
   expected errors by invoking the body parser or another parser/controller
   helper.
5. MUST preserve the existing parser and error semantics. Any implementation
   adjustment MUST retain all twelve exact public results and the unrelated
   `Ok(None)` result.

## Verification and review

- Run the focused integration test and full core test suite.
- Link public `parse_name` result assertions to `PARSE-NAME-001` and the
  unrelated control to `PARSE-NAME-001` in Duvet. Body-parser error-variant
  coverage for `PARSE-NAME-010` remains supplied by the existing direct body
  tests; this acceptance does not assert those helper results. Run the focused
  `PARSE-NAME-001` query and the full trace check.
- Run formatting, strict Clippy, and full `cargo xtask ci`; require hosted Linux
  assurance and Duvet Pages publication before marking acceptance complete.
- Independent review MUST verify each input has the stated recognized
  stem/profile, satisfies preflight, and returns public `Malformed` with no
  successful body candidate. It MUST NOT infer which overlapping candidate
  caused recognition; it MUST also check that Duvet links do not imply unique
  candidate selection or complete error coverage.

## Exclusions

This acceptance does not establish malformed-input completeness, a raw
filename-grammar iff theorem, all body-error cases, ambiguity reachability,
success-result refinement, or full `parse_name` controller refinement. It adds
no new Verus proof obligation and does not expand the normative scope of
`PARSE-NAME-010` beyond the existing body-error domain requirement.

## Checklist

- [x] Independent specification review accepts scope, inputs, and trace links.
- [x] Twelve public malformed-routing expectations and the unrelated control
      pass; Duvet links are reviewed.
- [x] Focused/full tests, Duvet queries/trace, formatting, strict Clippy, and
      local `cargo xtask ci` pass.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
