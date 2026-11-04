# TUI active-restoration cancellation acceptance plan

Status: proposed; specification only. This is independent of encrypted-reader
and filesystem-integration work.

## Goal and scope

Exercise the actual TUI event loop when the user requests cancellation with
`q` or `Esc` after a restore operation has entered its running state. Use the
existing official plain duplicity 3.0.7 fixture and native Linux restore path.
The test verifies the frontend operation token, typed cancellation result,
terminal lifecycle, and worker join. It does not require the encrypted-reader
hosted gate or modify core restore code.

This acceptance traces to
[RESTORE-UI-004](parsing-requirements.md#restore-ui-004) (one acknowledged
request), [RESTORE-UI-006](parsing-requirements.md#restore-ui-006) (one
operation and stale-ID isolation), [RESTORE-UI-007](parsing-requirements.md#restore-ui-007)
(running state and operation-specific cancellation),
[RESTORE-UI-008](parsing-requirements.md#restore-ui-008) (typed outcome), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (terminal evidence
recorded separately). The test does not cover the controlled I/O failure
scenario in RESTORE-UI-012.

## Test-only deterministic gate

1. Place both the parent PTY test and its ignored internal driver in the same
   `duplicity_tui` unit-test binary. The parent MUST spawn the ignored driver
   in a separate process from that same test binary. An integration-test
   binary cannot access `cfg(test)` hooks in the library. This keeps
   process-global Crossterm state isolated and makes the hook available only
   in `cfg(test)` builds.
2. Add a narrow test-only gate around the worker's handling of one
   `Command::Execute`. After the worker receives that command, the gate MUST
   capture its operation ID and a clone of that command's exact
   `Cancellation` token, verify the token is initially clear, signal
   `execute-entered`, and block before calling the real native backend.
   Production builds MUST contain no gate or hook behavior.
3. While held, the gate MUST observe cancellation on that same token and
   publish `cancel-observed`; it MUST NOT cancel the token itself. The parent
   releases the gate only after it has sent `q` or `Esc`, observed the
   cancellation-requested UI state, and received the child's
   `cancel-observed` marker. On release, the worker calls the real backend
   using the same now-cancelled token.
4. The helper MUST publish a bounded completion marker only after
   `browser::run` returns and worker shutdown/join completes. A timeout,
   missing phase marker, wrong operation ID, or unexpected gate failure MUST
   fail the test rather than count as acceptance.

## Acceptance requirements

1. Start the real TUI child on an isolated PTY with a test-owned destination
   parent and the official plain fixture. Drive the actual key sequence to
   select the full anchor/target, scan the final snapshot, request restore
   preview, enter the exact `restore` acknowledgment, and press Enter once.
   The parent MUST wait for both the visible “Restoration running” status and
   `execute-entered` before sending cancellation input.
2. Run two cases: send `q` in one and `Esc` in the other. For each case, require
   the operation ID to remain unchanged, the exact Execute cancellation token
   to become cancelled only after the key event, the visible
   cancellation-requested/waiting state, and the worker gate's
   `cancel-observed` marker before release.
3. After releasing the gate, require the real backend to return exactly
   `RestoreOutcome::Unpublished` with `Cause::Cancelled` and no uncertain
   outer candidate. The requested destination MUST remain absent. The result
   MUST be presented as unpublished/cancelled, not published,
   publication-indeterminate, or unavailable. No second restore request may
   be queued while the first operation is active.
4. For the `q` case, require the UI to retain the typed result until a
   subsequent `q` exits. For the `Esc` case, require the result to remain
   visible, then send `q` to exit. In each case, require the helper's
   post-`browser::run` marker, successful child exit, terminal raw-mode and
   alternate-screen restoration, and joined worker before the parent accepts
   completion.
5. Compare the source fixture fingerprint before and after the test. Require
   the destination parent to contain no destination or staging residue after
   the definitely unpublished cancellation. Do not inspect or clean an
   uncertain publication result; the expected outcome is definite
   `Unpublished(Cancelled)`.
6. Add Duvet test annotations for the linked RESTORE-UI requirements. The
   annotations and test names MUST say that the test cancels before backend
   filesystem work through a deterministic TUI-only gate; they MUST NOT imply
   interruption of an in-progress syscall or a fixed cancellation latency.

## Verification and evidence

- Run the focused Linux PTY parent cases and the full
  `cargo test -p duplicity-tui` suite. The test MUST use the existing local
  official fixture and MUST NOT change producer data or source contents.
- Run targeted Duvet test queries for RESTORE-UI-004, -006, -007, -008, and
  -014; then run `cargo xtask trace-update` and `cargo xtask trace`.
- Run formatting and strict TUI Clippy checks, then required assurance CI
  (`cargo xtask ci`) on Linux. Record hosted Linux CI evidence before marking
  acceptance complete. macOS checks may compile/run portable TUI unit tests,
  but do not substitute for the Linux backend/PTy acceptance.
- Independent review MUST confirm that the driver is isolated, gate code is
  test-only, cancellation is observed on the exact Execute token, the UI
  stimulus follows visible running state, the real backend returns the typed
  result, and the worker is joined before completion.

## Exclusions

This acceptance does not cover cancellation during actual backend syscalls,
publication races, fixed cancellation latency, output-failure shutdown,
encrypted credentials, S3, live bucket access, filesystem controller proofs,
or any OpenPGP primitive. It does not change core/backend code, producer
fixtures, restore destination policy, or the encrypted-restore acceptance.

## Checklist

- [ ] Add the isolated ignored driver and `cfg(test)` Execute gate in the TUI
      crate only.
- [ ] Implement and Duvet-link the q/Esc active-restore PTY cases.
- [ ] Focused/full TUI tests, Duvet trace, formatting, strict Clippy, and local
      Linux assurance CI pass.
- [ ] Independent review and hosted Linux assurance pass.
