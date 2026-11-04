# TUI active-preview abandonment acceptance plan

Status: proposed; specification only. This covers cancellation of an
in-flight executable preview and discarding a successful reply that arrives
after the user abandons it.

## Goal and scope

Verify that Esc or `q` while a `Prepare` operation is active cancels that
operation's exact token, leaves the frontend waiting for its matching result,
and discards even a successful late `Preview` without arming confirmation or
dispatching Execute. Use deterministic synthetic command/reply sequencing; no
filesystem restore backend is called.

This acceptance traces to [RESTORE-UI-007](parsing-requirements.md#restore-ui-007)
(operation-specific cancellation), [RESTORE-UI-004](parsing-requirements.md#restore-ui-004)
(abandonment clears/disarms confirmation), [RESTORE-UI-006](parsing-requirements.md#restore-ui-006)
(one active request and stale reply isolation), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (state-machine evidence
is distinct from backend and real-terminal evidence).

## Deterministic test arrangement

1. Add a TUI unit test in the `duplicity_tui` test binary, using a validated
   snapshot from the checked-in plain 3.0.7 fixture and a test destination.
   Arrange one active `Command::Prepare` with a clear operation token, active
   operation ID, no displayed preview, and no acknowledged confirmation.
2. For separate Esc and Press-`q` cases, invoke the same production active-key
   transition as the browser loop. If needed, extract an internal helper used
   by both. Require it to cancel the exact token, set `abandoning`, retain the
   active operation ID while awaiting its reply, preserve the already-empty
   and unarmed acknowledgment, and enqueue no additional command.
3. Deliver a stale successful `Reply::Preview` first. Pass it through the
   shared production reply dispatch and require it to leave the current active
   operation and UI state unchanged. Then deliver a matching successful
   `Reply::Preview` for the abandoned Prepare through that same dispatch path.
   The prepared value may be generated from the fixture before the event
   sequence; no thread sleeps or timing races are needed.

## MUST-level acceptance requirements

1. For both Esc and Press-`q`, require the exact active Prepare token to change
   from not-cancelled to cancelled; no other operation token may be substituted.
   Require active ID to remain set until the matching result is dispatched.
2. A stale reply ID MUST NOT clear active, display/arm a preview, clear or
   replace the active operation state, or enqueue a command. A matching late
   successful Preview MUST clear active before result mapping, then be dropped
   because `abandoning` is set. Require `preview == None`, acknowledgment
   empty and unarmed, and status `Preview abandoned; nothing staged.`
3. Processing the matching late result MUST enqueue no command. Afterward,
   feed a Press-Enter event through the same production preview-key/
   confirmation dispatch used by the browser loop, or an injected production-
   loop harness; require it to enqueue no Execute because no Preview is
   displayed. A later new `r` request is a separate operation and outside this
   acceptance. No destination or staging path may be inspected or modified.
4. The active-key transition, reply-ID dispatch, preview result mapping, and
   post-discard Enter/confirmation dispatch exercised by the test MUST be the
   production code used by the browser loop; tests MUST NOT reproduce those
   transitions independently.
5. Duvet-link the test to RESTORE-UI-007, -004, -006, and -014. Annotations
   MUST classify it as synthetic frontend state-machine coverage, not proof of
   actual terminal input delivery or filesystem/backend cancellation.

## Verification and evidence

- Run the focused test and full `cargo test -p duplicity-tui`; this state-only
  test should remain portable across macOS and Linux.
- Run targeted Duvet queries for RESTORE-UI-004, -006, -007, and -014, then
  `cargo xtask trace-update` and `cargo xtask trace`. Run formatting and strict
  TUI Clippy.
- Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages publication,
  and independent implementation review before acceptance.
- Review must confirm both key variants use the production transition, stale
  IDs cannot mutate active state, and late matching success cannot arm or
  execute a discarded preview.

## Exclusions

This acceptance does not prove a real terminal delivers Esc/`q`, a worker or
backend observes cancellation during work, any cancellation latency, native
restore/publication behavior, real filesystem absence or cleanup, actual PTY
behavior, S3 access, encrypted credentials, or formal frontend-controller
correctness. The late Preview is a deterministic synthetic result used to
check frontend state handling.

## Checklist

- [ ] Exercise active Prepare cancellation for Esc and `q` through the
      production key transition.
- [ ] Assert stale reply isolation and discard of a matching late successful
      Preview, with empty/unarmed acknowledgment and no follow-up command.
- [ ] Focused/full TUI tests, Duvet trace, formatting, and strict Clippy pass;
      hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
