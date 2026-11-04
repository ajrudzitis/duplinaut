# TUI unavailable-result diagnostic acceptance plan

Status: proposed; specification only. This complements the accepted
[active-restore shutdown case](tui-active-restore-shutdown-acceptance.md) by
checking successful diagnostic delivery when the worker result is unavailable.

## Goal and scope

Verify that a controlled event-loop failure during an active Execute, followed
by worker termination without a final reply, is reported as unavailable with
the configured operation and destination identity. The test uses a synthetic
worker driver and never enters the native filesystem backend.

This acceptance traces to [RESTORE-UI-012](parsing-requirements.md#restore-ui-012)
(worker result loss and final reporting), [RESTORE-UI-009](parsing-requirements.md#restore-ui-009)
(no retry or modifying recovery after an unavailable result),
[RESTORE-UI-010](parsing-requirements.md#restore-ui-010) (complete reversible
escaping of identity and diagnostics), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(do not fabricate a typed outcome), and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(record worker/result integration separately from backend evidence). Link
[RESTORE-UI-007](parsing-requirements.md#restore-ui-007) only for the exact
Execute-token cancellation assertion.

## Deterministic test arrangement

1. Add a Linux-gated unit test in `crates/tui/src/worker.rs`, in the existing
   `duplicity_tui` test binary. Use a real-shaped `Command::Execute` with the
   frozen request prepared from the checked-in plain 3.0.7 fixture. A test-only
   worker driver captures its operation ID and exact initially-clear token,
   publishes a bounded `execute-entered` marker, and waits at a gate. It must
   not call the native backend.
2. Run `terminal::interact` with an event-loop closure that waits for that
   marker and returns a distinct injected event I/O error. Use the test-only
   close observer to establish exact-token cancellation and request-sender
   closure before the lifecycle restoration attempt. The lifecycle records
   restoration and injects a distinct teardown error.
3. After restoration, release the gate. The worker driver verifies the same
   token is cancelled, publishes no `Reply::Outcome`, and terminates by an
   injected panic to model loss before a final result. Use synchronization
   markers and bounded waits, not sleeps.

## MUST-level acceptance requirements

1. Call production `worker::finish` with the resulting `Completion`, the
   operation ID, an `Arc<Destination>` containing test-local raw non-UTF-8 and
   control bytes in the parent path and leaf, and a successful in-memory
   `Vec<u8>` reporting sink. The destination need not exist and must never be
   inspected or modified.
2. Require the returned I/O error to downcast to `restore_flow::ShutdownFailure`.
   Assert `outcome == None`, `unavailable == true`, worker shutdown error is
   present, the operation ID and requested destination are retained exactly,
   terminal and teardown errors are present, and `output == None` because
   fallback reporting succeeded. Require the exact token to remain cancelled
   and the test driver to have terminated before `finish` returns.
3. Require the sink text to identify “Restoration outcome unavailable;
   inspection required,” include the operation ID, requested parent and leaf
   in the documented `\\`/`\\xNN` escaping, and state that no staging or
   publication absence is inferred. Assert raw control bytes, including ESC,
   are absent from the output. Do not report `Cancelled`, `Unpublished`, or any
   other backend outcome.
4. Assert that only one Execute was submitted and no retry, recovery, or
   modifying cleanup was attempted. The injected worker panic is explicitly a
   test harness event, not evidence about native backend failure behavior.
5. Duvet-link the test to RESTORE-UI-012, -009, -010, -008, and -014; include
   RESTORE-UI-007 only if the exact-token assertion remains in the test.

## Verification and evidence

- Run the focused Linux test and full `cargo test -p duplicity-tui` on Linux.
  macOS runs may cover portable tests but do not count as execution of this
  Linux-gated case.
- Run targeted Duvet queries for the linked requirements, then
  `cargo xtask trace-update` and `cargo xtask trace`; run formatting and strict
  TUI Clippy.
- Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages publication,
  and independent implementation review before checking this acceptance off.
- Review must confirm the report uses complete original destination bytes,
  worker loss cannot become a fabricated typed outcome, and the operation is
  neither retried nor modified after unavailability.

## Exclusions

This case does not exercise a real backend panic, real filesystem restoration,
destination/staging inspection, cleanup, publication uncertainty, actual PTY
or terminal output failure, S3, encrypted credentials, fixed cancellation
latency, syscall interruption, or formal frontend-controller proof. The
synthetic worker driver only tests frontend handling of a missing final reply.

## Checklist

- [ ] Add a deterministic active-Execute worker-result-loss test in test-only
      code; native backend behavior remains outside scope.
- [ ] Assert typed unavailability, exact operation/destination retention,
      successful escaped report, no inferred absence, and no retry or cleanup.
- [ ] Focused/full Linux TUI tests, Duvet trace, formatting, and strict Clippy
      pass; hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
