# duplicity-tui

Read-only terminal history and snapshot browser for local directories and S3.
Native Linux restoration is implemented as a software checkpoint with independent frontend acceptance pending. OpenPGP remains unsupported.

```sh
cargo run -p duplicity-tui -- local /path/to/backup
cargo run -p duplicity-tui -- s3 bucket backup-prefix/ --region us-west-2
```

Use Up/Down to select a full manifest and `a` to choose its anchor. Select the
exact target date and press Enter. The browser discloses the required volume
scan; a second Enter starts it. Esc cancels active work or returns to history;
`q` exits. Discovery reads metadata only. Completed reconstruction shows byte
paths, types, ordinary modes and regular-file sizes. All non-ASCII and control
bytes are escaped, including literal escape characters.

Space marks or unmarks the current path as a selection root. `p` previews those
subtrees and their required source ancestors; with no marks it previews the
whole snapshot. Previewing writes no files. Selections are limited to 1024 roots,
4 MiB of cloned path metadata and 16 million path comparisons. Planning runs
synchronously, so a large preview can delay terminal input; cancellation and
executable r preview and publication run on the lifecycle worker.

The reader runs on a worker thread and keeps the original catalog identities
across scans. Cancellation is cooperative; bounded synchronous parsing and
kernel operations have no fixed cancellation latency. S3 connection setup uses
the SDK credential chain before entering the terminal; no credentials are
requested or displayed by the UI. Optional `--profile`, `--endpoint`, and
`--path-style` select SDK configuration. S3 bucket writes are not exposed.

History is limited to 100,000 manifests and a conservative 32 MiB display charge;
snapshot rendering formats only visible rows. Core limits independently apply.
Terminal/backend allocation and rendering remain trusted infrastructure. Very
large trees can require linear traversal to reach a visible row.

Native escaping and CLI tests pass. An automated Unix PTY acceptance test loads
the official 3.0.7 plain fixture, waits for a rendered manifest row, sends `q`,
and verifies successful exit, raw-mode entry/restoration, alternate-screen
teardown, unchanged source contents, and an empty working directory. The earlier
manual PTY smoke loaded the full plus two incremental sets, displayed 21 entries
and 2,621,564 payload bytes, and restored terminal state on exit. This automated
case covers history startup and quit. A second automated PTY case uses a
zero-scrollback terminal screen model to follow the official full plus two
incremental sets, check oracle-derived snapshot counts and byte-path display,
and exercise whole-snapshot and marked-child read-only `p` previews. Its focused
macOS test, independent review and full macOS CI pass. The
[acceptance plan](../../docs/tui-readonly-pty-acceptance.md) records the scope:
the terse `p` preview supplies a count rather than executable-preview details,
and final source content/working-directory comparisons do not exclude transient
writes or access-time changes. Live S3 acceptance and frontend proofs remain open.
A third automated case shrinks the actual PTY from 40x240 to 18x96 and expands
back, checking new frame geometry, retained snapshot/mark/cursor, read-only
preview counts and terminal restoration. Its focused macOS test and independent
review and full macOS CI pass. The
[resize acceptance plan](../../docs/tui-resize-pty-acceptance.md) records the
explicit child `SIGWINCH` stimulus; automatic foreground-group signal delivery,
resize storms and executable-preview wrapping remain outside this case.
Two source-injected PTY cases drive the production browser's `q` and Esc scan
cancellation paths while a test wrapper holds an identity-checked local volume
open. They observe cancellation of that exact scan token, the visible waiting
state before releasing the worker, the combined validation/cancellation result,
and controlled browser return after its worker join. Focused macOS tests, independent review and full macOS CI pass. The default CLI target has seven
parent tests and one ignored internal driver, which both cancellation parents
explicitly execute in separate processes. The
[scan-cancellation plan](../../docs/tui-scan-cancel-pty-acceptance.md) distinguishes
these source-injected browser cases from the actual CLI-binary PTY tests and
leaves active filesystem-restoration cancellation outside their scope.
The output-failure case uses separate input/output PTYs and a gated read-only
scan. It closes only the output master, then sends one `q` on live input. That
key cancels the scan and changes the status; the following production redraw
must fail on closed stdout. The helper checks the structured event and teardown
faults, waits for worker shutdown, and the parent verifies input raw-mode
restoration and unchanged fixture contents. Its focused macOS test and Duvet
trace pass, and independent review is complete. Full macOS assurance CI and
hosted Linux CI for this revised variant remain pending. This covers the
q-triggered status redraw failure only, not close-only failures, active
restoration failure, alternate-screen restoration through failed output, or
live S3. See the
[output-failure acceptance plan](../../docs/tui-output-failure-pty-acceptance.md).
An additional terminal run marked `a/b/child.txt` and previewed exactly four
entries: the file and its three source directory ancestors. Independent source
review accepted the read-only flow after fixing confirmation reset on an invalid
anchor choice.

| Component | Specification review | Implementation | Proof | Integration tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Browser and restore flow | Browse requirements written; restore interaction adopted before implementation | Read-only browsing and native Linux restore work; acceptance pending | Component proofs remain separate; display/I/O trusted | Escaping/CLI tests, official worker preview checks, manual official-chain PTY smoke, and automated Unix PTY history/chain/read-only preview tests pass | Pending |

