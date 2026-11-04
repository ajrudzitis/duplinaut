# PTY output failure acceptance plan

Status: independently reviewed and adopted before implementation. The plan was
amended after close-only output did not trigger a redraw; implementation is
pending.

This records one controlled terminal output failure while a read-only scan is
active. It exercises the production browser and terminal lifecycle for
RESTORE-UI-014. It does not claim active filesystem restoration coverage for
RESTORE-UI-012 or any publication outcome guarantee.

## Process and source

Use the explicitly configured, ignored libtest child-driver pattern from the
[scan cancellation plan](tui-scan-cancel-pty-acceptance.md). A default parent
test starts the production `browser::run` in a child process with separate PTYs
for stdin and stdout, and a captured stderr pipe. Before browser startup or any
other terminal operation, the child driver calls safe
`nix::unistd::setsid()` (enable the `process` feature for the TUI test
dependency). The spawned child's PID is not its inherited process-group ID,
so it can create a new session without an unsafe `pre_exec` hook. Require
`setsid` success and have the child explicitly verify that opening `/dev/tty`
fails while stdin remains a TTY. This removes the inherited test-runner
controlling terminal. Pinned Crossterm 0.29's Unix `window_size()` opens
`/dev/tty` first and falls back to stdout when that open fails, on both macOS
and Linux; this child-side check establishes that the output PTY is the size
source. Before spawning the child, the parent must set and verify `FD_CLOEXEC`
on both PTY master descriptors. This ensures the child cannot inherit a hidden
master that would keep either PTY open after the parent closes its descriptor.
Keep the official 3.0.7 plain fixture and a gated identity-checked first-volume
open. The exact scan token must be uncancelled at gate entry; the driver never
cancels it.

The child reports bounded, fixed-field completion data through the separate
IPC directory only after `browser::run` returns. It records whether the
structured result contains an event error, teardown attempts, and worker
shutdown completion. Never put source paths or credentials in diagnostics.

## Failure stimulus and sequence

1. Wait for the actual history screen, select the full anchor and final
   incremental target, accept the first-Enter scan disclosure, and send the
   second Enter. Wait for the source gate and visible scan-running state.
2. Use the two distinct PTYs created at matching expanded dimensions (for
   example, 40 rows by 240 columns). The child is in its own session with no
   controlling terminal, but stdin remains attached to its input PTY for key
   input and raw-mode lifecycle. With the source gate held, resize the output
   slave to a distinct, nonzero compact geometry using
   `rustix::termios::tcsetwinsize`; read it back with `tcgetwinsize`. Send
   `SIGWINCH` directly to the positive child PID using the existing safe
   `nix::sys::signal::kill` pattern. Do not rely on automatic foreground
   process-group signal delivery. Update the parent's bounded VT screen model
   to the compact size and wait until the model reports exactly the compact
   row and column counts and shows a complete frame (both top and bottom
   borders reach the model's exact right and bottom edges) while the output
   master remains open. A partial old buffer, parser-only resize or stale
   expanded frame does not satisfy this check. This proves the child observes
   the output PTY resize and completes a full frame before the fault.
3. Close only the parent’s stdout PTY master, retaining the stdout slave
   descriptor used by the child and the stdin master/slave pair. While the
   output gate remains held, restore the retained output slave to the original
   expanded dimensions using safe rustix; require `tcsetwinsize` to succeed
   and `tcgetwinsize` to read back the expanded dimensions after master close.
   This post-close ioctl is a required macOS/Linux feasibility check, not an
   assumed PTY property. If either target platform rejects it, stop and design
   another deterministic changed-frame trigger before implementing this case.
   Send `SIGWINCH` directly to the same positive child PID only after the
   post-close size change. The child has no controlling terminal, so the
   harness does not rely on automatic foreground-group signal delivery.
   Crossterm's resize query falls back to stdout. Ratatui checks backend size
   on every `draw` and resets the previous buffer after a viewport change, so
   the next production draw must write a full frame. Because the geometry is
   changed only after closing the sole output reader, there is no opportunity
   for a successful expanded redraw before the fault; the full frame must fail
   through the real terminal writer. This plan claims a controlled
   resize-triggered redraw failure only; it does not claim that close alone
   fails, that resize notification is automatic, or that all output failures
   behave identically.
4. Require the real scan token to become cancelled while the source gate stays
   blocked. This establishes that the terminal-loop redraw failure, rather
   than the test driver or a normal quit key, initiated shutdown. Require the
   child to remain alive until the parent releases the gate.
5. Release the gate and require bounded child exit with its expected controlled
   browser error. In the helper, downcast the returned error payload to
   `terminal::Completion` and require `event.is_some()` and `teardown.is_some()`.
   Downcast the teardown error payload to `terminal::TeardownFaults`; record
   explicit `raw_failed`, `screen_failed`, and `cursor_failed` booleans. Raw
   mode is restored through the live input PTY and is expected to have no fault;
   alternate-screen and cursor output faults are expected on the closed output
   PTY. The marker records attempts and observed faults, without claiming
   alternate-screen or cursor restoration succeeded. Publish the completion
   marker only after `browser::run` returned, which follows the production
   worker close/drain/join path.
6. The parent has closed the output PTY master, so libtest cannot safely print
   its normal success line afterward. Once the helper has validated `Completion`
   and atomically published the completion marker, it calls `process::exit(0)`.
   This is an internal ignored-driver harness detail. Acceptance requires both
   the marker contents and successful child exit; an earlier exit cannot pass.
7. Read the live stdin PTY termios and confirm its original attributes were
   restored. The separate output master has intentionally been closed, so do
   not claim that alternate-screen teardown reached a terminal. Require no
   filesystem entries in the browser working directory and unchanged final
   fixture names/content/symlink targets.

Use bounded output and IPC sizes, explicit deadlines in both processes,
atomic phase markers, child kill/reap cleanup, and unique owned temporary
directories. A missing event error, already-cancelled token, gate timeout,
IPC error, or fabricated completion marker must fail the parent test.

## Evidence limits

This case covers a controlled redraw/output failure during a source-injected
read-only scan. It does not cover active restoration cancellation, typed restore
outcomes, successful restoration reporting, alternate-screen restoration over
the failed output, input failure, both stdout and stderr unavailable, real
controlling-terminal job control, arbitrary faults, live S3, or frontend formal
controller proofs. The source cancellation wait is cooperative; no fixed
cancellation latency or interruption of a real blocking operating-system call
is claimed.

Record focused Unix tests, independent review, Duvet trace, and full assurance
CI before adoption. The ignored child helper carries no separate Duvet or
assurance-gate claim; only default parent tests count as the acceptance cases.
