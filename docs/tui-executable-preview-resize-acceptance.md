# TUI executable-preview resize acceptance plan

Status: accepted. The Linux-gated executable-preview resize acceptance passed
in hosted run [41905441225](https://github.com/ajrudzitis/duplinaut/actions/runs/41905441225),
including full assurance and Duvet Pages publication. That run compiled and
executed the Linux-only production PTY test described here.

## Goal and scope

Launch the actual Linux TUI against the official local plain fixture, create a
whole-snapshot `r` preview, and resize the PTY from 40x240 to 18x96 and back.
After each resize, prove the child rendered a new frame at the changed
geometry, keep preview details reachable through production scrolling, and
verify the entered acknowledgment remains intact without dispatching Execute.
Esc must abandon the preview without creating a destination or staging entry.

This is separate from the accepted
[read-only resize case](tui-resize-pty-acceptance.md): that case retains a
snapshot selection and `p` preview, and explicitly excludes executable-preview
wrapping/scrolling and acknowledgment. This acceptance uses only the local
plain source and never enters native filesystem restoration. Native restore
options are accepted only on Linux, so this is a Linux-gated test with no AWS
or external-service prerequisite.

Trace this test to [RESTORE-UI-003](parsing-requirements.md#restore-ui-003)
(the concrete preview and full details accessible by bounded scrolling),
[RESTORE-UI-004](parsing-requirements.md#restore-ui-004) (empty Enter cannot
execute and leaving the preview clears acknowledgment), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (real terminal
interaction evidence recorded separately). These links cover only the listed
observations; they do not establish publication behavior or frontend
controller proofs.

## Fixture and process setup

1. Add a Linux-only Unix PTY integration test in
   `crates/tui/tests/cli.rs`, alongside the existing `pty_acceptance` module.
   Launch `CARGO_BIN_EXE_duplicity-tui` as a separate production process with
   `local tests/fixtures/duplicity-3.0.7/plain`, a fresh empty working
   directory, `TERM=xterm-256color`, and a complete pair of
   `--restore-parent PARENT --restore-name LEAF`. The parent must be a
   test-owned empty directory on an allowed filesystem and the leaf must be
   absent before launch.
2. Begin with a 40-row by 240-column PTY. Establish the original termios and
   source fingerprint. Reconstruct the official full-plus-two-incremental
   plain chain at target `1700172800` using the existing visible anchor, target,
   and two-Enter full-scan flow. The source oracle is `expected.json`,
   `mode = "plain"`, `index = 2`: 21 entries and 2,621,564 regular payload
   bytes.
3. Keep the test wholly in browser/preview state. Do not press Enter after
   successfully entering the acknowledgment. The native `Execute` command
   must never be submitted; there is no backend call, filesystem publication,
   or restore-result assertion in this case.

## Acceptance sequence

1. Press `r` with no marked roots and wait for the successful whole-snapshot
   preview. Require target `1700172800`, whole-snapshot scope, the oracle's
   21-entry and 2,621,564-byte totals, requested parent and leaf, no-replacement
   statement, the visible `Include exact source directory ancestors; no
   flattening.` policy, backend-validation caveat, and supported
   mode/mtime/symlink policy to be present in the rendered preview over the
   current screen and its production scroll positions. Require
   `Acknowledgment [] unarmed` when the preview first becomes ready.
2. Press Enter once with the acknowledgment empty. Require the visible
   `Acknowledgment must be exactly restore, then Enter.` status, no
   `Restoration running` state, no Published result, and an absent destination.
   Then type exactly the ASCII bytes `restore`; wait for the visible
   `Acknowledgment [restore] ready` footer. Do not press Enter again.
3. Before shrinking, drain currently pending PTY output while the model remains
   at 40x240. Set the PTY slave to exactly 18x96 with safe
   `rustix::termios::tcsetwinsize`, read the size back with `tcgetwinsize`,
   resize the existing zero-scrollback `vt100` model, and send `SIGWINCH` to
   the child's positive PID through safe `nix::sys::signal::kill`. Wait for
   output bytes beyond the pre-resize transcript length and require a complete
   new 18x96 application frame, including the right/bottom border at row 17,
   column 95. A local model-size change or stale frame is not sufficient.
4. At 18x96, require the preview panel and `Acknowledgment [restore] ready`
   footer to remain visible. Use production Up/Down preview scrolling to
   observe the required fields over current frames: first the target/scope and
   entries/payload summary, then parent/leaf and no-replacement/backend
   validation/mode-metadata policy. Since each frame is narrower than the
   original, assert meaningful visible line fragments rather than requiring a
   wrapped source line to appear as one contiguous screen string. After each
   scroll, require the acknowledgment footer unchanged, no running/Published
   state, and the destination absent.
5. Expand the slave back to exactly 40x240 using the same drain, safe resize,
   readback, model update, and explicit child `SIGWINCH`. Require new output
   after the stimulus and a complete current 40x240 frame, including the
   right/bottom border at row 39, column 239. Verify the preview still
   identifies target, whole-snapshot scope, counts, destination and policy;
   require the acknowledgment still reads `[restore] ready` and the destination
   remains absent. Do not send the confirming Enter.
6. Press Esc while the preview is open. Require the production abandonment
   status, preview removal, cleared acknowledgment, absent destination, and no
   `.duplicity-*` staging entry in the test-owned parent. Then press `q` and
   require successful controlled process exit, original PTY termios restored,
   alternate-screen leave output, unchanged source fingerprint, and an empty
   temporary working directory.

## Resize harness and evidence

- Reuse the safe PTY setup, `resize_pty`, `frame_at_geometry`, bounded output
  pump, source fingerprint, and teardown checks from the accepted
  `pty_acceptance` tests. The resize helper must drain old output before each
  stimulus, set and read back the actual slave geometry, update the same
  current-screen model, signal only the positive child PID, and return the
  previous output length. Require both new bytes and child-rendered frame
  corners at each new geometry; `vt100::Screen::set_size` alone proves nothing
  about a child redraw.
- Keep the PTY model in the same alternate-screen session and preserve the
  preview's scroll state deliberately: move to the top before checking first
  fields, then down to the later policy fields. Do not create a blank model or
  satisfy an observation from a raw transcript retained before the resize.
- Bound transcript storage, use deadlines for each visible state and child
  exit, and kill/reap the child on failures. Cleanup the temporary parent only
  after the child is reaped and the destination is known absent; this test
  never sends Execute. The parent itself must remain empty.
- Duvet-link only the integration test to RESTORE-UI-003, -004 and -014. The
  annotations MUST distinguish visible-preview/scroll evidence, empty-Enter
  refusal and Esc acknowledgment clearing from successful restoration or
  publication evidence.
- Hosted run [41905441225](https://github.com/ajrudzitis/duplinaut/actions/runs/41905441225)
  passed Linux assurance (`cargo xtask ci`) and published the Duvet Pages
  report. Its Linux job compiled and executed the gated PTY case, including
  full TUI tests, Duvet trace, formatting, and strict Clippy. macOS execution
  of other Unix PTY cases does not count as this Linux-gated case.

## Exclusions

This case covers one successful local plain snapshot and one preview while
resizing through the exact two stated geometries. It does not execute the
restore, test destination publication or outcomes, resize during active
restoration, test resize storms/tiny dimensions/automatic signal delivery,
alter the destination or selection during the preview, test S3 or encrypted
credentials, or prove controller semantics. The accepted read-only resize,
successful-restore PTY, raw-identity PTY, and backend restore tests retain their
separate scopes.

## Checklist

- [x] Create an actual production-browser whole-snapshot `r` preview and enter
      the exact acknowledgment without confirming it.
- [x] Prove 40x240 -> 18x96 -> 40x240 child-rendered frames and accessible
      preview fields through production scrolling; acknowledgment remains
      armed and no Execute is sent.
- [x] Esc abandons the preview, destination/staging remain absent, source is
      unchanged, and terminal state is restored.
- [x] Focused/full Linux TUI tests, Duvet trace, formatting, strict Clippy,
      hosted assurance, and Duvet Pages pass in
      [run 41905441225](https://github.com/ajrudzitis/duplinaut/actions/runs/41905441225).
- [x] Independent implementation review accepts the scoped evidence.
