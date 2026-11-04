# PTY output failure acceptance plan

Status: q-triggered acceptance stimulus independently reviewed and adopted
before implementation.
The earlier resize-only variant passed focused macOS PTY testing and local
macOS CI, but its post-close PTY resize is not portable to Linux and does not
validate this revised stimulus.

Previous-variant local full-CI receipt: source SHA-256
`d5b4e7031b851f113055f0628d2be885a282671a31db88f6b3f930b38a22d422`;
fresh Verus/Vest/core proof counts were 2,047/3,183/1,274 with zero errors.
These results do not establish acceptance of the revised q-triggered case.

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
   descriptor used by the child and the stdin master/slave pair. The explicit
   `FD_CLOEXEC` assertions ensure the child inherited no hidden master. Do not
   issue an ioctl or signal after closing the output master. Send exactly one
   `q` on the still-live input PTY while the source gate remains held. The
   production key handler requests scan cancellation and changes the visible
   status to
   `Cancellation requested; waiting for operation outcome. q after the result exits.`
   If the next loop runs before gate release, this status is drawn. If the
   parent releases the gate first and the worker result is polled first, the
   failure-result status is drawn instead. Either path changes the visible
   status from the last successfully drawn compact frame; the next production
   draw writes changed cells to the closed stdout PTY. Do not require the
   cancellation-requested text specifically to be the failed frame. The
   stimulus avoids a post-close PTY resize, which returns `EIO` on Linux.
4. Require the exact scan token to become cancelled while the source gate stays
   blocked. In this case `q` initiates scan cancellation; the later failed
   status draw causes controlled browser shutdown. Do not claim the output
   fault initiated cancellation. The test driver never cancels the token.
   Require the child to remain alive until the parent releases the gate and
   require no browser-return marker before release.
5. After gate release, require bounded child exit with its expected controlled
   browser error. In the helper, downcast the returned error payload to
   `terminal::Completion` and require `event.is_some()` and `teardown.is_some()`.
   This retained event error is evidence that the changed-status draw—not the
   successful `q` handler—caused controlled shutdown. Downcast the teardown
   error payload to `terminal::TeardownFaults`; record explicit `raw_failed`,
   `screen_failed`, and `cursor_failed` booleans. Raw mode is restored through
   the live input PTY and is expected to have no fault; alternate-screen and
   cursor output faults are expected on the closed output PTY. The marker
   records attempts and observed faults, without claiming alternate-screen or
   cursor restoration succeeded. Publish the completion marker only after
   `browser::run` returned, which follows the production worker close/drain/join
   path.
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

This case covers a q-triggered scan cancellation followed by a controlled
status-redraw/output failure during a source-injected read-only scan. It does
not claim the output failure initiated cancellation. It does not cover active
restoration cancellation, typed restore outcomes, successful restoration
reporting, alternate-screen restoration over the failed output, input failure,
both stdout and stderr unavailable, real
controlling-terminal job control, arbitrary faults, live S3, or frontend formal
controller proofs. The source cancellation wait is cooperative; no fixed
cancellation latency or interruption of a real blocking operating-system call
is claimed.

Record focused Unix tests, independent review, Duvet trace, and full assurance
CI before adoption. The ignored child helper carries no separate Duvet or
assurance-gate claim; only default parent tests count as the acceptance cases.
