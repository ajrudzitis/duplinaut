# TUI both-output-sinks output-failure acceptance plan

Status: implementation and hosted Linux assurance are complete; Duvet Pages
publication and independent review remain pending. This complements the accepted
active-restore output-failure case by making the fallback output sink fail too,
proving the typed result is still retained.

## Goal and scope

Exercise the production TUI when stdout fails during an active `Execute` and the
stderr fallback sink is also unwritable. Closing the stdout PTY master makes the
next redraw fail; `q` through the live input PTY cancels the exact Execute
token. The frontend requests cancellation, closes the worker request path,
attempts terminal restoration, drains and joins the worker, retains the typed
`Unpublished(Cancelled)` result, and attempts the escaped fallback report
through stderr — which is `/dev/full`, so that write also fails. The result is
still preserved in the typed `ShutdownFailure`, not converted to unavailable or
publication-indeterminate.

This acceptance traces to [RESTORE-UI-012](parsing-requirements.md#restore-ui-012)
(controlled shutdown, result retention, and separate output fault recording),
[RESTORE-UI-007](parsing-requirements.md#restore-ui-007) (exact token
cancellation), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008) (typed
result distinction), and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(real terminal evidence recorded separately).

## Process and terminal setup

Use the same split-PTY arrangement as the
[active-restore output-failure case](tui-active-restore-output-failure-acceptance.md):
separate input and output PTYs, `FD_CLOEXEC` on the parent masters, a child-side
`safe nix::unistd::setsid()`, and an assertion that `/dev/tty` cannot be opened
while stdin remains a TTY. Instead of a captured stderr pipe, the child's stderr
is `Stdio::from(File::open("/dev/full"))`, so every fallback write returns
`ENOSPC`. Keep the official plain fixture, a test-owned destination, bounded IPC
markers, deadlines, and the existing exact-Execute worker gate.

## Failure sequence and acceptance requirements

1. Drive the production browser through full/target selection, snapshot scan,
   executable preview, exact `restore` acknowledgment, and one Enter. Require
   both visible `Restoration running` state and `execute-entered`.
2. Close the stdout PTY master and send one `q` through the live input PTY.
   Require the same Execute token to become cancelled while the gate remains
   held.
3. Keep the helper alive until the parent releases the gate, then require the
   real backend call with the now-cancelled token to return exactly
   `Unpublished(Cancelled)` with no uncertain outer candidate. Require the
   requested destination to remain absent and no `.duplicity-*` staging entry;
   the source fixture fingerprint MUST be unchanged.
4. Require controlled helper completion whose error downcasts to
   `restore_flow::ShutdownFailure`, with a terminal event error, attempted
   teardown, `outcome == Some(Unpublished(Cancelled))`, `unavailable == false`,
   and `output == Some(_)` because the `/dev/full` fallback write failed. Do not
   transform the typed result into unavailable or publication-indeterminate.
5. Downcast teardown details: raw mode MUST be restored through the live input
   PTY; screen/cursor restoration writes may fail through the closed stdout PTY.
   Verify input PTY termios matches its original settings after completion.
6. The helper publishes its bounded marker through the control directory and
   exits without touching the broken stderr sink; the parent validates both
   marker and child status.

## Duvet and verification

- Duvet-link the default parent test to RESTORE-UI-012, -007, -008, and -014.
  Annotations MUST say the fallback output also failed and the typed result was
  retained; they MUST NOT claim cancellation latency or interruptible syscalls.
- Run the focused Linux PTY case and full `cargo test -p duplicity-tui` on
  Linux. macOS may run portable TUI tests but does not count as this Linux-gated
  native restoration test.
- Run targeted Duvet queries, `cargo xtask trace`, formatting, and strict TUI
  Clippy. Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages
  publication, and independent implementation review before acceptance.

## Exclusions

This test does not cover input PTY failure, close-only (no-key) initiation with
both sinks unavailable, successful screen/cursor restoration through a closed
output, fatal signals, crashes, fixed cancellation latency, syscall
interruption, publication races, S3, or encrypted credentials. The backend
receives the already-cancelled token before filesystem restore work begins.

Input-PTY failure is excluded because it is a trusted-boundary limitation, not a
testable controlled-shutdown case: closing the input PTY master hangs up the
child's stdin, and pinned crossterm 0.29's tty poll reports only `POLLHUP`
(never `POLLIN`) after the hangup, so its `event::poll` busy-spins and
`event::read` never returns; the browser hangs instead of returning an input
error. A real terminal close is handled by the operating system via `SIGHUP`
rather than a clean input-error path.

## Checklist

- [x] Specify and implement the both-sinks case using `/dev/full` stderr, the
      existing split-PTY harness, and the exact-Execute gate.
- [x] Write the parent test and helper asserting exact token, retained typed
      outcome, failed fallback output, teardown faults, worker join, and
      filesystem/source invariants.
- [x] Focused/full Linux tests, strict TUI Clippy and hosted Linux assurance
      (`cargo xtask ci`) pass in [run 43564648218](https://github.com/ajrudzitis/duplinaut/actions/runs/43564648218);
      Duvet Pages publication runs only on push to `main` and remains pending.
- [ ] Independent implementation review accepts the scoped evidence.
