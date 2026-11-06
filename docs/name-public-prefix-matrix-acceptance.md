# Public `parse_name` Configured-Prefix Matrix Acceptance

**Status:** Specification and implementation/test independently reviewed. The
12-row public matrix, 24 prefix near-misses, Duvet links, formatting, strict
Clippy, full workspace tests, and local full `cargo xtask ci` pass. Hosted Linux
assurance and Duvet Pages publication remain pending.

## Goal and scope

Exercise the public `parse_name` entry point with nonempty, distinct configured
prefixes across all six supported object kinds and both filename profiles. The
acceptance oracle is twelve independently specified `ObjectName` results, plus
one-byte near misses for the global and selected kind prefix of each input.

This covers the composition of configured prefixes with supported name
classification, and literal prefix ordering and kind-to-prefix selection. It
does not establish raw filename grammar completeness or parser-controller
formal refinement.

## Fixed configuration and oracle

Use one valid `NameConfig` throughout:

| Field | Bytes |
| --- | --- |
| global prefix | `root-` |
| archive prefix | `archive+` |
| manifest prefix | `manifest?` |
| signature prefix | `signature!` |
| time separator | `:` |
| `max_name_bytes` | 4096 |

All prefixes are nonempty and pairwise distinct; none contains `/` or NUL. For
each row, the input is `root-` followed by the selected kind prefix and listed
stem. Expected values below are literal test oracle data and MUST NOT be
computed by another parser/helper. All results have `Envelope::Plain` and
`partial == false`.

| Kind | Profile | Stem | Start | End | Volume |
| --- | --- | --- | ---: | ---: | ---: |
| FullVolume | long | `duplicity-full.19700101T000000Z.vol42.difftar` | 0 | 0 | `Some(42)` |
| FullVolume | short | `df.1.10.dt` | 1 | 1 | `Some(36)` |
| IncrementalVolume | long | `duplicity-inc.19700101T000000Z.to.19700101T000001Z.vol42.difftar` | 0 | 1 | `Some(42)` |
| IncrementalVolume | short | `di.1.2.10.dt` | 1 | 2 | `Some(36)` |
| FullManifest | long | `duplicity-full.19700101T000000Z.manifest` | 0 | 0 | `None` |
| FullManifest | short | `df.1.m` | 1 | 1 | `None` |
| IncrementalManifest | long | `duplicity-inc.19700101T000000Z.to.19700101T000001Z.manifest` | 0 | 1 | `None` |
| IncrementalManifest | short | `di.1.2.m` | 1 | 2 | `None` |
| FullSignatures | long | `duplicity-full-signatures.19700101T000000Z.sigtar` | 0 | 0 | `None` |
| FullSignatures | short | `dfs.1.st` | 1 | 1 | `None` |
| IncrementalSignatures | long | `duplicity-new-signatures.19700101T000000Z.to.19700101T000001Z.sigtar` | 0 | 1 | `None` |
| IncrementalSignatures | short | `dns.1.2.st` | 1 | 2 | `None` |

Use `archive+` only for volume kinds, `manifest?` only for manifest kinds,
and `signature!` only for signature kinds. Assert every `ObjectName` field:
kind, `short`, start/end, volume, envelope, and partial.

## Requirements

1. MUST test all twelve rows through public `parse_name` with the same
   configured prefixes and assert the independent expected values above.
   Annotate positive classification coverage with `PARSE-NAME-001`.
2. MUST also annotate the twelve positive cases under `PARSE-NAME-002`, since
   they exercise literal global-then-kind composition and kind-to-prefix
   selection.
3. MUST prove literal global-then-kind prefix composition for every row with
   two near misses. First change only the global prefix's first byte from `r`
   to `x`, retaining the selected kind prefix and stem. Then restore the global
   prefix and change only the selected kind prefix's first byte from `a`, `m`,
   or `s` to `x`. Both inputs MUST return `Ok(None)`; no other byte may change.
   Annotate these cases with `PARSE-NAME-002`.
4. MUST exercise the public entry point; body parsers, candidate helpers, or a
   helper-composed expected result alone do not satisfy acceptance.
5. MUST keep the expected result table independent of parser implementation
   helpers, preventing the oracle from duplicating parser/configuration bugs.
6. MUST link positive test evidence to both `PARSE-NAME-001` and
   `PARSE-NAME-002`, and near-miss assertions to `PARSE-NAME-002` in Duvet.
   The trace report MUST show these links.

## Verification and review

- Run the focused public-prefix integration test and full core test suite.
- Run focused Duvet queries for `PARSE-NAME-001` and `PARSE-NAME-002`, then the
  complete trace check; run formatting, strict Clippy, and `cargo xtask ci`.
- Require hosted Linux assurance and Duvet Pages publication before acceptance.
- Independent review MUST verify all twelve literal expectations, intended
  prefixes, and that each near miss changes exactly one byte and yields
  `Ok(None)`.

## Exclusions

This acceptance does not claim malformed-input/error completeness, a raw
filename-grammar iff theorem, ambiguity reachability or uniqueness, body-parser
proofs, or complete `parse_name` controller refinement. It adds no Verus
obligation; those broader assurance items remain open.

## Checklist

- [x] Independent specification review accepts the bounded matrix and oracle.
- [x] Twelve public parser expectations and twenty-four one-byte near misses
      pass; Duvet links are independently reviewed.
- [x] Focused/full tests, Duvet queries/trace, formatting, strict Clippy, and
      local `cargo xtask ci` pass.
- [x] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