- [x] Create the executable crate with no unsupported feature claims.
- [x] Escape terminal controls, non-ASCII bytes and literal escapes while retaining original path identity; native tests pass.
- [x] Implement metadata history, explicit anchors/dates, volume-scan disclosure and byte-preserving snapshot navigation.
- [x] Implement byte-preserving selection and bounded restore previews; independently review the read-only flow.
- [x] Add focused official-fixture worker checks for RESTORE-UI-003 whole-snapshot and marked-preview details; both portable preview tests pass on macOS, independent review and full CI passed. The marked case only prepares a preview and does not publish; broader frontend acceptance remains pending.
- [x] Test the RESTORE-UI-002 out-of-range selection error through the worker preview path, including absence of a destination; independent review and full macOS CI passed. This covers one invalid-plan path only.
- [x] Test the RESTORE-UI-011 1,025 repeated valid-root preview limit, confirming rejection before deduplication and no destination; independent review and full macOS CI passed. This covers only the root-count bound.
- [x] Test the configured 20 MiB preview-text cap at the exact boundary: exact-limit appends succeed, and a one-byte overflow is rejected without changing the existing text; independent review and full macOS CI passed. This covers preview-text growth only.
- [x] Test the RESTORE-UI-011 candidate-comparison budget arithmetic at exactly 16,000,000, above the cap, and multiplication overflow; independent review and full macOS CI passed. Both root-copy and preview preparation use this shared checked predicate. This tests budget arithmetic only, without snapshot candidate traversal.
- [x] Test RESTORE-UI-004 key-kind normalization: reported Repeat/Release `r` and Enter events are ignored, while Press `restore` plus Press Enter satisfy the existing acknowledgment policy; independent review and full macOS CI passed. This is a pure key-event test; it does not establish physical-key origin, since legacy terminals may report repeats as Press.
- [x] Add a Unix PTY acceptance test for local fixture history: wait for a rendered manifest row, send `q`, require successful exit, compare raw terminal attributes before/during/after, verify alternate-screen teardown, and confirm the source fingerprint and empty working directory are unchanged. This does not cover preview, acknowledgment, restoration, cancellation, resize, terminal failure or S3.
- [x] Complete the official-chain read-only PTY acceptance case: actual anchor/target/scan-confirmation keys, oracle-derived final snapshot count/payload, visible escaped non-UTF-8 path, whole `p` count and marked `a/b/child.txt` four-entry `p` count, restored terminal state and unchanged final source contents/empty working directory. Independent review and full macOS CI pass. Executable preview, acknowledgment, publication, cancellation, resize, terminal failure and S3 remain outside this case.
- [x] Complete Unix PTY shrink/expand acceptance: actual OS window-size updates and explicit child `SIGWINCH`, fresh 18x96 and 40x240 frames, retained marked child/four-entry preview, then Space unmarks that same child and `p` reports the oracle's whole-snapshot count. Independent review and full macOS CI pass. Terminal restoration and final source-content/empty-working-directory checks remain included; automatic signal delivery, tiny dimensions, resize storms and executable-preview wrapping remain open.
- [x] Complete deterministic production-browser read-only scan cancellation for `q` and Esc in isolated PTYs: initially uncancelled exact scan token, held-worker cancellation/waiting observation, explicit gate release and combined error result, browser-return marker after join, terminal restoration and final source-content/empty-working-directory checks. Both focused macOS parent cases, independent review and full macOS CI pass. The ignored internal child driver has no separate Duvet/gate claim; active restoration, cancellation latency and real blocking-call interruption remain open.
- [x] Test one-shot RESTORE-UI-004 confirmation: the first Press Enter consumes and clears the armed acknowledgment, and a second Enter cannot confirm again; independent review and full macOS CI passed. This exercises the pure confirmation helper only; it does not test worker execution.
- [x] Verify on macOS that ordinary local and S3 CLI configurations parse without a restore destination while native restore flags remain rejected; independent review and full CI passed. This is CLI parsing coverage only.
- [x] Connect explicitly acknowledged selection to native Linux publication; use the existing SDK credential provider.
- [ ] Test remaining display escaping, error and interaction paths.
- [ ] Automate terminal interaction, cancellation, resize and S3 frontend tests. The bounded source-injected q-triggered redraw-output-failure case is implemented and independently reviewed; its focused macOS test and Duvet trace pass, while full macOS assurance CI and hosted Linux CI remain pending.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml).

Native Linux restoration now has an isolated working software checkpoint. Start
with paired `--restore-parent PARENT --restore-name LEAF`; `r` freezes a preview,
then type `restore` and press Enter. `p` remains read-only. Active q/Esc cancels
and waits for a typed outcome; q again after completion exits. No overwrite or
bucket-write path is exposed. Success or uncertainty disables further restores.

- [x] Implement paired raw-OS destination flags and frozen worker previews.
- [x] Connect one acknowledged immutable request to the reviewed native backend.
- [x] Add CLI/lifecycle/outcome and original three-date worker restoration tests.
- [x] Record a real PTY smoke with raw-byte destination and terminal mode restoration.
- [ ] Complete independent frontend software acceptance and formal controller proofs.
- [ ] Add comprehensive terminal-loop fault injection, resize/cancel and live S3 tests.

See [implementation evidence and bounds](../../docs/restore-interaction-implementation.md).
