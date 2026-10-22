# Native terminal restoration checkpoint

The adopted [interaction contract](restore-interaction.md) preceded this software.
The executable accepts paired raw-OS `--restore-parent` and `--restore-name` values
for local and S 3 modes. No destination pair means browsing/read-only previews;
there is no startup restoration, overwrite switch, destination editor, credential
entry or bucket write. Parent/name byte limits and structural restrictions fail
before copying destination values. OS argument storage remains trusted infrastructure.
Public args_os, bounded-channel and crossterm key-kind documentation was consulted;
the pinned Rust toolchain and crossterm 0.29.0 remain authoritative.

One lifecycle-owned worker retains the immutable source catalog and processes
scans, executable previews and restores serially. Capacity-one request/reply queues
and strictly increasing operation IDs prohibit a second active command or stale
reply from finalizing another command. Snapshots use Arc; payload bytes are never
cloned. The UI freezes marked indices, snapshot and immutable destination. The
worker charges before cloning original root components, builds the core plan,
checks counts/payload sum and returns one owned Frozen request plus safe preview.
Confirmation moves that request back without cloning roots; execution rebuilds
that same deterministic borrowed plan and compares its summary before calling the
backend with default limits. No source access is needed for execution.

`p` remains bounded read-only planning. `r` prepares a concrete executable preview
on the worker, including target, marked/all roots, exact required ancestor policy,
counts/payloads, requested parent/new leaf, no-replacement/fidelity policy and
empty-tree metadata policy. Backend eligibility/profile selection remains backend
work after acknowledgment. Full ASCII-escaped preview and result values wrap to
viewport-width segments and scroll; no identity is shortened for display.

Every successful preview starts with a dedicated empty seven-byte acknowledgment.
The concrete preview and empty field render while unarmed. At most 64 events drain
per iteration; a zero-duration poll must see no pending input before arming.
Preparation input and reported Repeat/Release events are ignored. Exact observed
ASCII `restore` then Enter moves the frozen request, clears/disarms immediately and
starts one operation. Legacy Unix Press events cannot prove physical fresh typing.
Empty/mismatched Enter, held Enter or repeated single-character Press events cannot
supply the word. Esc/q abandons a ready preview. Active q/Esc requests cancellation
and waits for its completion; q again after the result exits. Navigation/selection
and scans are disabled during active work. Cancellation does not override typed
published success or publication uncertainty.

Outcomes are matched as typed backend variants. Published success, uncertain outer
creation, indeterminate publication or missing worker results disable session
restoration retries; definitely unpublished results require an entirely new preview
and acknowledgment. Every byte name is escaped, and errno formatting uses integer
scalars rather than locale-dependent OS text. Receipts retain exact profile,
filesystem, identities, staging/destination names, empty-root policy and cleanup
warnings. Unavailable results retain the configured requested destination and
operation metadata without claiming no staging/publication occurred.

The frontend owns a fallible terminal lifecycle. The pinned ratatui 0.30.2
`run` uses infallible initialization/restoration, while even `try_init` installs a
process-global panic hook that calls infallible `restore`/`eprintln!`. It therefore
uses the equivalent fallible crossterm raw-mode/alternate-screen primitives and
`Terminal::new`, without replacing the process panic hook. This is deliberate;
worker panic must not trigger an infallible terminal-output path before joining.

The pinned ratatui-core 0.1.2 Terminal destructor also retries cursor-show when its
internal hidden flag remains set and infallibly logs on error. Native's Linux
Crossterm backend therefore owns a guarded writer: after all explicit real
restoration attempts, the shared restoration helper switches only backend output
to a successful no-I/O sink before Terminal drops. Its implicit cursor command
then cannot reach stdout or the infallible stderr error branch. This uses neither
unsafe code nor forgetting/leaking the terminal. It does not hide restoration
failures: the real raw/screen/cursor results above the output gate remain retained.
The gate's one fixed-size Arc/atomic control is bounded operation metadata, separate
from root/text/backend budgets; it introduces no payload or path copy.

