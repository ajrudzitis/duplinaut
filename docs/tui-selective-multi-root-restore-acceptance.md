# TUI selective multi-root restore acceptance plan

Status: proposed; specification only. This adds a Linux production-PTY case
for restoring multiple marked roots with distinct ancestor chains.

## Goal and requirements

Drive the actual Linux TUI through the official plain fixture history, mark
exactly two files on separate branches, inspect the executable preview, enter
the exact acknowledgment, and restore through the native backend. Compare the
Published result and complete output tree with the selected subset of
`expected.json`, including required ancestors and the absence of unselected
entries.

This traces to [RESTORE-UI-003](parsing-requirements.md#restore-ui-003)
(selection, ancestors, count and bytes in the concrete preview),
[RESTORE-UI-004](parsing-requirements.md#restore-ui-004) (exact acknowledgment
and one operation), [RESTORE-UI-005](parsing-requirements.md#restore-ui-005)
(native backend execution), [RESTORE-UI-008](parsing-requirements.md#restore-ui-008)
(known Published result), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (real PTY and native
worker/result evidence). UI-008 coverage is limited to a known Published
result. This case does not establish full controller correctness.

## Fixture and oracle facts

Use `tests/fixtures/duplicity-3.0.7/plain`, target timestamp `1700172800`,
`mode = "plain"`, `index = 2` in
`tests/fixtures/duplicity-3.0.7/expected.json`. Mark exactly these two raw
component paths:

| Selected path components | Oracle record | Required ancestors |
| --- | --- | --- |
| `[b"a", b"b", b"child.txt"]` | regular file, 16 bytes, SHA-256 `0d52c47f7783d2887fc36b0ab7657533cad3b748e12793a13837b19e9ff80965` | snapshot root, `a/`, `a/b/` |
| `[b"a!", b"sibling.txt"]` | regular file, 12 bytes, SHA-256 `ad319e8ad90014e64d573d16f3594fcf1fe52fe767cea9f4e5caf2b9b17fcd2b` | snapshot root, `a!/` |

The exact selected restore plan contains six entries: the snapshot root, the
four required directories listed above, and the two regular files. It contains
28 regular payload bytes. `expected.json` gives mode `0755` and
`mtime_ns = 1700172700000000000` for the root and all four directories; both
files have mode `0644` and the same exact nanosecond mtime. These values use
oracle numeric modes 493 and 420 respectively. No selected entry is a symlink,
so no symlink timestamp exception applies.

## Process and destination setup

1. Implement this as a Linux-only Cargo integration test in
   `crates/tui/tests/cli.rs` or an equivalent integration test. Launch the
   built production executable with `CARGO_BIN_EXE_duplicity-tui` (or Cargo's
   equivalent path), not a browser/unit-test helper. Use a unique test-owned
   parent on an allowed filesystem, a fresh absent destination leaf, and
   `TERM=xterm-256color` with a bounded PTY geometry large enough to inspect
   the preview. Do not use S3, credentials, encryption, or modified producer
   fixtures.
2. Capture the source fixture's raw paths, file hashes, symlink targets,
   modes, and supported timestamps before launch. Require unchanged source
   identity/content after the run. Use bounded PTY output, explicit deadlines,
   and child kill/reap handling.
3. Cleanup is permitted before Execute is sent after child reaping. Once
   Execute could have been sent, preserve and report the test-owned paths on a
   timeout, child failure, or result other than confirmed Published with all
   assertions complete.

## Acceptance sequence

1. Through the production PTY, choose the official full anchor and target and
   confirm the required full scan. On the validated snapshot, navigate by the
   visible raw-path rows and mark exactly `a/b/child.txt` and
   `a!/sibling.txt`. Require both selected paths to be visibly identified and
   the marked-root count to equal two; no other path may be marked.
2. Press `r`. Before acknowledgment, require a successful executable preview
   for target `1700172800` that identifies both selected paths, marked-root
   scope, six resulting entries, 28 regular payload bytes, the exact included
   ancestor policy, requested destination, no-replacement behavior, supported
   metadata policy, and backend-validation caveat. Use production Up/Down
   scrolling when necessary; prove the required visible text in current parsed
   PTY screens rather than only in a retained output transcript.
3. Require the preview to begin with an empty/unarmed acknowledgment. Press
   Enter once while empty; require no Execute/running or Published state and
   an absent destination. Type exactly ASCII `restore`, require the armed
   display, and press Enter exactly once. Do not send a retry.
4. Require the production worker to return and the UI to retain/render
   `RestoreOutcome::Published` for the requested leaf. The integration test
   MUST use the actual frontend worker/backend path; it MUST NOT construct a
   replacement restore plan or call the backend directly.
5. Walk the published destination using raw path components and
   `symlink_metadata`, recursing only into entries known to be directories.
   Require exactly the six oracle-derived entries listed above, with no
   symlink or other entry. For every entry, require exact kind and
   `metadata.mode() & 0o777 == oracle.mode`. For files, require exact size,
   SHA-256, and signed nanosecond mtime. For directories, require exact
   nanosecond mtime. Compute nanoseconds losslessly as
   `i128::from(metadata.mtime()) * 1_000_000_000 +
   i128::from(metadata.mtime_nsec())` and compare to signed `oracle.mtime_ns`;
   do not round to whole seconds. Require the 16-byte and 12-byte file
   contents to match their oracle hashes.
6. Confirm that all other snapshot entries are absent from the published
   tree, including `a!/` descendants other than `sibling.txt` and all
   unrelated top-level entries. Require no `.duplicity-*` staging residue in
   the destination parent and an unchanged source fingerprint.
7. Quit through the production key path after the Published result is
   visible. Require successful controlled process exit, original PTY termios
   restoration, and alternate-screen leave output. Clean the test-owned
   destination only after Published and all assertions pass. If Execute may
   have been sent but the final outcome is not known to be Published, preserve
   and report the test path for inspection.

## Duvet and verification

- Duvet-link the integration test only to RESTORE-UI-003, -004, -005, -008,
  and -014. The annotations must identify a real production PTY preview,
  exact two-root selection, one acknowledged backend operation, known
  Published result, and filesystem subset oracle. Do not imply coverage of
  other outcome classes, all UI-008 behavior, or formal controller proof.
- Run the focused Linux PTY test and full `cargo test -p duplicity-tui` on
  Linux, targeted Duvet queries, `cargo xtask trace-update`, `cargo xtask
  trace`, formatting, and strict TUI Clippy. Require hosted Linux assurance
  (`cargo xtask ci`), Duvet Pages publication, and independent implementation
  review. macOS portable tests do not count as Linux native publication
  evidence.
- Independent review MUST verify the fixture paths/count/bytes/modes/times
  against `expected.json`; exactly two roots were marked; the preview and
  Published result were observed through the production PTY; the output tree
  equals the oracle-derived subset with no extra entries; and cleanup cannot
  erase an unknown publication result.

## Exclusions

This is one successful selective restore of two regular files from the final
plain fixture snapshot. It does not cover whole-snapshot restore, overlapping
or ancestor-selected roots, symlink selections, raw non-UTF-8 selections,
historical or encrypted profiles, S3, failed/indeterminate outcomes, retry,
cancellation, terminal output failure, alternate filesystems, crash
durability, ACL/xattr/ownership fidelity, or formal controller proofs.

## Checklist

- [ ] Mark and preview exactly the two specified roots through the production
      Linux PTY; assert six entries, 28 payload bytes, and required ancestors.
- [ ] Enter the exact acknowledgment once and observe the typed Published
      result through the actual worker/backend path.
- [ ] Compare the exact six-entry restored tree, file hashes, modes, mtimes,
      and absence of all unselected entries; verify source and terminal state.
- [ ] Focused/full Linux TUI tests, Duvet trace, formatting, strict Clippy,
      hosted assurance, and Duvet Pages pass.
- [ ] Independent implementation review accepts the scoped evidence.
