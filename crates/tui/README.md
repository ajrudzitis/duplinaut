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

Native escaping and CLI tests pass. A real PTY smoke run loaded the official
3.0.7 plain fixture through full plus two incremental sets, displayed 21 entries
and 2,621,564 payload bytes, and restored terminal state on exit. This does not
establish automated interaction coverage, live S3 acceptance or a frontend proof.
An additional terminal run marked `a/b/child.txt` and previewed exactly four
entries: the file and its three source directory ancestors. Independent source
review accepted the read-only flow after fixing confirmation reset on an invalid
anchor choice.

| Component | Specification review | Implementation | Proof | Integration tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Browser and restore flow | Browse requirements written; restore interaction adopted before implementation | Read-only browsing and native Linux restore work; acceptance pending | Component proofs remain separate; display/I/O trusted | Escaping/CLI tests, official worker preview checks and manual official-chain PTY smoke pass | Pending |

- [x] Create the executable crate with no unsupported feature claims.
- [x] Escape terminal controls, non-ASCII bytes and literal escapes while retaining original path identity; native tests pass.
- [x] Implement metadata history, explicit anchors/dates, volume-scan disclosure and byte-preserving snapshot navigation.
- [x] Implement byte-preserving selection and bounded restore previews; independently review the read-only flow.
- [x] Add focused official-fixture worker checks for RESTORE-UI-003 whole-snapshot and marked-preview details; both portable preview tests pass on macOS, independent review and full CI passed. The marked case only prepares a preview and does not publish; broader frontend acceptance remains pending.
- [x] Test the RESTORE-UI-002 out-of-range selection error through the worker preview path, including absence of a destination; independent review and full macOS CI passed. This covers one invalid-plan path only.
- [x] Test the RESTORE-UI-011 1,025 repeated valid-root preview limit, confirming rejection before deduplication and no destination; this covers only the root-count bound.
- [x] Test the configured 20 MiB preview-text cap at the exact boundary: exact-limit appends succeed, and a one-byte overflow is rejected without changing the existing text. This covers preview-text growth only.
- [x] Verify on macOS that ordinary local and S3 CLI configurations parse without a restore destination while native restore flags remain rejected; independent review and full CI passed. This is CLI parsing coverage only.
- [x] Connect explicitly acknowledged selection to native Linux publication; use the existing SDK credential provider.
- [ ] Test remaining display escaping, error and interaction paths.
- [ ] Automate terminal interaction, cancellation, resize and S3 frontend tests.

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
