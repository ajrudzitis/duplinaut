# duplicity-tui

Read-only terminal history and snapshot browser for local directories and S3.
Filesystem restoration and OpenPGP support are still being built.

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
publication remain separate work.

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
| Browser and restore flow | Browse requirements written; restore interaction pending | Read-only browsing works; restore pending | Component proofs remain separate; display/I/O trusted | Escaping/CLI tests and manual official-chain PTY smoke pass | Pending |

- [x] Create the executable crate with no unsupported feature claims.
- [x] Escape terminal controls, non-ASCII bytes and literal escapes while retaining original path identity; native tests pass.
- [x] Implement metadata history, explicit anchors/dates, volume-scan disclosure and byte-preserving snapshot navigation.
- [x] Implement byte-preserving selection and bounded restore previews; independently review the read-only flow.
- [ ] Connect selection to native filesystem publication and implement private credential entry.
- [ ] Test display escaping, errors and interaction flows.
- [ ] Automate terminal interaction, cancellation, resize and S3 frontend tests.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml).
