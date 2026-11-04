# TUI restore retry-lockout acceptance plan

Status: proposed; specification only. This tests the frontend guard that
prevents another restore after an outcome that is published or uncertain.

## Goal and scope

Verify that a subsequent `r` key cannot enqueue a new preview request after a
`Published`, `Unpublished` with an uncertain outer candidate, or
`PublicationIndeterminate` result. The test uses synthetic typed results and
the production browser key-transition path. It does not invoke the filesystem
backend.

This acceptance traces to [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(preserve the distinct outcome classes), [RESTORE-UI-009](parsing-requirements.md#restore-ui-009)
(no retry or modifying recovery after publication uncertainty), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (separate state-machine
evidence from native backend and real-terminal evidence). Do not link
RESTORE-UI-007: this case does not test cancellation.

## Deterministic test arrangement

1. Add a focused TUI unit test that invokes both the production
   `Reply::Outcome` transition and the production browser transition used for
   the `r` key. If needed, extract these into small internal helpers over
   browser state; the production loop and test MUST call those same helpers.
2. Construct one synthetic value for each blocked outcome: a `Published`
   receipt, an `Unpublished` outcome with `uncertain_outer: Some(...)`, and a
   `PublicationIndeterminate` receipt. Wrap each in the matching successful
   `Reply::Outcome` and pass it through the production result transition so it
   sets retry policy, typed outcome, and diagnostic document. Then submit a
   Press `r` event while a snapshot and destination are available. Use a
   recording command sender rather than a live backend.
3. Keep each case isolated with a fresh browser state. Do not exercise the
   ordinary definitely-unpublished retry path or unavailable-worker path in
   this acceptance; they have separate policy and test coverage.

## MUST-level acceptance requirements

1. For each of the three blocked outcomes, require `retry` to be disabled and
   require the `r` event to return the existing disabled-status text without
   sending `Command::Prepare` or `Command::Execute`.
2. Require the typed outcome and its accessible diagnostic document to remain
   retained after the rejected key, including the receipt or uncertain
   candidate evidence belonging to that case. Do not convert one outcome
   variant into another or infer cleanup/publication facts.
3. Require the only observed command sequence to contain no command after the
   synthetic result. No restore plan, second confirmation, backend call,
   destination inspection, retry, rollback, or modifying cleanup may occur.
4. Duvet-link the test to RESTORE-UI-008, -009, and -014. Test annotations MUST
   identify this as a browser outcome/key-transition test using synthetic
   replies; they MUST NOT imply native backend or real-terminal evidence.

## Verification and evidence

- Run the focused test and full `cargo test -p duplicity-tui` on Linux; run
  portable tests on macOS as available.
- Run targeted Duvet queries for RESTORE-UI-008, -009, and -014, then
  `cargo xtask trace-update` and `cargo xtask trace`. Run formatting and strict
  TUI Clippy.
- Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages publication,
  and independent implementation review before acceptance.
- Review must confirm the test exercises the production `r` transition for
  all three typed outcome variants after the production `Reply::Outcome`
  transition, preserves the original result document, and records no queued
  command.

## Exclusions

This acceptance does not exercise the native filesystem backend, real PTY or
terminal presentation, actual restoration or publication, filesystem
inspection/cleanup, worker-unavailable behavior, ordinary definitely
unpublished retries, cancellation timing, S3, cryptographic behavior, or
formal frontend-controller proof. Synthetic outcomes do not prove backend
publication behavior.

## Checklist

- [ ] Exercise the production browser `r` transition with Published,
      uncertain Unpublished, and PublicationIndeterminate results.
- [ ] Assert no command is sent and the typed result/document remain retained.
- [ ] Focused/full TUI tests, Duvet trace, formatting, and strict Clippy pass;
      hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
