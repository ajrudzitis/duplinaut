# Unix PTY resize acceptance plan

Status: independently reviewed and adopted before implementation;
implementation and execution are pending.

This extends [read-only PTY acceptance](tui-readonly-pty-acceptance.md) with a
shrink/expand cycle while the actual local browser retains a reconstructed
snapshot and marked selection. It supplies one additional real-terminal case
for [RESTORE-UI-014](parsing-requirements.md#restore-ui-014) and scoped display
and selection observations. It introduces no new normative format requirement.

## Fixture and starting state

Launch the actual `duplicity-tui local` binary in a Unix PTY against the official
`tests/fixtures/duplicity-3.0.7/plain` fixture, with `TERM=xterm-256color`, an empty
working directory and no restoration destination. Start at 40 rows by 240 columns.
Use the existing visible anchor/target/two-Enter sequence to reconstruct the
final incremental date, `1700172800`. Derive the expected 21 entries and
2,621,564 regular payload bytes from the independent `plain`, index-2 snapshot
in `expected.json`.

Navigate to the oracle path with original components `a`, `b`, `child.txt`, mark
it with Space, and obtain the read-only `p` preview. Observe its marked file row
and four-entry count: one 16-byte file and the source root, `a`, and `a/b`
directory ancestors. The terse `p` display supplies count/selection evidence;
separate worker/core tests establish the exact plan membership.

## Resize stimulus and observations

For each resize, change the slave PTY window size through safe
`rustix::termios::tcsetwinsize`, read it back with `tcgetwinsize`, update the
zero-scrollback `vt100` screen model to the same rows and columns, and explicitly
send `SIGWINCH` to the running child's positive PID through safe
`nix::sys::signal::kill`. Fail on any operation error or premature child exit.
Enable only the pinned dependencies' necessary test features; preserve the
workspace prohibition on project unsafe code.

The harness attaches slave stdio without establishing a controlling-terminal
session or foreground process group. Explicit child notification is therefore
part of this stimulus. The case does not establish automatic kernel delivery
of `SIGWINCH`, job-control behavior, or whether the resulting redraw was driven
by a particular event variant rather than the browser's regular size checks.

1. Shrink the PTY from 40 by 240 to 18 by 96. Send no navigation or marking
   input while waiting for the resized display. Require new terminal output
   after the stimulus and application-rendered geometry at the new boundary,
   such as the footer's bottom-right frame cell at row 17, column 95. A model
   `size()` assertion alone supplies no evidence that the child redrew.
2. In that current resized screen, observe the snapshot panel, the marked
   `a/b/child.txt` file row, and the four-entry read-only preview with its
   nothing-written statement. Choose assertions that fit within the 96-column
   viewport and distinguish the footer's new position from stale wide-screen
   content.
3. Expand the PTY back to 40 by 240 using the same explicit stimulus. Again
   require new output and geometry at the expanded boundary, such as the footer
   bottom-right cell at row 39, column 239. Observe the same marked child row and
   four-entry preview in the current screen.
4. Press Space once and observe zero marked roots; this checks that the cursor
   still selects the previously marked child across the resize cycle. Press
   `p` and observe the oracle's whole-snapshot count of 21, supplying an
   additional check that the snapshot remains available and unchanged.
5. Quit with `q`, require successful controlled exit, and check original slave
   terminal attributes are restored and alternate-screen leave is emitted.
   Observe raw-mode entry while the UI is active. Compare recursive source
   names/content/symlink-target fingerprints before and after the process and
   require its temporary working directory remains empty.

## Harness and evidence bounds

Process incoming terminal bytes into the existing current-screen model. Drain
currently pending output before applying the next resize so the model does not
interpret an older frame at new dimensions. Use explicit deadlines and bounded
transcript storage, with child kill/reap and temporary-directory cleanup on every
failure. Do not satisfy later observations from a retained raw transcript or
from the screen model's local resizing alone. Keep the model in the same
alternate-screen state rather than creating an unrelated blank model mid-run.

Pinned API references are the
[rustix 1.1.5 terminal operations source](https://docs.rs/crate/rustix/1.1.5/source/src/termios/tc.rs),
[nix 0.29.0 signal source](https://docs.rs/crate/nix/0.29.0/source/src/sys/signal.rs),
and [vt100 0.16.2 screen source](https://docs.rs/crate/vt100/0.16.2/source/src/screen.rs).
The latter exposes `Screen::set_size` through `Parser::screen_mut`; the screen
model remains test infrastructure, outside project formal proofs.

Record focused Unix test execution, independent implementation review, Duvet
trace results and the required CI result before marking the milestone complete.
Link the test to RESTORE-UI-014 without claiming complete frontend acceptance.
The filesystem fingerprints do not establish absence of transient writes or
metadata changes such as access times. This case does not cover zero/tiny
dimensions, arbitrary resize storms, executable preview wrapping/scrolling,
restoration, acknowledgment, cancellation, terminal/output faults, live S3,
encrypted credentials or formal frontend controller obligations.
