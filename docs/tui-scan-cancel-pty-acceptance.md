# Read-only scan cancellation PTY acceptance plan

Status: independently reviewed and adopted before implementation. Implementation,
independent review, focused macOS tests and full macOS CI pass.

This adds two deterministic read-only scan-cancellation cases through the
production browser event loop and native terminal lifecycle. It supplies scoped
real-terminal evidence for RESTORE-UI-014 and the reader's no-partial-snapshot
contract, PARSE-ENV-024. It introduces no new normative format requirement.
It does not supply active filesystem-restoration cancellation evidence for
RESTORE-UI-007 or RESTORE-UI-012.

## Process and source arrangement

Use a separate test-only subprocess because crossterm terminal state is global
to a process. That subprocess invokes the public production
`browser::run(Arc<dyn ObjectSource>, prefix, Cancellation::default())`; it uses
neither a modified production CLI nor a special application command-line flag.
Record this as a source-injected production-browser PTY test, distinct from the
existing actual CLI-binary PTY tests.

The child driver is an explicitly ignored internal libtest helper. Parent tests
spawn their current integration-test executable with that helper's exact name,
`--ignored`, `--nocapture`, and `--test-threads=1`, plus dedicated test-only
configuration. Invoking the helper without valid driver configuration fails;
it must never silently pass without running the browser.

The parent tests for `q` and Esc run by default and carry the Duvet test links.
The ignored helper has no Duvet annotation, no assurance-ledger gate entry, and
is never counted as a third accepted scenario. An ordinary full test run has
exactly two new default parent acceptance tests and one ignored internal helper;
each successful parent explicitly executes one child-driver invocation. Record
those counts accurately, including that the helper itself is not an ordinary
zero-work passing test.

Each case owns two distinct temporary directories: an initially empty browser
working directory and a separate IPC control directory. The control directory
contains only fixed, harness-owned coordination files. Publish complete marker
contents before they become observable, and validate each marker's expected phase
and bounded contents before accepting it. Its expected test writes
are excluded from the browser working-directory/source observations. Parent and
child use fixed PTY dimensions and `TERM`, with slave stdin/stdout attached to
the PTY and a bounded current-screen model. No restoration destination is set.

Wrap `LocalSource` for the official
`tests/fixtures/duplicity-3.0.7/plain` fixture. Delegate listing and normal opens
unchanged. On the exact required first full-volume object open, obtain the
identity-checked underlying reader first, then enter the test gate before any
volume bytes are delivered. Discovery and manifest operations remain available.
The wrapper's gate uses a clone of the cancellation token passed to that exact
scan open; the initial discovery token is not a substitute. The driver never
sets that token itself. During the gated acceptance phase, only the production browser's key-handling
path requests cancellation.

## Two-phase gate

Use state-based synchronization, independent of fixture speed:

1. Require that the exact scan token is initially uncancelled, then publish a
   `gate-entered` control marker identifying the required volume key. Wait with a
   bounded deadline for that same token to become cancelled.
2. Once cancellation is observed, publish `cancel-observed`. Keep the open
   blocked until the parent publishes a separate `release` marker. This gives
   the browser time to render its cancellation-requested state without letting
   the worker completion race past it.
3. After release, drop the provisional local reader and return the typed
   `SourceError::Cancelled` from the gated `open`. A deadline, IPC failure or
   unexpected gate reentry is a test failure, never a successful cancellation.

Both waits are bounded. Polling an explicit state/control marker is allowed;
unbounded waits and arbitrary fixture-speed sleeps are not. The release marker
only permits the error to return: it must not cancel the token, change the
source, fabricate a successful reader, or inject a worker/browser reply.

## Parent acceptance sequence

Run this sequence once for `q` and once for Esc, with fresh child, source wrapper,
token and directories:

1. Observe the official history, select the full anchor, choose the final
   incremental target, observe the first-Enter scan disclosure, and send the
   second Enter. Wait for `gate-entered` for the expected full-volume key and
   the visible running-scan state before delivering the cancellation key. Require
   that `cancel-observed` is still absent immediately before injecting that key.
