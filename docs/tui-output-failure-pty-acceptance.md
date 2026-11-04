# PTY output failure acceptance plan

Status: proposed before implementation; independent plan review and adoption are pending.

This records one controlled terminal output failure while a read-only scan is
active. It exercises the production browser and terminal lifecycle for
RESTORE-UI-014. It does not claim active filesystem restoration coverage for
RESTORE-UI-012 or any publication outcome guarantee.

## Process and source

Use the explicitly configured, ignored libtest child-driver pattern from the
[scan cancellation plan](tui-scan-cancel-pty-acceptance.md). A default parent
test starts the production `browser::run` in a child process with separate PTYs
for stdin and stdout, and a captured stderr pipe. Keep the official 3.0.7 plain
fixture and a gated identity-checked first-volume open. The exact scan token
must be uncancelled at gate entry; the driver never cancels it.

The child reports bounded, fixed-field completion data through the separate
IPC directory only after `browser::run` returns. It records whether the
structured result contains an event error, teardown attempts, and worker
shutdown completion. Never put source paths or credentials in diagnostics.

## Failure stimulus and sequence

1. Wait for the actual history screen, select the full anchor and final
   incremental target, accept the first-Enter scan disclosure, and send the
   second Enter. Wait for the source gate and visible scan-running state.
2. Close only the parent’s stdout PTY master. Keep the stdin PTY master and the
   parent’s stdout slave descriptor open, so the child remains alive and the
   original stdin terminal can still be inspected. The browser redraws on each
   loop before its 50 ms input poll; a draw to the now-closed stdout PTY should
   fail. This stimulates the production writer path without resize, signal
   injection, or an assertion about a particular crossterm event.
3. Require the real scan token to become cancelled while the source gate stays
   blocked. This establishes that terminal-loop failure, rather than the test
   driver or a normal quit key, initiated shutdown. Require the child to remain
   alive until the parent releases the gate.
4. Release the gate and require bounded child exit with its expected controlled
   browser error. In the helper, downcast the returned error payload to
   `terminal::Completion` and require `event.is_some()` and `teardown.is_some()`.
   Downcast the teardown error payload to `terminal::TeardownFaults`; require
   raw-mode restoration to have no fault and record explicit `raw_failed`,
   `screen_failed`, and `cursor_failed` booleans (the latter two are expected
   to be true on the closed output PTY). This
   records that each teardown action was attempted without claiming alternate
   screen or cursor restoration succeeded. Publish the completion marker only
   after `browser::run` returned, which follows the production worker
   close/drain/join path.
5. The parent has closed the output PTY master, so libtest cannot safely print
   its normal success line afterward. Once the helper has validated `Completion`
   and atomically published the completion marker, it calls `process::exit(0)`.
   This is an internal ignored-driver harness detail. Acceptance requires both
   the marker contents and successful child exit; an earlier exit cannot pass.
6. Read the live stdin PTY termios and confirm its original attributes were
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
