# TUI invalid-parent restore diagnostic acceptance plan

Status: proposed; specification only. This adds one real Linux PTY case for a
typed backend rejection and complete escaped destination diagnostics.

## Goal and requirements

Drive the production TUI through a concrete restore preview and exact
acknowledgment, then deliberately configure an existing regular file as the
restore parent. The native backend's directory-only parent open must fail
before staging with a typed definite `Unpublished` outcome. The result screen
must retain that outcome, preserve the complete requested raw parent/leaf
identity, and render all identity bytes with reversible terminal-safe
escaping. No restore tree or staging entry may be created.

This traces to [RESTORE-UI-003](parsing-requirements.md#restore-ui-003)
(preview before confirmation), [RESTORE-UI-005](parsing-requirements.md#restore-ui-005)
(delegate to native restore backend), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(retain the definite `Unpublished` class and cleanup warnings),
[RESTORE-UI-010](parsing-requirements.md#restore-ui-010) (accessible complete
identity with reversible escaping), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (actual terminal
interaction/result evidence). This is one backend error path, not a general
controller or error-matrix proof.

The accepted [executable-preview resize case](tui-executable-preview-resize-acceptance.md)
MUST pass first. Reuse its fixture, production-binary PTY history/scan/preview
flow, and terminal harness. This case uses fixed PTY size and no resize.

## Fixture and deterministic failure setup

1. Add a Linux-only PTY integration test to the existing
   `crates/tui/tests/cli.rs` acceptance harness. Launch
   `CARGO_BIN_EXE_duplicity-tui` against the checked-in official
   `tests/fixtures/duplicity-3.0.7/plain` source. Use a fresh test-owned
   working directory and a fixed 40x240 PTY with `TERM=xterm-256color`.
2. Create an ordinary test-owned file in that working directory with raw
   component bytes `[b'p', 0x1b, b'\\', 0xff]`; write a short sentinel body
   and capture its bytes/metadata. Use the file's full raw OS path as
   `--restore-parent`. Set `--restore-name` from raw bytes
   `[0x1b, b'\\', 0xff, b'R']`. Both values are valid nonempty OS paths with no
   NUL; the leaf contains no slash. The outer working directory is a private
   test-owned directory. Do not convert either argument through UTF-8.
3. This parent choice deterministically causes the production native backend's
   `open(parent, O_DIRECTORY)` to fail with Linux `ENOTDIR` before plan
   staging/publication. The expected typed outcome is exactly
   `Unpublished(Cause::Syscall(ENOTDIR))`, with no uncertain outer candidate
   and zero cleanup attempts/failures. If execution reaches a filesystem
   staging or publication step, the test fails and preserves its directory.
4. Use the official plain target `1700172800` and a whole-snapshot `r` preview.
   The source oracle identifies 21 entries and 2,621,564 regular payload
   bytes. This plan does not repeat a complete filesystem tree comparison; the
   successful restore PTY acceptance owns that oracle check.

## Acceptance sequence

1. Through the real PTY, select the full anchor and final target and confirm
   the required full scan. Press `r` with no marks. Require the successful
   preview to show target, whole-snapshot scope, 21 entries, 2,621,564 payload
   bytes, the exact source-directory-ancestor statement, requested parent and
   new leaf, no-replacement behavior, and supported metadata/backend-check
   policy before acknowledgment. Require the parent to be rendered in its
   escaped form with final component suffix `p\x1b\\\xff`, and the leaf as
   exactly `\x1b\\\xffR`, following `display::bytes`.
2. Type exactly ASCII `restore`, observe the ready/armed acknowledgment, and
   press Enter once. Require the completed result document with typed
   `Unpublished(ENOTDIR)` evidence. An intermediate `Restoration running.`
   frame is not required because this immediate backend error may be polled
   before the next draw. Do not retry.
3. Require the production UI to retain a typed
   `RestoreOutcome::Unpublished` with cause `Syscall(ENOTDIR)`, no
   `uncertain_outer`, and cleanup counters `attempted=0`, `failures=0`,
   `exhausted=false`, `detail_truncated=false`. The user-visible result MUST
   identify the outcome as `Unpublished` and show the same cause/cleanup
   evidence. Because this definite outcome contains no backend receipt, the
   production frontend MUST add the original configured parent and leaf from
   the frozen request to the accessible result document; it MUST NOT invent a
   receipt, identity, cleanup failure or publication uncertainty.
4. Scroll the result document using production Up/Down controls. Require the
   exact requested parent and leaf in their documented escaped form. For each
   raw input byte, visible escaping MUST follow `display::bytes`: printable
   ASCII other than backslash is literal, backslash is doubled, and control or
   non-ASCII bytes render as lowercase `\xNN`. Assert the expected suffixes
   `p\x1b\\\xff` for the parent component and
   `\x1b\\\xffR` for the leaf. The parsed terminal screen must contain
   these visible ASCII sequences; raw identity bytes MUST NOT appear as
   rendered content. Ordinary VT protocol control sequences are not identity
   text and are outside this assertion.
5. Verify no leaf path exists beneath the configured parent (which remains a
   regular file), no `.duplicity-*` staging sibling exists in the test-owned
   working directory, and the parent sentinel bytes/metadata are unchanged.
   Require the local backup source fingerprint unchanged. Quit with `q`; check
   successful controlled exit, original PTY termios restoration, and
   alternate-screen leave output. Only after the child is reaped and all
   absence, sentinel, source, and terminal assertions pass, explicitly remove
   the unique test-owned scratch directory and assert it is absent. On any
   failure, preserve and report the scratch directory for inspection. The test
   MUST NOT recursively remove any path outside its unique scratch directory.

## Duvet and verification

- Duvet-link the Linux integration test to RESTORE-UI-003, -005, -008, -010,
  and -014. Annotations MUST say this is a real PTY path with a real backend
  pre-staging failure and a definite typed `Unpublished` result. The frontend
  destination identity comes from the retained configured request because
  this result variant has no receipt.
- Run the focused Linux test and full `cargo test -p duplicity-tui` on Linux,
  targeted Duvet queries, `cargo xtask trace-update`, `cargo xtask trace`,
  formatting, and strict TUI Clippy. Require hosted Linux assurance
  (`cargo xtask ci`), Duvet Pages publication, and independent implementation
  review. No AWS credentials or external service are needed.
- Independent review MUST confirm the parent is a regular file before UI
  launch and its raw bytes are passed unchanged; the backend fails at the
  directory-only parent open before staging; the result remains typed
  `Unpublished(ENOTDIR)`; the diagnostic adds only the frozen requested parent
  and leaf; escape strings match exact bytes; no backend cleanup or false
  receipt is claimed; the harness removes its scratch directory only after
  successful assertions and preserves it on failure; and all filesystem
  assertions stay within the test-owned scratch root.

## Exclusions

This acceptance covers one Linux `ENOTDIR` parent-open failure after explicit
acknowledgment. It does not cover a successful restore, publication
indeterminacy, uncertain staging candidates, cleanup warnings, other syscall
failures, restoration cancellation, terminal/output failure, raw symlink
targets, S3/encrypted credentials, every escape byte, or formal controller
proof. Existing success, retry, unavailable-result and output-failure plans
retain their separate scopes.

## Checklist

- [ ] Drive the production Linux browser through an official whole-snapshot
      preview and exact acknowledgment into a deterministic `ENOTDIR` result.
- [ ] Assert typed `Unpublished`/cleanup evidence and escaped full requested
      parent/leaf in the scrollable result screen.
- [ ] Assert no staging/publication, unchanged sentinel/source, and terminal
      restoration.
- [ ] Focused/full Linux TUI tests, Duvet trace, formatting, strict Clippy,
      hosted assurance, and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
