# TUI successful restore PTY acceptance plan

Status: proposed; specification only. This adds one real-browser, successful
Linux filesystem publication case using the official plain fixture.

## Goal and scope

Drive the production browser through the actual Linux terminal interface from
snapshot selection to a published restore. Observe the concrete executable
preview, require the exact acknowledgment sequence, then check the published
receipt and restored filesystem tree against the independent official oracle.
Use one full-plus-two-incremental snapshot date and whole-snapshot selection.

This acceptance traces to [RESTORE-UI-003](parsing-requirements.md#restore-ui-003)
(concrete preview), [RESTORE-UI-004](parsing-requirements.md#restore-ui-004)
(exact acknowledgment and one operation), [RESTORE-UI-005](parsing-requirements.md#restore-ui-005)
(use the adopted native restore API), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(preserve and render the `Published` result), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (real terminal and
native worker/result evidence recorded separately). Links identify the aspects
exercised here; UI-008 is covered only for a known `Published` result, not
unpublished/indeterminate outcomes, cancellation races, or full requirement
acceptance. This test does not claim controller proof.

## Fixture and filesystem setup

1. Run only on Linux from a Cargo integration test (for example,
   `crates/tui/tests/restore_success_pty.rs`). Launch the actual executable
   supplied by `CARGO_BIN_EXE_duplicity-tui` (or Cargo's equivalent binary path)
   in a Unix PTY against `tests/fixtures/duplicity-3.0.7/plain`, with fixed
   `TERM=xterm-256color` and bounded terminal dimensions large enough to show
   the preview. Provide `--restore-parent PARENT --restore-name LEAF`, where
   `PARENT` is a unique test-owned directory on a filesystem admitted by the
   existing native restore profile and `LEAF` is one new, ordinary UTF-8 name.
   Create the parent with mode `0700`; require the leaf to be absent before
   launch. Do not use S3, encrypted profiles, or a producer-generated backup.
2. Select the official plain chain's full anchor and target the final snapshot
   at `1700172800`. With no marks, the restore scope is the whole snapshot.
   Use `tests/fixtures/duplicity-3.0.7/expected.json`, `mode = "plain"`,
   `index = 2`, as the independent expected tree: exactly 21 entries and
   2,621,564 regular-file payload bytes.
3. Capture the source fixture's recursive raw names, file content hashes,
   symlink target bytes, and supported modes/mtimes before launch. Do not
   modify the source. Use explicit deadlines and bounded PTY transcript
   storage. Kill/reap the child on failure. Temporary-directory cleanup is
   permitted after the child is reaped if Execute was never sent. After
   Execute could have been sent, preserve and report the test path on timeout,
   child failure, or any result other than confirmed Published plus completed
   assertions.

## Acceptance sequence and requirements

1. Through the PTY, perform the production history-anchor and target-scan
   interactions. Wait for the final validated snapshot and whole-snapshot
   preview state. Require the rendered preview to identify target `1700172800`,
   the exact requested parent and leaf, whole-snapshot scope, 21 entries,
   2,621,564 regular payload bytes, source-directory-ancestor policy,
   no-replacement behavior, supported mode/mtime/symlink policy, and the
   backend-validation caveat before confirmation is offered. If the preview
   spans multiple rows, use the production Up/Down scrolling keys and assert
   visible rows from the beginning and end; do not infer complete text from a
   retained raw transcript alone.
2. Require the acknowledgment to be visibly empty and unarmed when the
   successful preview first appears. Press Enter once with an empty field and
   require no Execute/running state, no Published result, and the destination
   leaf still absent. Type exactly the seven ASCII bytes `restore`; require
   the displayed acknowledgment to be armed. Press Enter once and require one
   running restore state. Do not send a second confirmation or retry.
3. Wait for the production worker result and require the UI to retain and
   render `RestoreOutcome::Published` as a completed publication. The result
   shown to the user MUST correspond to the requested leaf. Require the leaf
   to be present and the parent to contain exactly that leaf, with no
   `.duplicity-*` staging entry or unrelated output. The frontend must use the
   immutable previewed request and adopted backend; the test must not call the
   backend directly or construct a replacement restore plan.
4. Walk the published leaf using raw Unix path components and
   `symlink_metadata`; recurse only into entries identified as directories and
   never follow a symlink. Require exactly the 21 oracle entries, preserving
   component boundaries and raw bytes, with matching entry type and exactly
   `metadata.mode() & 0o777 == oracle.mode`. For regular files, require exact
   size and SHA-256; for regular files and directories, compare nanosecond
   mtimes exactly by computing
   `i128::from(metadata.mtime()) * 1_000_000_000 +
   i128::from(metadata.mtime_nsec())` and requiring equality with the oracle's
   signed `mtime_ns`. For symlinks,
   require exact target bytes and mode, but do not compare mtime: the
   `expected.json` record declares `official_comparison_exclusions` containing
   `symlink_mtime_ns`, matching the producer's symlink timestamp limitation.
   Do not claim fidelity for fields absent from this oracle.
5. After observing the Published result, quit through the production key path.
   Require controlled successful process exit, restoration of the PTY's
   original terminal attributes, and alternate-screen leave output. Compare
   source fingerprints after the run and require them unchanged. On a
   successful run, clean the test-owned parent only after the outcome is known
   to be Published and all assertions have completed. If Execute may have been
   sent but that condition is not met, preserve the parent and report its path
   for inspection.

## Harness, traceability, and verification

- Use the existing PTY acceptance harness and synchronization helpers where
  applicable. Wait for each preceding visible state before sending its next
  key; use bounded deadlines rather than sleeps. On assertion or timeout
  failure, kill/reap the child and preserve enough diagnostics to identify the
  failed stage. Never recursively clean a destination after an unknown or
  indeterminate process outcome; retain it and report its path for inspection.
- Duvet-link the integration test to RESTORE-UI-003, -004, -005, -008, and
  -014. Trace only the preview, acknowledgment, native execution,
  `Published`-result preservation/rendering, and real-PTY evidence actually
  asserted. Do not infer that this one case closes other UI-008 outcome classes,
  all terminal interaction, publication, or formal controller obligations.
- Run the focused Linux PTY acceptance and full `cargo test -p duplicity-tui`
  on Linux, targeted Duvet queries, `cargo xtask trace-update`,
  `cargo xtask trace`, formatting, and strict TUI Clippy. Require hosted Linux
  assurance (`cargo xtask ci`), Duvet Pages publication, and independent
  implementation review before marking acceptance complete. macOS portable
  tests do not count as this Linux publication test.
- Independent review MUST check that a Cargo integration test launches the
  built production executable and worker path, not a unit-test helper; test
  assertions observe the preview and result in the PTY; the exact
  acknowledgment gates exactly one Execute; the Published receipt matches the
  configured destination; filesystem enumeration uses
  raw components and does not follow links; all oracle fields and the symlink
  mtime exclusion are applied accurately; and failure cleanup cannot erase an
  unknown/indeterminate publication.

## Exclusions

This is one successful Linux restore of the official plain profile's final
snapshot with whole-snapshot selection. It does not cover encrypted or
historical profiles, S3, marked-subtree restore, malformed or unsupported
destinations, failed preview/restore outcomes, retries, cancellation races,
terminal I/O failure during restore, alternate filesystems, power-loss or crash
recovery, ACL/xattr/ownership fidelity, symlink mtime, or formal proof of the
browser/backend/filesystem controllers. Existing backend tests and other
acceptances retain their separate responsibilities.

## Checklist

- [ ] Drive the production Linux browser through preview, exact acknowledgment,
      one Execute, and a visible Published result using the official plain
      final-date snapshot.
- [ ] Compare the complete published tree with the independent 21-entry
      oracle, including raw paths, kinds, modes, file size/hash, regular
      file/directory mtimes, and symlink target bytes; omit symlink mtime.
- [ ] Assert destination-parent/staging state, unchanged source, and PTY
      restoration.
- [ ] Focused/full Linux TUI tests, Duvet trace, formatting, and strict Clippy
      pass; hosted Linux assurance and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
