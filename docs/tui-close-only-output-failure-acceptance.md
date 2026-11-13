# TUI close-only output-failure acceptance plan

Status: specification and implementation in place; the Linux-gated PTY case and
hosted Linux assurance remain pending. This case complements the accepted
q-triggered active-restore output-failure evidence by proving that a closed
stdout, with no keypress, drives cancellation and controlled shutdown on its
own.

## Goal and scope

Exercise the production TUI when stdout fails during an active `Execute` and
the user sends **no key**. Closing only the output PTY master makes the next
periodic redraw fail; that render error, not a `q` or Esc key event, requests
cancellation of the exact Execute token, closes the worker request path,
attempts terminal restoration, drains and joins the worker, retains the typed
backend result, and reports it through the still-writable stderr fallback.

This acceptance traces to [RESTORE-UI-012](parsing-requirements.md#restore-ui-012)
(controlled shutdown ordering and result retention), [RESTORE-UI-007](parsing-requirements.md#restore-ui-007)
(exact operation token cancellation), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(typed result distinction), and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(real terminal evidence recorded separately).

## Process and terminal setup

1. Put the parent PTY test and ignored helper in the same `duplicity_tui`
   unit-test binary. Spawn the helper as a separate process so Crossterm's
   process-global state is isolated and test-only worker hooks are available.
   Reuse the accepted `gated_restore_output_failure_browser_helper` driver: its
   assertions are identical because the close-only path reaches the same
   controlled shutdown, typed cancellation result and teardown faults.
2. Create separate PTYs for stdin and stdout, plus a captured stderr pipe.
   Before spawn, set `FD_CLOEXEC` on both parent master descriptors. In the
   child, call safe `nix::unistd::setsid()` before terminal initialization and
   assert that `/dev/tty` cannot be opened while stdin remains a TTY. This
   makes Crossterm use the stdout slave for terminal size and ensures the
   child cannot retain a hidden PTY master. Keep the input master/slave and
   output slave alive when the parent closes only the output master.
3. Use the official plain duplicity 3.0.7 fixture, a test-owned destination,
   bounded IPC markers, deadlines, and the existing exact-Execute worker gate.
   The gate MUST capture the operation ID and exact initially-clear token,
   signal `execute-entered`, block before backend entry, and observe
   cancellation without cancelling the token itself.

## Failure sequence and acceptance requirements

1. Drive the production browser through full/target selection, snapshot scan,
   executable preview, exact `restore` acknowledgment, and one Enter. Require
   both visible `Restoration running` state and `execute-entered` before
   injecting the fault.
2. Close only the parent's stdout PTY master and send no key. The browser's
   event loop redraws every iteration with a 50 ms input poll timeout, so the
   next redraw fails through the closed stdout. Require the same Execute token
   to become cancelled while the gate remains held, with no key event on the
   input PTY. Do not use a resize, post-close ioctl, signal, or key as the
   trigger; the redraw failure itself initiates cancellation.
3. Keep the helper alive until the parent releases the gate, then require the
   real backend call with the now-cancelled token to return exactly
   `Unpublished(Cancelled)` with no uncertain outer candidate. Require the
   requested destination to remain absent and the test-owned destination
   parent to contain no `.duplicity-*` staging entry. The source fixture
   fingerprint MUST be unchanged.
4. Require controlled helper completion whose error downcasts to
   `restore_flow::ShutdownFailure`, with a terminal event error, attempted
   teardown, `outcome == Some(Unpublished(Cancelled))`, and `unavailable ==
   false`, and `output == None` because fallback reporting succeeded. Require
   the worker to have been drained and joined before the completion marker.
   With stderr still writable, require fallback output to retain the
   cancellation diagnostic. Do not transform the typed result into unavailable
   or publication-indeterminate.
5. Downcast teardown details and record raw-mode, alternate-screen, and cursor
   restoration attempts/failures separately. Raw mode MUST be restored through
   the live input PTY. Screen/cursor restoration writes may fail through the
   closed stdout PTY; do not claim those writes succeeded. Verify input PTY
   termios matches its original settings after child completion.
6. Because the output PTY master is closed, suppress the ignored driver's
   libtest completion output: after validating structured completion and
   publishing its bounded marker, the helper may `process::exit(0)`. The
   parent MUST validate both marker and child status; this is test-harness
   behavior only.
7. Require no requested destination or `.duplicity-*` entry in the test-owned
   destination parent, and unchanged source object names/content/symlink
   targets. This checks only those test-owned paths; it is not a global
   filesystem scan. Use unique temporary paths, bounded stderr and IPC,
   explicit deadlines, and child kill/reap cleanup.

## Duvet and verification

- Duvet-link the default parent test (not the ignored helper) to RESTORE-UI-012,
  -007, -008, and -014. Annotations MUST say that closing stdout alone
  initiates cancellation and controlled shutdown with no key event; they MUST
  NOT claim cancellation latency or interruptible syscalls.
- Run the focused Linux PTY case and full `cargo test -p duplicity-tui` on
  Linux. macOS may run portable TUI tests but does not count as this
  Linux-gated native restoration test.
- Run targeted Duvet queries, `cargo xtask trace-update`, `cargo xtask trace`,
  formatting, and strict TUI Clippy. Require hosted Linux assurance
  (`cargo xtask ci`), Duvet Pages publication, and independent implementation
  review before acceptance.

## Exclusions

This test does not cover input PTY failure, both stdout and stderr unavailable,
successful screen/cursor restoration through a closed output, fatal signals,
crashes, fixed cancellation latency, syscall interruption, publication races,
S3, or encrypted credentials. The backend receives the already-cancelled token
before filesystem restore work begins; no claim is made about cancellation
during an active filesystem syscall.

## Checklist

- [x] Specify and implement the close-only output-failure case using the
      existing split-PTY harness and Execute gate, sending no key event.
- [x] Write the parent test asserting exact token, typed outcome retention,
      controlled teardown, fallback reporting, worker join, and
      filesystem/source invariants.
- [ ] Focused/full Linux tests, strict TUI Clippy, hosted Linux assurance and
      Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
