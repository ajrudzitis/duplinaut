# TUI final-diagnostic size acceptance plan

Status: accepted. Local portable TUI tests, Duvet trace, formatting, and strict
Clippy passed. The Linux-gated diagnostic-cap test, independent review, hosted
Linux assurance (`cargo xtask ci`), and Duvet Pages publication passed in [run
41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167).
This acceptance remains independent of the active-restore PTY cancellation test
and encrypted filesystem restore test.

## Goal and scope

Check that rendering a typed restore outcome into the final diagnostic buffer
obeys the configured byte cap and fails closed instead of returning partial
identity or receipt text. Keep the production cap at 1 MiB. Use a small
test-local cap to exercise the exact boundary without allocating a 1 MiB
fixture.

This acceptance traces to [RESTORE-UI-011](parsing-requirements.md#restore-ui-011)
(bounded preview and receipt metadata before allocation) and
[RESTORE-UI-010](parsing-requirements.md#restore-ui-010) (complete,
terminal-safe identity and warning fields). It verifies this diagnostic
rendering boundary only; it does not establish a global allocation bound.

## Acceptance requirements

1. Keep the production `diagnostics(outcome)` entry point using
   `DIAGNOSTIC_BYTES` (1 MiB). Factor only the internal renderer enough to
   accept a test-local limit; do not expose a user-configurable cap or change
   the production limit.
2. Build a deterministic synthetic typed outcome with byte-valued destination
   and staging names that exercise escaping. Determine its complete rendered
   byte length `N` from the successful output, then rerender with cap `N` and
   require success with exactly the same complete text, including all identity,
   cleanup, and outcome fields. The test must not rely on a truncated preview
   or estimate the expected output length.
3. Rerender that same outcome with cap `N - 1` and require exactly
   `Err(Error::Limit)`. No partial string may be returned or accepted as a
   diagnostic. Include an assertion that escaped identity bytes remain
   distinct and terminal-safe in the successful exact-fit output.
4. Exercise a typed receipt-bearing outcome and an unpublished outcome with an
   uncertain candidate, so the boundary covers both receipt fields and the
   candidate path. If one shared small-cap test can cover these independently,
   keep assertions separate per outcome; a failure for one must not be
   inferred from the other.
5. Link the production renderer and its boundary test to RESTORE-UI-011 with
   Duvet annotations. Link the test to RESTORE-UI-010 where it asserts
   escaping and preservation of complete identity. Do not broaden the claim
   to escaping of arbitrary OS error strings or every terminal presentation
   path.

## Verification and evidence

- In the hosted Linux run, the focused diagnostic-cap test and full
  `cargo test -p duplicity-tui` suite passed. The new boundary test is
  Linux-gated and was not run on macOS.
- Targeted Duvet queries for RESTORE-UI-010 and RESTORE-UI-011,
  `cargo xtask trace-update`, `cargo xtask trace`, workspace formatting, strict
  TUI Clippy passed locally.
- Independent review must confirm production still uses the fixed 1 MiB cap,
  exact-fit output is complete, one-byte overflow returns `Error::Limit`, and
  no API returns or exposes partial diagnostics on failure.
- Hosted Linux assurance and Duvet Pages publication passed in [run
  41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167).
  The hosted Linux assurance workflow ran `cargo xtask ci`.

## Exclusions

This acceptance does not prove the 1 MiB limit is a global process or allocator
bound, bound external error construction or renderer internals, exercise a
live PTY or terminal output failure, establish restore lifecycle/controller
correctness, or cover encrypted credentials, S3, filesystem publication, or
cryptographic behavior. It must not edit the active PTY cancellation test or
the encrypted restore implementation/test files.

## Checklist

- [x] Add the test-local diagnostic cap seam without changing the fixed
      production limit.
- [x] Add exact-fit and one-byte-overflow tests for typed receipt and
      uncertain-candidate diagnostics; link implementation/test to Duvet.
- [x] Portable TUI tests, Duvet trace, formatting, and strict Clippy pass
      locally; the Linux-gated boundary test and hosted assurance
      (`cargo xtask ci`) pass in the hosted Linux run.
- [x] Independent review and hosted Linux assurance pass.
