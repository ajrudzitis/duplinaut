# TUI close-only read-only-scan output-failure acceptance plan

Status: implementation, macOS portable test, and hosted Linux assurance are
complete; Duvet Pages publication and independent review remain pending. This
complements the accepted q-triggered read-only-scan output-failure case by
proving that a closed stdout, with no keypress, drives cancellation and
controlled shutdown on its own.

## Goal and scope

Exercise the production TUI when stdout fails during a held read-only snapshot
scan and the user sends **no key**. Closing only the output PTY master makes the
next periodic redraw fail; that render error, not a `q` or Esc key event,
requests cancellation of the exact held scan token, closes the worker request
path, attempts terminal restoration, drains and joins the worker, and returns
the typed `Source(Cancelled)` read failure.

This acceptance traces to [RESTORE-UI-012](parsing-requirements.md#restore-ui-012)
(controlled shutdown ordering) and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(real terminal evidence recorded separately). It reuses the accepted
`gated_output_failure_browser_helper` driver and the source-injected
`GatedLocalSource` scan gate from the [q-triggered case](tui-output-failure-pty-acceptance.md).

## Failure sequence and acceptance requirements

1. Drive the production browser through full/target selection and the explicit
   scan confirmation, then wait for the held scan gate to publish `gate-entered`.
2. Resize the output slave to a compact geometry and send a direct child
   `SIGWINCH`, proving the child derives its terminal geometry from the output
   PTY, and wait for a complete compact frame.
3. Close only the output PTY master and send no key. The event loop redraws
   every iteration with a 50 ms poll timeout, so the next redraw fails through
   the closed stdout. Require the exact scan token to become cancelled while the
   gate remains held, with no key event on the input PTY.
4. Keep the helper alive until the parent releases the gate, then require the
   production browser to return its controlled error. On Linux the error
   downcasts to `restore_flow::ShutdownFailure` with a retained terminal event
   error; on other Unix hosts it downcasts to `terminal::Completion` with an
   event fault. Require `raw_failed == false` and `screen_failed == true` and
   `cursor_failed == true` (raw mode is restored through the live input PTY;
   screen/cursor restoration fails through the closed stdout).
5. Verify input PTY termios is restored, the source fixture fingerprint is
   unchanged, the browser working directory is empty, and the held scan gate
   observed entered/cancelled/released with no gate failure.

## Verification and limits

- Duvet-link the default parent test to RESTORE-UI-012 and -014. Annotations
  MUST say that closing stdout alone initiates cancellation and controlled
  shutdown with no key event; they MUST NOT claim cancellation latency or
  interruptible syscalls.
- Run the focused and full `cargo test -p duplicity-tui` on both macOS and
  Linux. The read-only-scan harness is portable across Unix hosts; the active
  restore outcome type is not involved.
- Run targeted Duvet queries, `cargo xtask trace`, formatting, and strict
  Clippy. Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages
  publication, and independent implementation review before acceptance.

## Exclusions

This test does not cover active restoration output failure, typed restore
outcomes, input PTY failure, both stdout and stderr unavailable, successful
screen/cursor restoration through a closed output, fatal signals, crashes,
fixed cancellation latency, syscall interruption, S3, or formal controller
proofs. The scan cancellation wait is cooperative; no fixed cancellation
latency is claimed.

## Checklist

- [x] Specify and implement the close-only read-only-scan case using the
      existing split-PTY harness and scan gate, sending no key event.
- [x] Write the parent test asserting exact-token cancellation, retained event
      fault, teardown fault fields, terminal restoration, and source/working
      directory invariants; the macOS portable test passes.
- [x] Hosted Linux assurance (`cargo xtask ci`) passes in [run 43564648218](https://github.com/ajrudzitis/duplinaut/actions/runs/43564648218);
      Duvet Pages publication runs only on push to `main` and remains pending.
- [ ] Independent implementation review accepts the scoped evidence.
