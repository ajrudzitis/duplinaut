# Native terminal restoration interaction

Adopted profile, written and independently reviewed before frontend restoration implementation. The
canonical normative requirements are RESTORE-UI-001 through RESTORE-UI-014 in
[parsing requirements](parsing-requirements.md#native-terminal-restoration-interaction).
This profile composes the reviewed [read-only selection](restore-plan.md) with
the adopted [native Linux backend](restore-linux-implementation.md). Its frontend
implementation, interaction tests and formal state-machine evidence are pending.
Existing backend native acceptance does not establish frontend acceptance.

## Initial destination configuration

```sh
cargo run -p duplicity-tui -- local /backups \
  --restore-parent /home/aleks/restored --restore-name project
cargo run -p duplicity-tui -- s3 bucket backup-prefix/ \
  --region us-west-2 --restore-parent /home/aleks/restored --restore-name project
```

Both destination flags are optional as a pair and apply to either source mode.
The destination is a **new leaf** under an existing caller-chosen parent. The
frontend uses OS strings, preserving raw Linux bytes; shell quotes and expansions
have already been applied before argument acquisition. It never interprets the
terminal's displayed `\xNN` escaping as input syntax. The paired values are fixed
for this process. There is no in-terminal path editor, overwrite option or startup
restore. Starting without the pair retains browsing and `p` read-only previews;
`r` explains the missing configuration without writing.

Use `args_os` for parsing. Option tokens/S3 API strings require valid UTF-8 where
needed, while local and restore paths retain OS strings. Consume a flag's next
OS-string argument as its value, including a value starting with `--`; reject
unknown option tokens, duplicate restore options, missing values and incomplete
pairs. Accept standalone flag/value spelling initially; an `--option=value`
alias is not required. On Linux validate the leaf as a raw single component and
cap parent/name values at 65,536/4,096 bytes before copies. On another OS report
native restoration unsupported rather than attempting lossy conversion or a
weaker backend. CLI and help diagnostics must remain safe outside raw-terminal
mode and disclose actual supported capabilities.

The preview labels the parent path as **requested**, because the backend anchors
the opened parent FD and its later pathname may be rebound. Opening a parent,
checking policy/filesystem/destination, selecting the automatic profile and
validating exact metadata remain backend work after explicit confirmation and
before staging. No separate frontend preflight promises eligibility or bypasses
those checks. The default backend limits apply without override in this profile.

## Selection and confirmation

A completed immutable `ValidatedSnapshot` supplies the payloads and metadata.
Space marks byte-path roots. No marks explicitly selects the whole snapshot.
Selected subtrees include exactly their core-plan source directory ancestors;
there is no implicit flattening or renamed extraction. `p` keeps its existing
read-only selection summary. `r` freezes the selection and prepares the executable
preview on a worker. A failure to prepare it never arms Enter.

The preview identifies the snapshot target, marked/all scope, each selected root,
required-ancestor policy, entry count, total regular payload bytes, requested
parent/new leaf and no-replacement publication. It explains ordinary-mode/time/
raw-link fidelity and ownership/ACL/xattr/crash-durability exclusions. Empty plans
without source-root metadata identify policy root mode `0700` and unspecified root
mtime. Full escaped values are accessible through bounded scrolling; viewport
clipping is not identity truncation. Preview scrolling changes only its viewport.

The ready preview displays an initially empty, dedicated acknowledgment field:
**type `restore`, then Enter**. Consume/ignore acknowledgment and confirmation input
while preparation is active. Before ready-preview input is armed, drain pending
terminal events until a nonblocking poll observes none; a bounded drain can take
multiple render-loop iterations and must not arm while buffered input remains.
Display the concrete preview and empty field before accepting its acknowledgment.

Accept at most seven ASCII bytes, with Backspace editing only this fixed word.
Enter with an empty or mismatched value does nothing except explain the required
acknowledgment. Character shortcuts such as `r` become acknowledgment characters
inside this view, not a second preview request. Ignore events reported as Repeat
or Release. Legacy Unix terminals can report repeated keys as Press without event-
type enhancement: the profile claims an observed acknowledgment sequence, not
physical-fresh-key/typing-origin detection. Held Enter or a single held character
cannot supply the exact multi-character word. No terminal enhancement is required.

Enter after the exact field value moves the one frozen request to the worker and
immediately clears/disarms it. Enter in history remains only the disclosed scan
confirmation. Selection, source/snapshot or destination changes, leaving the
preview, errors and completed operations clear the field/disarm restoration.
Returning requires a new `r`, empty field, `restore`, then Enter. No acknowledgment
carries across views or replies. While restoration runs, selection/navigation/
source replacement and new scans are disabled. Status identifies running or
cancellation-requested states without invented progress percentages/ETAs.

| State | User action / event | Result |
| --- | --- | --- |
| Snapshot | `p` | Read-only plan preview; no confirmation |
| Snapshot with destination | `r` | Freeze exact snapshot/roots/destination; worker prepares preview |
| Preparing preview | Esc or `q` | Request cancellation; consume completion without arming restoration |
| Executable preview | Exact `restore`, then Enter | Move one frozen request to worker; clear field/disarm immediately |
| Executable preview | Empty/mismatched field + Enter | Remain in preview; no restoration |
| Executable preview | Esc, marks or snapshot navigation | Disarm; return to snapshot before changing selection |
| Restoring | Esc or `q` | Request cancellation; remain until result; `q` again after result exits |
| Published | Completion | Show success plus cleanup warnings; no second restore to fixed destination |
| Definitely unpublished | Completion | Show cause and cleanup details; a new `r`/`restore`/Enter may start a new attempt |
| Unpublished with uncertain first leaf | Completion | Show candidate uncertainty; disable further restores in session |
| Publication indeterminate | Completion | Show attempted-publication uncertainty; disable retries/recovery |
| Worker result unavailable | Panic/disconnection | Show outcome unavailable/inspection required; disable retries/recovery |

A request arriving after the backend's last cancellation check can race with
publication. A published receipt remains published despite that request; an
attempted rename error remains indeterminate. Neither the UI token nor a completed
payload counter decides the result.

## Worker ownership and shutdown

The existing source worker owns its immutable catalog; the terminal currently
owns each reconstructed snapshot and performs `p` planning synchronously. A
minimal implementation can change snapshot ownership to `Arc<ValidatedSnapshot>`
and introduce one lifecycle-controlled restore/preview worker. It need not change
the source contract or move payload bytes into a second state.

The frozen request owns the only required root copy and destination configuration
reference/copy. The worker creates a borrowed plan from that same immutable
snapshot. It may return a summary and frozen request for later confirmation;
executing confirmation recreates the same deterministic plan and checks summary
counts/payloads before calling the backend. Avoid self-referential owning plans
or a second deep snapshot clone. Requests/replies have capacity one, with at most
one active scan, preview or restore. Explicit operation identity ensures a delayed
reply cannot arm, replace or finalize another operation. Preview cancellation and
worker failure consume/release their request; later operations use a new token.

Plan construction remains synchronous bounded work on that worker. It observes
cancellation before and after preparation; the core plan does not promise internal
interruptibility. Backend work uses the operation-specific token. Polling the
terminal remains responsive while a kernel operation can still block indefinitely
within the stated trusted I/O boundary. No fixed cancellation-latency guarantee is
introduced.

For a controlled terminal I/O failure during a restore, request cancellation,
close all frontend-owned request senders or signal equivalent worker shutdown,
attempt terminal teardown, then drain the reply/equivalent result path before
joining. Merely joining can deadlock if the worker is blocked sending a result or
waiting for another request. Do not detach it and exit merely because the terminal
failed.

Preserve a known typed backend result, then attempt escaped reporting through an
available non-raw fallback such as stderr. Use fallible output; report/return
teardown and output faults separately. The terminal and fallback can both fail,
so successful external delivery is conditional on an available output sink. A
printing failure does not convert a known published/unpublished/indeterminate
outcome into uncertainty. If the worker itself panics/disconnects without a final
result, state that its outcome is unavailable; do not assert no staging or
publication occurred. Abort/fatal-signal/crash boundaries remain held, not
handled-shutdown guarantees. No automatic receipt-file fallback, filesystem
cleanup or retries arise from a frontend/channel/output error.

## Result presentation and bounds

The frontend matches the exact `RestoreOutcome` variants, not a generic success/
error string. Published results retain profile, filesystem magic, parent/root
device-inode identities, destination/staging bytes, empty-root policy and cleanup
warnings. Unpublished results show original cause, attempted cleanup counts,
first errno, budget/truncation flags and any uncertain outer candidate. An
indeterminate result retains attempted destination/staging/identities and errno,
explaining that publication may have occurred. Read-only status observations must
not promote that result or authorize deletion.

Escape every control/non-ASCII byte as `\xNN`, double literal backslashes and retain
other printable ASCII. Display text never becomes a syscall path. Restore result
rendering uses typed scalar fields and byte names rather than dumping credentials,
SDK configuration/errors, environment, argv or file contents. Initial restoration
needs no further repository request and never writes a source bucket. There is no
automatic receipt file, clipboard operation, upload or private credential entry.

Independent initial frontend bounds are 1,024 roots, 4 MiB owned root capacity,
16 million plan comparisons, 20 MiB escaped preview text and 1 MiB final rendered
diagnostics, in addition to CLI parent/name limits and bounded queues. Capacity,
checked sums and up-to-fourfold byte escape expansion are charged before allocation.
All simultaneous root copies count, so a move/return ownership design is preferable.
Fail explicitly rather than truncate a required path or receipt. Rendering clips
only a viewport while retaining accessible bounded full values. The existing
snapshot/catalog and backend limits remain separate; no total heap proof follows
from these frontend caps. No per-chunk progress callback is needed for this slice.

## Acceptance before implementation completion

- [x] Coordinator and independent contract review of this profile/14 requirements.
- [ ] Implement paired raw-OS CLI options, immutable requests and confirmation states.
- [ ] Native CLI/state tests: bytes, bounds, missing/duplicate flags, invalid leaf,
      stale replies/previews, acknowledgment reset/mismatch/buffer draining and held
      Enter/single-character repeats; no writes without the complete confirmation.
- [ ] Worker/result tests: one active operation, cancellation/publication race,
      all outcomes/warnings, uncertain candidate, result loss and controlled
      terminal-error request closure/draining/joining without channel deadlock,
      and output failure preserving a known typed result.
- [ ] Original official snapshot restoration through the frontend worker with the
      existing independent content/metadata oracles and backend profile constraints.
- [ ] Real PTY test/smoke evidence: preview text/key sequence, filesystem publication,
      cancel/results, raw-byte names and terminal restoration.
- [ ] Duvet implementation/test links and separately recorded controller proof scope.

The native Linux backend already has separately reviewed integration/fault tests;
that evidence remains distinct from these new frontend obligations. Complete
filesystem, selection, lifecycle and frontend-controller proofs remain pending.
A PTY smoke run alone is not comprehensive interaction assurance.

Primary API references: [Rust args_os](https://doc.rust-lang.org/std/env/fn.args_os.html),
[Unix OsStrExt bytes](https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html),
[thread ownership/joining](https://doc.rust-lang.org/std/thread/fn.spawn.html),
[bounded synchronous channels](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html),
and [crossterm 0.29.0 key kinds and reporting limits](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyEvent.html#structfield.kind).
These document stable APIs; the pinned Rust/tool identities remain authoritative.
The product flow, frontend bounds and initial capability restrictions are project
policy composed with RESTORE-001..027, not claims mandated by duplicity.
