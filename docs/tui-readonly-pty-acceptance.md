# Read-only snapshot PTY acceptance plan

Status: acceptance scope independently reviewed and adopted before implementation;
execution and implementation review are pending.

This extends the existing automated local-history/quit test through the actual
browser event loop and the official full-plus-two-incremental chain. It applies
the existing [RESTORE-UI-002, RESTORE-UI-004, RESTORE-UI-013 and
RESTORE-UI-014 requirements](parsing-requirements.md#native-terminal-restoration-interaction)
to one read-only interaction case. UI-004 applies here only to its read-only `p`
and distinct history-Enter behavior. It introduces no new normative requirements.

## Fixture and independent expectations

Launch the actual `duplicity-tui local` binary in a Unix PTY against
`tests/fixtures/duplicity-3.0.7/plain`, using a new empty working directory and no
restoration destination. The independently recorded producer/source oracle is
`tests/fixtures/duplicity-3.0.7/expected.json`: select `mode = "plain"`,
`index = 2`, time `1700172800` (2023-11-16 22:13:20 UTC). Its 21 entries contain
2,621,564 regular-file payload bytes. Derive expected counts from that oracle,
rather than from the reader or its `RestorePlan` output.

The marked root is the original component sequence `a`, `b`, `child.txt`. The
oracle identifies one file of 16 bytes and its three directory ancestors:
the source root, `a`, and `a/b`. The expected selected plan therefore has four
entries. The sibling `a!/sibling.txt` is outside that selection.

## Acceptance sequence

1. Wait for a rendered official full-manifest history row. Select that full set
   as the anchor with `a`; observe the anchor-selection acknowledgment.
2. Navigate to the final incremental target and press Enter once. Observe the
   explicit full-volume-scan disclosure and its second-Enter instruction before
   sending the second Enter. This checks the visible confirmation sequence; it
   does not independently instrument which source objects were read.
3. Confirm the scan. Wait for a rendered validated snapshot reporting the oracle
   entry count and regular-payload total. Check the displayed snapshot path list
   includes `a/b/child.txt` and the escaped non-UTF-8 path `raw-\xff.bin`.
4. With no marks, press `p` and observe a read-only preview count equal to all 21
   oracle entries, with the displayed statement that nothing is written.
5. Navigate to `a/b/child.txt`, mark it with Space, and observe one marked root.
   Press `p` and observe the four-entry read-only preview including source
   ancestors. The count and selected path are the scope of this UI observation;
   this terse `p` preview does not list each planned ancestor or excluded sibling.
   Existing worker/core tests supply that separate membership evidence.
6. Quit with `q`, require successful controlled exit, and verify the original
   slave terminal attributes are restored and alternate-screen leave is emitted.
   Check that active terminal attributes differ from the initial attributes.
7. Compare recursive source names/content/symlink-target fingerprints before and
   after the process, and require the working directory remains empty. These
   observations do not establish the absence of transient writes or metadata
   changes such as access times.

## Harness and evidence bounds

Use fixed PTY dimensions and `TERM`, bounded transcript storage, explicit
deadlines for each visible state and exit, and child kill/reap cleanup on every
test failure. Synchronize inputs with observations of their preceding states;
do not use unbounded sleeps or accept a marker left over from an earlier state
as evidence of a later transition. Terminal redraws may update only changed
cells, so assertions must account for the chosen transcript/screen model.

Record the focused Unix test result, independent implementation review, Duvet
trace result and required CI result before marking the task complete. Link the
test to RESTORE-UI-014; any additional links represent only the tested portions
of their requirements. In particular, `p` supplies no executable destination,
metadata-policy display, acknowledgment or publication evidence for the full
RESTORE-UI-003 contract.

This case covers local plain/gzip reconstruction and read-only selection through
the real terminal loop on supported Unix hosts. Executable `r` previews,
acknowledgment, Linux publication/outcomes, cancellation races, resize, terminal
fault injection, S3, encrypted credential flow and formal frontend controller
proofs remain separate backlog work.
