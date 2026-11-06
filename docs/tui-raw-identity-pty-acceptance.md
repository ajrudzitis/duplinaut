# TUI raw destination identity PTY acceptance plan

Status: accepted. The production Linux PTY case, full assurance gate, and Duvet
Pages deployment passed in [run 41892380922](https://github.com/ajrudzitis/duplinaut/actions/runs/41892380922).
This adds real Linux terminal evidence that raw destination and selected-path
bytes remain identities while their preview and successful receipt render
reversible, terminal-safe escapes.

## Goal, requirements, and prerequisite

Use the production browser and native worker through a Linux PTY to restore one
official raw-byte path. Supply a destination leaf containing a control byte,
backslash, and invalid UTF-8; observe their reversible spellings in both the
executable preview and the Published receipt, then verify the original bytes
identify the actual filesystem destination.

This acceptance traces to [RESTORE-UI-001](parsing-requirements.md#restore-ui-001)
(valid raw OS destination input), [RESTORE-UI-010](parsing-requirements.md#restore-ui-010)
(byte-preserving safe rendering), and
[RESTORE-UI-014](parsing-requirements.md#restore-ui-014) (real terminal
evidence). These are scoped links: UI-001 coverage is only the one valid raw
leaf, UI-010 coverage is only the exercised preview and Published identity,
and the case does not close either full requirement or prove a controller.

The accepted [successful restore PTY case](tui-restore-success-pty-acceptance.md)
MUST pass first. Reuse its Linux executable/PTY harness, ordinary acknowledgment
flow, terminal restoration checks, and known Published cleanup boundary. This
case narrows the selection and changes the destination identity; it does not
repeat the successful case's full 21-entry oracle matrix.

## Fixture and exact byte expectations

1. Use a Linux Cargo integration test launching the built production binary
   via `CARGO_BIN_EXE_duplicity-tui` (or Cargo's equivalent path), not a unit
   helper. Use `tests/fixtures/duplicity-3.0.7/plain`, fixed
   `TERM=xterm-256color`, a unique test-owned `0700` parent on an allowed
   filesystem, and no S3 or encryption.
2. Set `--restore-name` from raw bytes `[0x1b, b'\\', 0xff, b'R']` using
   `OsStringExt`; the expected visible escape is exactly `\x1b\\\xffR`.
   Verify the byte sequence is nonempty, contains neither NUL nor slash, and
   the destination is absent before launch. Do not round-trip through UTF-8 or
   parse the visible escape back into an argument.
3. Select the official plain final snapshot at `1700172800` and mark only its
   raw path component `b"raw-\xff.bin"`, verified from
   `tests/fixtures/duplicity-3.0.7/expected.json` (`mode = "plain"`, `index = 2`,
   `path_hex = "7261772dff2e62696e"`). Its oracle record is a 9-byte regular
   file with SHA-256
   `83e33998a9bc776bbe6bf39220f517734c5dc8b14168817e65b2a3b6edfee5f8`.
   The expected restore plan contains this file and the source root directory.

## Acceptance sequence

1. Through the PTY, select the full anchor and final snapshot, locate and mark
   the raw-byte file above, and invoke `r`. Require the preview to show the
   selected source path as `raw-\xff.bin`, the requested parent, and the exact
   destination escape `\x1b\\\xffR`. Use the production Up/Down scroll
   controls to observe each identity field if the viewport clips the preview;
   do not rely on a raw transcript as proof of current rendered text. Require
   the preview's entry count and payload bytes to match the two-entry, 9-byte
   plan, and observe its no-replacement and supported metadata policy.
2. Require the acknowledgment to be initially empty/unarmed. An Enter before
   acknowledgment MUST leave the destination absent and start no Execute.
   Type exactly ASCII `restore`, observe the armed state, and press Enter once.
   Wait for either the production running frame or direct Published completion;
   a fast restore may finish before an intermediate frame is observable. Send no
   retry.
3. Wait for a retained `Published` result. Scroll the production result document
   to its `Destination:` receipt field and require the exact same visible
   escape `\x1b\\\xffR`. The test MUST separately establish raw identity:
   enumerate the test-owned parent using `OsStrExt::as_bytes` and require its
   only child name to equal `[0x1b, b'\\', 0xff, b'R']`. Verify that the
   published root contains exactly the expected raw component `b"raw-\xff.bin"`
   and that its regular-file size and SHA-256 match the oracle. Inspect with
   `symlink_metadata`; do not follow links. The selected root is a regular
   file, so no symlink-mtime policy is involved.
4. Quit through the production key path after Published is visible. Require
   successful exit, original PTY termios restoration, and alternate-screen
   leave output. Require the source fixture fingerprint unchanged and no
   `.duplicity-*` staging entry in the destination parent. Clean the parent
   only after Published and all assertions pass. If Execute could have been
   sent but the outcome is unknown or not Published, preserve and report the
   test path for inspection.
5. Inspect the parsed terminal screen, not only the emitted byte stream, and
   require visible identities to contain the exact ASCII escapes. Neither the
   raw ESC byte nor the invalid UTF-8 byte from the destination may appear as
   rendered identity content. Ordinary terminal protocol control sequences
   remain expected and are outside this identity assertion.

## Duvet and verification

- Duvet-link the Linux integration test to RESTORE-UI-001, -010, and -014.
  Annotate that it proves one valid raw-byte CLI leaf, exact escaped rendering
  in preview and Published receipt, and raw-byte filesystem identity. Do not
  claim all diagnostics, all destination/selected-path cases, or complete UI
  requirement coverage.
- Run the focused Linux integration test and full `cargo test -p
  duplicity-tui`, targeted Duvet queries, `cargo xtask trace-update`,
  `cargo xtask trace`, formatting, and strict TUI Clippy. Require hosted Linux
  assurance (`cargo xtask ci`), Duvet Pages publication, and independent
  implementation review. macOS portable checks do not count as this Linux
  native publication case.
- Independent review MUST verify the integration test launches the production
  binary; argv uses the raw OS byte leaf; expected rendering is computed from
  the documented `display::bytes` rule; both preview and Published receipt
  are observed through the PTY screen; actual directory enumeration confirms
  exact original bytes; and cleanup preserves any possibly published tree if
  the outcome is unknown.

## Exclusions

This is one successful Linux restore of one official regular file from the
plain profile. It does not test every control/non-UTF-8 byte, a raw parent
pathname, symlink names or targets, staging/candidate escaping, error
diagnostic escaping, unavailable or uncertain outcomes, S3, encrypted
profiles, retries, cancellation, terminal output failure, or formal
controller proofs. Those identities and behaviors retain separate
requirements and acceptance obligations.

## Checklist

- [x] Restore the official raw-byte file through the production Linux browser
      PTY using the exact raw destination leaf.
- [x] Assert exact reversible destination and selected-path escapes in the
      rendered preview and Published receipt, and exact raw filesystem names.
- [x] Verify the one-file oracle, unchanged source, staging absence, and
      terminal restoration; preserve paths on uncertain outcomes.
- [x] Focused/full Linux TUI tests, Duvet trace, formatting, strict Clippy,
      hosted assurance, and Duvet Pages pass.
- [x] Independent implementation review accepts the scoped evidence.
