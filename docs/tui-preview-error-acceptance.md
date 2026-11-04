# TUI executable-preview error acceptance plan

Status: proposed; specification only. This covers a matching Prepare reply
that reports preview construction failure, distinct from user abandonment or
worker result loss.

## Goal and scope

Verify that a matching failed `Reply::Preview` clears the active Prepare
operation, presents the preview-failed state, leaves no executable preview or
armed acknowledgment, and cannot dispatch Execute. Use a synthetic typed
preview error through production reply handling; do not call the filesystem
restore backend.

This acceptance traces to [RESTORE-UI-004](parsing-requirements.md#restore-ui-004)
(errors/completed requests clear and disarm acknowledgment),
[RESTORE-UI-006](parsing-requirements.md#restore-ui-006) (matching reply
handling and one active operation), and [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
(state-machine evidence is separate from native/backend and terminal
evidence).

## Deterministic test arrangement

1. Add a portable unit test in `crates/tui/src/browser.rs`, in the existing
   `duplicity_tui` test binary. Initialize browser reply state with one active
   Prepare ID, no displayed `Preview`, and an empty/unarmed acknowledgment.
2. Construct a matching `Reply::Preview { result: Err(Error::Selection) }`
   (or another precise preview-construction error) and pass it through the
   production reply dispatch and preview result mapping used by the browser
   loop. The test and production path MUST share those transitions.
3. Record outgoing commands with a bounded test channel. Do not use sleeps,
   PTYs, a native backend, or filesystem operations.
4. After the matching failure, submit a Press-Enter event through the shared
   production idle-key dispatch used when `preview == None`, or an injected
   browser-loop harness. Record any resulting commands from that production
   path.

## MUST-level acceptance requirements

1. Require the matching reply ID to clear the active operation before mapping
   the failed preview. Require the status to be `Executable preview failed; no
   restore is armed.`, `preview == None`, and acknowledgment empty/unarmed.
2. A stale error reply ID MUST leave the current active operation and state
   unchanged; it MUST NOT clear the active ID or replace the status.
3. The failure transition itself MUST enqueue no command and MUST NOT display
   a confirmation prompt. Afterward, process Press Enter through the shared
   production idle-key dispatch or injected browser-loop harness; it MUST NOT
   enqueue `Command::Execute`.
4. Duvet-link the test to RESTORE-UI-004, -006, and -014. Annotations MUST
   identify this as synthetic state-machine coverage and make no claim about
   native filesystem or real-terminal behavior.

## Verification and evidence

- Run the focused test and full `cargo test -p duplicity-tui` on macOS and
  Linux; this state-only test should be portable.
- Run targeted Duvet queries for RESTORE-UI-004, -006, and -014, then
  `cargo xtask trace-update` and `cargo xtask trace`. Run formatting and strict
  TUI Clippy.
- Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages publication,
  and independent implementation review before acceptance.
- Review must confirm the test feeds matching and stale errors through the
  shared production dispatch, preserves the no-preview/no-confirmation state,
  then sends Press Enter through the production idle-key route and observes no
  Execute command.

## Exclusions

This case does not test active-preview cancellation/abandonment, worker panic
or unavailable results, restore outcomes or retries, native filesystem
restoration, real PTY presentation, cancellation timing, destination or
staging behavior, S3, cryptographic behavior, or formal frontend-controller
proof.

## Checklist

- [ ] Exercise matching and stale preview-error replies through production
      reply dispatch and mapping.
- [ ] Assert active clearing only for the matching ID, no armed preview, and
      no Execute dispatch.
- [ ] Focused/full TUI tests, Duvet trace, formatting, and strict Clippy pass;
      hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
