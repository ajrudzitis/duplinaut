# TUI definitely-unpublished retry acceptance plan

Status: proposed; specification only. This is the positive counterpart to the
restore retry-lockout acceptance and covers only an ordinary definitely
unpublished result.

## Goal and scope

Verify that an `Unpublished(Cancelled)` result with no uncertain outer
candidate permits one new restore preview, using a fresh operation ID and
cancellation token. The new preview must start with an empty, unarmed
acknowledgment; it must not dispatch another Execute until a new successful
preview receives its own exact acknowledgment.

This acceptance traces to [RESTORE-UI-009](parsing-requirements.md#restore-ui-009)
(the ordinary definitely-unpublished retry allowance),
[RESTORE-UI-007](parsing-requirements.md#restore-ui-007) (fresh operation token),
[RESTORE-UI-004](parsing-requirements.md#restore-ui-004) (fresh preview
confirmation), [RESTORE-UI-006](parsing-requirements.md#restore-ui-006)
(single frozen request and bounded worker commands), and
[RESTORE-UI-008](parsing-requirements.md#restore-ui-008) (preserve the exact
unpublished outcome). Link [RESTORE-UI-014](parsing-requirements.md#restore-ui-014)
to classify the evidence as a state-machine test, not native backend or real
terminal evidence.

## Deterministic test arrangement

1. Add a Linux TUI unit test in the `duplicity_tui` test binary. Read the
   checked-in plain 3.0.7 fixture to prepare a frozen snapshot/selection and
   pair it with a test destination; no filesystem backend is invoked.
2. Pass a synthetic successful `Reply::Outcome` containing exactly
   `Unpublished(Cancelled)` with default cleanup and `uncertain_outer: None`
   through the production outcome transition. Start with a prior operation ID
   and a cancellation token that has been cancelled, as after active
   cancellation.
3. Invoke the same production transition used by a Press `r` event, with a
   recording sender. The test and browser loop MUST share the production path
   that increments the operation ID, creates the fresh token, clears the
   prior acknowledgment/document, and sends the exact `Command::Prepare`.
   Extract an internal helper or inject the reply through a test harness for
   the production event loop; do not reimplement these effects in the test.
4. Feed the recorded preparation's successful `Reply::Preview` through the
   same production/test reply-dispatch transition used by the browser loop.
   That transition MUST clear the active operation ID only when the reply ID
   matches, then perform the successful-preview state mapping. Assert the
   matching reply clears active before mapping, and a stale reply leaves the
   current active ID unchanged. The test and production path MUST share this
   transition, including the fresh empty/unarmed acknowledgment state.
5. Send Press Enter through the shared production preview-key/confirmation
   transition, or through an injected production-loop harness, so the test
   observes whether Execute is enqueued. Use state assertions or bounded
   channels; do not use sleeps or a real backend.

## MUST-level acceptance requirements

1. Require the outcome transition to retain `Unpublished(Cancelled)`, set
   retry eligibility, and retain the typed diagnostic. The result's lack of an
   uncertain candidate MUST be explicit in the fixture.
2. Require one Press `r` to enqueue exactly one `Command::Prepare` with the
   same frozen snapshot, selected roots, and destination, and an operation ID
   strictly greater than the completed operation's ID. The old token MUST
   remain cancelled; the new token MUST be initially clear and distinct in
   cancellation state. No `Command::Execute` may be sent by `r`.
3. Require a matching successful Preview reply to clear its active operation
   ID before mapping preview state, while a stale reply leaves the current
   active ID unchanged. Require the new preview acknowledgment to be empty
   and unarmed. An Enter event before typing the full `restore` acknowledgment
   MUST NOT enqueue Execute, and this assertion MUST exercise the production
   preview-key/confirmation transition or production-loop harness. This
   acceptance stops before testing the new acknowledged Execute itself.
4. Duvet-link the test to RESTORE-UI-009, -007, -004, -006, -008, and -014. Test
   annotations MUST identify the outcome and preview as synthetic/frontend
   state-machine evidence, not native backend behavior.

## Verification and evidence

- Run the focused test and full `cargo test -p duplicity-tui` on Linux; run
  portable TUI tests on macOS as available.
- Run targeted Duvet queries for RESTORE-UI-004, -006, -007, -008, -009, and
  -014, then `cargo xtask trace-update` and `cargo xtask trace`. Run formatting
  and strict TUI Clippy.
- Require hosted Linux assurance (`cargo xtask ci`), Duvet Pages publication,
  and independent implementation review before acceptance.
- Review must confirm the new command is Prepare-only, the old token remains
  cancelled, the new token is fresh, and confirmation state cannot carry over
  to the next preview. Both the `r` dispatch and successful Preview mapping
  and the unarmed Enter check must exercise shared production code. Matching
  Preview IDs must clear active before mapping; stale IDs must leave active
  unchanged.

## Exclusions

This acceptance does not cover `Published`, uncertain `Unpublished`,
`PublicationIndeterminate`, or unavailable-worker outcomes; those cases remain
locked out or have separate acceptance. It does not invoke the native
filesystem backend, perform a second restoration, exercise real PTY input,
claim cancellation latency or syscall interruption, inspect/modify a
destination, cover S3 or crypto, or prove frontend controller behavior.

## Checklist

- [ ] Exercise the production outcome and Press-`r` transitions for a definite
      `Unpublished(Cancelled)` reply.
- [ ] Assert one fresh Prepare operation/token and a new empty, unarmed
      acknowledgment with no Execute on Enter alone.
- [ ] Focused/full TUI tests, Duvet trace, formatting, and strict Clippy pass;
      hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