2. Send exactly the case's `q` or Esc key through the PTY. Wait for both the
   `cancel-observed` control marker and the current-screen text
   `Cancellation requested; waiting for operation outcome`. Require that the
   child is still alive and that no browser-return completion marker exists.
   While the gate remains held, the first cancellation key cannot count as a
   successful process exit.
3. Publish `release`. Wait for the current-screen message
   `Snapshot validation failed or was cancelled.` with history still available,
   no `Snapshot paths` panel, and no validated-snapshot success display. The
   blocked required volume plus typed source error establishes the exercised
   failure path; these display checks do not expose an internal snapshot value.
4. Send `q` after that terminal worker result. In the `q` case this is the second
   `q`; in the Esc case it is the subsequent normal-exit `q`. Require controlled
   success and bounded exit, rather than accepting a killed process.
5. Require a child control completion marker written only after
   `browser::run` returned successfully. The marker records the expected gate
   entry, observed token cancellation and released cancellation return. The
   production run path drains and joins its worker before returning; the marker
   distinguishes that returned lifecycle from merely terminating a child with
   a potentially active thread. Do not create it from a destructor, timeout,
   panic handler or unconditional finally-style path.
6. Observe raw terminal mode while the child is active, compare original slave
   terminal attributes after exit, and require alternate-screen teardown.
   Compare recursive fixture names/content/symlink-target fingerprints before
   and after the process, and require the browser working directory remains
   empty. Test IPC directory writes are expected and separately owned.

## Diagnostics and evidence limits

The source gate returns `SourceError::Cancelled`, but the current production
worker maps every `Catalog::read_snapshot` failure to
`Reply::Error { text: "Snapshot validation failed or was cancelled." }`.
The UI intentionally exposes that combined message in this checkpoint; the test
must not claim a cancellation-specific typed UI result or distinguish all
validation errors. A stronger reason-preserving frontend diagnostic would need
its own adopted policy and implementation scope.

Use bounded transcript/control storage, explicit deadlines for every phase,
current-screen assertions instead of historical transcript matches, and parent
kill/reap cleanup plus owned-directory cleanup on every failure. Child-side
failures never write the success completion marker. Keep IPC errors separate
from expected source cancellation. Concurrent default parent tests use distinct
PTYs and unique directories.

Record both focused parent results, independent implementation review, Duvet
trace and required CI results before marking this milestone complete. Neither
ignored-driver counts, annotation counts nor existing core proofs establish
frontend-controller verification. No fixed cancellation latency or interruption
of a real blocking OS call is established: the test source cooperatively waits
on the token. Filesystem fingerprints do not prove absence of transient writes
or metadata changes such as access times.

This case does not cover active restoration, publication outcomes or races,
preview acknowledgment, arbitrary channel/terminal/output faults, discovery
cancellation, live S3, encrypted credentials or formal browser controller
obligations.

## Accepted evidence

The plan was adopted at `901d1ae` before implementation. The parent tests and
ignored driver were independently reviewed and rerun on macOS; full CI passed at
the implementation commit. On 2026-11-03, unfiltered `cargo xtask ci` passed on
aarch64 macOS at `SOURCECOMMIT`, source SHA-256
`fa22169e395a2a5095ae6f6d1678939b554f3091a224f82194ca14e35d5ab4e0`.
Fresh isolated verification reported 2,047 vstd, 3,183 Vest and 1,274 core
obligations with zero errors. Duvet, formatting, strict Clippy, all workspace
tests/doctests and exact named-test replay passed. The CLI target ran seven
default tests and one ignored internal driver; both parent cancellation tests
each explicitly ran a separate configured helper process. The ignored helper is
not independently counted as passing coverage. This evidence covers scan
cancellation in the source-injected production browser, not filesystem-restore
cancellation or frontend controller proofs.
