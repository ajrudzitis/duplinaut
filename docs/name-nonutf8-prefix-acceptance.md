# Public `parse_name` Non-UTF-8 Prefix Acceptance

**Status:** Specification and implementation/test independently reviewed. The
three public results, six byte-exact near-misses, Duvet links, formatting,
strict Clippy, full workspace tests, and local full `cargo xtask ci` pass.
Hosted Linux assurance and Duvet Pages publication remain pending.

## Goal and scope

Exercise successful public `parse_name` calls when configured global and
kind-specific prefixes contain non-UTF-8 bytes. Cover one volume/archive name,
one manifest name, and one signatures name, with independent exact
`ObjectName` expectations. For each positive row, require one-byte global and
selected kind-prefix near misses to return `Ok(None)`.

The test MUST use byte slices throughout. It MUST NOT convert the prefixes or
input names through UTF-8 or another text representation. This is a focused
acceptance of arbitrary valid prefix bytes and literal matching, not a new
filename grammar or controller proof.

## Configuration and oracle

Use one `NameConfig` for all cases, with these distinct one-byte prefixes:

| Field | Byte |
| --- | --- |
| global prefix | `0xFF` |
| archive prefix | `0xFE` |
| manifest prefix | `0xFD` |
| signature prefix | `0xFC` |

Use separator `:` and `max_name_bytes = 4096`. Every configured prefix is
nonempty, distinct, non-UTF-8, and contains neither NUL nor `/`, so it is valid
under `PARSE-NAME-009`.

Construct each input as a byte-slice concatenation of the global byte, selected
kind byte, and ASCII basename shown below. These expected values are literal
oracle data, not results derived by parser helpers. All three successful
objects have `Envelope::Plain` and `partial == false`.

| Kind | Input bytes | Expected start | Expected end | Expected volume | `short` |
| --- | --- | ---: | ---: | ---: | --- |
| FullVolume | `0xFF || 0xFE || b"df.1.1.dt"` | 1 | 1 | `Some(1)` | `true` |
| FullManifest | `0xFF || 0xFD || b"df.1.m"` | 1 | 1 | `None` | `true` |
| FullSignatures | `0xFF || 0xFC || b"dfs.1.st"` | 1 | 1 | `None` | `true` |

For every row, assert `parse_name(input, config) == Ok(Some(expected))` by
checking every `ObjectName` field: kind, short, start, end, volume, envelope,
and partial.

## Requirements

1. MUST parse all three positive inputs through the public `parse_name` API
   using the same configured byte prefixes and assert the exact oracle values.
   Link kind/date/volume classification to `PARSE-NAME-001`.
2. MUST establish literal byte matching with two near misses per row. First
   change only the global input byte from `0xFF` to `0xEF`; then restore the
   global byte and change only the selected kind input byte (`0xFE`, `0xFD`, or
   `0xFC`) to a distinct nonmatching byte (`0xEE`, `0xED`, or `0xEC`,
   respectively). Each near miss MUST return exactly `Ok(None)`. Link these
   assertions to `PARSE-NAME-002`.
3. MUST annotate the valid configured non-UTF-8 prefix cases under
   `PARSE-NAME-009`, which permits arbitrary prefix bytes other than NUL and
   `/`. The test MUST use only byte slices/arrays and MUST NOT call UTF-8
   decoding or string conversion for configured prefixes or input names.
4. MUST keep all expected values independent from implementation helpers. A
   test that calls candidate/body helpers to construct the expected object
   does not satisfy this acceptance.
5. MUST add Duvet links for positive classifications under `PARSE-NAME-001`,
   positive and negative literal matching under `PARSE-NAME-002`, and valid
   arbitrary-byte prefix policy under `PARSE-NAME-009`. The focused queries and
   full trace MUST show these links.

## Verification and review

- Add a focused integration test in `crates/core/tests/names.rs` and run it,
  then run the full core test suite.
- Run focused Duvet queries for `PARSE-NAME-001`, `PARSE-NAME-002`, and
  `PARSE-NAME-009`, followed by the full trace check.
- Run formatting, strict Clippy, and full `cargo xtask ci`; require hosted Linux
  assurance and Duvet Pages publication before marking acceptance complete.
- Independent review MUST verify the input byte construction, exact one-byte
  mutations, expected `ObjectName` fields, and that no string conversion
  undermines the non-UTF-8 test.

## Exclusions

This acceptance does not test malformed inputs, establish a raw filename
grammar iff theorem, cover every kind/profile combination, prove ambiguity
unreachability, add a Verus obligation, or complete `parse_name` controller
refinement. Those broader questions remain open.

## Checklist

- [x] Independent specification review accepts the byte cases, oracle, and
      trace links.
- [x] Three positive public-parser expectations and six near misses pass;
      Duvet links are independently reviewed.
- [x] Focused/full tests, all three Duvet queries/full trace, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
