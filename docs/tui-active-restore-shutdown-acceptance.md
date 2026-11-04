# TUI active-restore controlled-shutdown acceptance plan

Status: accepted. The Linux-gated compositional test, hosted Linux assurance,
and Duvet Pages passed in [run 41851231765](https://github.com/ajrudzitis/duplinaut/actions/runs/41851231765).

## Goal and scope

Verify that a controlled terminal event-loop failure while an Execute operation
is active cancels the exact request token, closes the worker request path,
attempts terminal restoration, drains and joins the worker, retains a known
typed outcome, and records a failed fallback output separately. Use a
test-only worker gate and a synthetic `Unpublished(Cancelled)` reply. Do not
call the filesystem backend in this test.

This acceptance traces to [RESTORE-UI-012](parsing-requirements.md#restore-ui-012)
(controlled shutdown ordering and result retention),
[RESTORE-UI-008](parsing-requirements.md#restore-ui-008) (typed outcome
preservation), and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(separate recording of worker and terminal evidence). Also link
[RESTORE-UI-007](parsing-requirements.md#restore-ui-007) because the test
asserts cancellation on the exact Execute token.

## Deterministic test arrangement

1. Add a Linux-gated unit test in the `duplicity_tui` test binary, adjacent to
   the worker shutdown tests or in a focused TUI test module. Use capacity-one
   request/reply channels and a worker thread that receives a real-shaped
   `Command::Execute` with a frozen request prepared from the checked-in
   duplicity 3.0.7 plain fixture. The test driver MUST NOT call the native
   filesystem backend.
2. At Execute entry, the driver MUST capture the operation ID and clone that
   command's exact cancellation token, require it to be initially clear, then
   publish an `execute-entered` synchronization marker and block. The marker
   must be bounded and phase-specific; use channel synchronization rather than
   sleeps.
3. Invoke `terminal::interact` with a test `Lifecycle`. Its event-loop closure
   MUST wait for `execute-entered`, then return a distinct injected terminal
   event I/O error while the worker gate is still held. Add a test-only close
   observer that publishes `close-observed` only after cancellation and request
   sender closure. The lifecycle restore method MUST wait for this marker,
   verify the exact Execute token is cancelled, record that it was called, and
   return a distinct injected teardown error.
4. The test MUST assert this ordered synchronization sequence without sleeps:
   `execute-entered`; event-loop error; `close-observed` with the exact token
   cancelled and sender closed; lifecycle restoration attempt/error; gate
   release; synthetic result publication; `worker::finish` reply drain and
   worker join. After release, the worker driver publishes one synthetic
   `Reply::Outcome` for the same operation ID containing exactly
   `RestoreOutcome::Unpublished` with `Cause::Cancelled` and no uncertain outer
   candidate, then exits after observing request closure.

## Acceptance requirements

1. Pass the resulting `Completion` to the production `worker::finish` path
   with a sink that returns a distinct injected write error. Require `finish`
   to drain the typed outcome and join the worker before returning an
   `io::Error` whose payload downcasts to `restore_flow::ShutdownFailure`.
2. Require the downcast result to preserve the injected terminal event error,
   teardown-attempt/error fields, and fallback-output error separately. Its
   `outcome` MUST be `Some(Unpublished(Cancelled))`, and `unavailable` MUST be
   false. The test MUST NOT report the known result as unavailable or replace
   it with a fabricated terminal/backend error. Verify the operation ID is
   retained where present.
3. Require the worker to be joined and the operation-specific token to remain
   cancelled. The synthetic result MUST be the only worker outcome; no second
   Execute may be queued. Do not infer filesystem absence, staging cleanup,
   or successful backend cancellation because the native backend is not
   entered.
4. Duvet-link the implementation/test to RESTORE-UI-012, -008, and -014, and
   to -007 for the exact-token assertion. Test descriptions MUST state that
   the event failure and backend result are injected at the worker/lifecycle
   boundary.

## Verification and evidence

- Run the focused Linux test and full `cargo test -p duplicity-tui` on Linux.
  On macOS, run only portable TUI tests; this acceptance test is Linux-gated
  because it constructs the native restore outcome type.
- Run targeted Duvet queries for RESTORE-UI-007, -008, -012, and -014, then run
  `cargo xtask trace-update` and `cargo xtask trace`.
- Run formatting and strict TUI Clippy locally. Require the hosted Linux
  assurance workflow (`cargo xtask ci`) and Duvet Pages publication before
  acceptance.
- Independent review must check that the failure occurs with Execute active,
  the same token is cancelled, request closure precedes restoration and
  worker drain/join, the typed synthetic outcome survives, and event/teardown/
  fallback-output faults remain distinct.

## Exclusions

This acceptance does not exercise a real PTY or actual terminal output failure,
the native filesystem backend, cancellation during a syscall, fixed
cancellation latency, publication races, filesystem cleanup or restoration,
S3, encrypted credentials, cryptographic behavior, or frontend controller
proofs. It does not claim that the synthetic `Unpublished(Cancelled)` outcome
was produced by the real backend.

## Checklist

- [x] Add a deterministic Execute gate and lifecycle-error test in test-only
      code; production behavior remains unchanged.
- [x] Assert exact-token cancellation, request closure, restoration attempt,
      retained typed outcome, separate error fields, and joined worker.
- [x] Focused/full Linux TUI tests, Duvet trace, formatting, and strict Clippy
      pass; hosted Linux assurance and Duvet Pages pass in [run 41851231765](https://github.com/ajrudzitis/duplinaut/actions/runs/41851231765).
- [x] Independent implementation review accepts the scoped test evidence.

The evidence is limited to the injected worker/lifecycle composition described
above. It does not establish behavior from a real filesystem backend or a real
PTY/output failure.