On event/render/poll/read failure or normal exit, the owned lifecycle requests
cancellation and drops its sole worker request sender **before** teardown. It then
independently attempts raw-mode disable, alternate-screen leave and cursor show;
all three errors are retained in TeardownFaults rather than short-circuiting or
printing. Partial initialization failure follows the same cancel/close/teardown
path and never enters the event loop. Only afterward does the frontend drain all
replies and join, releasing a worker blocked on its bounded result send.

A known typed result is retained and safely rendered through fallible stderr.
The returned io::Error wraps a public ShutdownFailure retaining initialization,
event, teardown (including each raw/screen/cursor fault), worker/join and output
faults separately alongside the typed result. It never converts known success to
uncertainty. Worker panic without a result is outcome-unavailable, with no retry
or modifying recovery; it is drained/joined. Stdout/stderr delivery can both fail.
Fatal signals, allocator aborts and process crashes remain held. No infallible
terminal diagnostic helper or global restoration panic hook is installed.

## Independent resource boundaries

| Boundary | Limit and charge |
| --- | --- |
| CLI values | parent 65536 bytes, new leaf 4096 bytes, checked on original OS strings before frontend copies; paired configuration is immutable |
| Selection roots | at most 1024 roots;4 MiB including exact root vector slots, each cloned component vector slot and conservatively charged component capacities before allocation |
| Core plan work |16 million candidate comparisons; root-index membership passes separately have a derived bound from state/root counts; no CPU-instruction bound is claimed |
| Preview text |20 MiB of complete escaped ASCII, exact checked byte expansion before reserve/push; viewport clips only rendering |
| Diagnostics |1 MiB of final escaped text; typed outcome retained independently even if rendering/output fails |
| Queues |one request and one reply; one active operation; root ownership moves rather than retaining multiple clones |

Root-mark indices remain bounded at 1024 in the UI/command, separate from root byte
storage. Snapshot/catalog/backend limits, renderer/allocator/kernel/argument storage
and fixed scalar formatting temporaries remain independent trusted infrastructure.
The frontend caps do not imply a total heap proof. Synchronous plan construction
and kernel calls can block; no cancellation latency or fabricated progress estimate
is promised. The executable performs no automatic receipt-file write or upload.

## Evidence and pending assurance

Fifteen native library tests cover raw CLI bounds/pairs/flag-like values, acknowledgment
reset and multi-iteration event drain, escaping limits, all typed outcomes,
fallback-output failure retaining authority, stale IDs, worker panic and full reply
queue close/drain/join. Two CLI binary tests check safe capability help/nonterminal
rejection. Five frontend-worker integration tests reconstruct/restore original
full plus two incremental official dates and compare raw paths, modes, contents
and timestamps against independent source oracles. Source-only link timestamp
changes follow the recorded oracle exclusion; actual emitted link mtimes are
checked exactly. Preview cancellation and command serialization write nothing. Production lifecycle integration injects event, teardown and fallback-output faults during an actual active restore, drains/joins and retains the exact typed result. Partial-init tests check request-channel closure before teardown with a deterministically queued Ready reply; Native teardown aggregation tests prove all three actions are attempted despite errors. A concrete Terminal/CrosstermBackend regression hides the cursor, breaks its writer, invokes the production restoration-and-disarm helper with a failed actual cursor show, and proves dropping the terminal makes zero additional underlying writer calls; its infallible error-log branch therefore cannot be entered even if stderr is unavailable. Synthetic worker-result-loss tests distinguish unavailable outcomes from unpublished results.

The checked-in PTY smoke script exercises real rendering and key sequences with a
non-UTF-8 destination leaf. It observes read-only/preparation/empty-confirmation
no-write states, explicit acknowledgment publication and exact terminal mode
restoration on exit. Its receipt names limitations; it is not exhaustive terminal
fault injection, resize, cancellation-race or live S 3 testing. Pure channel/output
fault tests and the active-restore shutdown-helper integration exercise lifecycle primitives; a complete injectable terminal-loop
fault test and independently checked frontend state-machine/controller proofs remain
pending. Backend fault/race/containment evidence is separately recorded and not
replaced by frontend annotations. Independent software review is pending.
