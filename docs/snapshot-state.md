# Snapshot-state reconstruction checkpoint

Native state reconstruction implements PARSE-STATE-001/002 over completed
`AssembledArchive` inputs. `from_full` accepts snapshot operations only;
`apply_incremental` applies ordered snapshot, diff and deletion records to a
private provisional map. It returns a new read-only state only after every
operation and final ancestor check succeeds. Errors leave the preceding state
unchanged. Content and metadata ownership stay private; getter APIs return
slices and copied fields, never mutable maps or Arc handles.

Directory snapshots replace that directory's metadata and preserve descendants.
Nondirectory snapshots replace their path and remove descendants. Deletions
remove a path and its descendants. Diffs require regular tar records and a
regular file at the same path in the preceding SET; earlier operations in the
current archive cannot supply or mutate that basis. Unmentioned entries persist.
After applying the entire set, every nonroot path must have directory ancestors,
and a present root must be directory. A genuinely empty state is valid without
inventing root metadata; restoration must handle its absence explicitly.

Component-order map ranges remove subtrees without scanning unrelated entries.
A borrowed directory stack checks final ancestors in sorted path order. Its
capacity is bounded and charged before allocation. Special filesystem types
retain browse metadata and are marked unsupported for the later restoration
selection. There are no filesystem effects in this checkpoint.

Limits distinguish logical and live-transition budgets. Logical byte limits
bound every provisional/resulting regular-file content length; entry count,
per-file bytes and metadata budgets are separate. Live payload budgets charge
preceding state buffer capacities, caller-owned assembled payload capacities,
provisional retained capacities and reconstruction buffers. Unchanged shared
Arc allocations can be counted twice, making this conservative. Native delta
Vec growth is allowed only when `max(8, 3*output_bound)` fits remaining live
budget, allowing old/new buffers during reallocation. Snapshot payload copies
use bounded `try_reserve_exact`; returned capacities are checked again.

Metadata charges include input archive entry slots and owned vector capacities,
preceding/provisional state metadata, component buffers, Arc control blocks,
state-entry values, conservative pinned standard-library BTree node layouts,
one copied removal key and the final ancestor stack. Bounds are Rust capacity
budgets, excluding allocator bookkeeping, unrelated caller allocations and other
caller-held snapshots. Standard-library allocation/Arc behavior is trusted;
this is not a global heap or formal allocator proof. Changing limits to values
below the preceding state can conservatively reject a transition, even one
that would subsequently delete enough data to fit.

The caller must establish intended manifest provenance, source identity,
filename classification and full/incremental chain continuity. Successful
assembly/state transitions do not authenticate unsigned manifests or select a
unique chain. The API accepts assembled archives and does not accept an
unchecked-entry or integrity boolean bypass.

Tests cover immutable preceding-SET basis, transactional failure, directory
preservation, type replacement, subtree deletion/recreation, deferred ancestor
checks, empty/root states, unsupported display metadata and independent logical
and live limits. Original duplicity 3.0.7 gzip volumes and manifests reconstruct
all three plain snapshot dates. Source-tree oracles check every byte-valued path,
entry type, ordinary mode, regular-file length/SHA256, directory/file mtime and
symlink target. The recorded official comparison excludes source symlink mtime:
an unchanged link can have a touched source mtime without an emitted data record,
and duplicity's restore deliberately skips symlink utime. Native tests separately
check preservation of actually emitted link metadata. No producer executable is
invoked by the reader.

Independent native review found no semantic blocker. Workspace tests, strict
Clippy and Duvet regression checks passed at the initial native checkpoint. Formal controller
refinement and resource-model proofs remain pending. The underlying delta kernel has its existing exact-output proofs;
those do not constitute a proof of this new state controller.

The first formal resource checkpoint verifies `check_counts` against
`budget_fits`, an independent natural-number acceptance predicate. It returns
exactly `Ok(())` when accumulated logical payload/metadata and conservative live
payload/metadata totals fit `usize` and their respective limits; every other
input returns exactly `StateError::Limit`. The native `check_budget` wrapper
passes the same private counters through an allocation-free scalar projection.
A strict `--no-cheating` state-module check reports four verified obligations and
zero errors for the marked scalar surface. All twelve existing state/restore-plan
integration groups and strict core-library Clippy pass on that source.

The second scalar checkpoint verifies `allowed_file_bytes` against
`allowance_model`. Its exact result accounts for checked retained live payload,
remaining logical payload and the minimum of per-file, logical and reconstruction
allocation bounds. Successful snapshot allowances fit remaining live payload.
For a nonzero delta allowance, both eight bytes and three times the allowance
fit the remaining live budget, establishing the conservative
`max(8, 3*allowance)` arithmetic ceiling. A zero delta allowance needs no minimum
eight-byte guarantee. Native basis selection, snapshot reservation, and delta
invocation remain unchanged. The marked state surface now has five strict
verified obligations with zero errors; all twelve existing state/restore-plan
integration groups and strict core-library Clippy pass on the checked source.

This checkpoint establishes neither that map counters equal retained allocations
nor that accounting charges bound allocator behavior. Preceding-set basis,
ordered map transitions, subtree removal, final ancestors, error atomicity and
complete controller/resource refinement remain pending. Native integration
tests continue to cover those behaviors. The pure budget predicate contributes
to PARSE-GEN-003/005; it does not complete either whole-reader requirement or
PARSE-STATE-001/002.

The componentwise removal-charge transition is independently accepted at
candidate `f9fad46` and passed exact-source full `cargo xtask ci` with source
SHA-256 `688b27d37ca9e99310f1e517675cc906402442761f3a314593661271ddedc508`.
Verus proves that the helper returns the exact three decremented counters only
when payload, metadata and capacity can all be subtracted, and otherwise returns
`None`. Production code computes all three results before assigning any counter.
The unit test covers success and each individual underflow; the state integration
test confirms subtree removal drops payload charge from 18 to 13 while the
preceding state retains 18. Duvet matched 138 requirements and 515 references;
Verus/Vest reported 2,047 vstd, 3,183 Vest and 1,017 core obligations with zero
errors; formatting, strict Clippy, workspace tests and doctests passed. This
checkpoint does not prove that counters equal retained allocations, BTreeMap
subtree selection, ordered state transitions or global resource accounting.

Infrastructure references used for this capacity/ownership review:
[Arc ownership API](https://doc.rust-lang.org/std/sync/struct.Arc.html),
[pinned Rust 1.98 byte-vector growth](https://raw.githubusercontent.com/rust-lang/rust/1.98.0/library/alloc/src/raw_vec/mod.rs),
and [pinned BTree node layout](https://raw.githubusercontent.com/rust-lang/rust/1.98.0/library/alloc/src/collections/btree/node.rs).
A zero-output delta needs no content buffer; the minimum eight-byte growth
allowance applies to nonempty output.

The complete local `cargo xtask ci` gate passed after this scalar proof was adopted at commit `474ffd4`, source SHA-256 `4b7b0f11d3fda0f7494c3a269bd57963ccfc07e399e19b5817918dd4c07ac4a0`. Fresh full-workspace verification checked 998 core obligations with zero errors; all existing generation, traceability, lint and test gates passed. The accepted milestone remains tooling-only, and full state/controller proofs remain pending.

The component-prefix predicate has a separate scoped accepted verification in
`BackupPath::is_component_prefix_of`. Its model equates a prefix with a shorter
or equal component sequence whose corresponding elements are all equal. The
production subtree-removal loop uses this helper instead of `Vec::starts_with`,
and a Duvet-linked test checks root, equality, descendants and a near-prefix
sibling such as `a` versus `a-`. This proves path-prefix classification only;
it does not prove BTreeMap range ordering, iteration, removal, or complete state
transitions. Candidate `e5b602b` passed exact-source full `cargo xtask ci` with
source SHA-256 `3253d014838986e755bac17497a742f6cc99c393d81f5348ce80a430144c1ea0`.
Duvet matched 138 requirements and 517 references; Verus/Vest reported 2,047
vstd, 3,183 Vest and 1,019 core obligations with zero errors; formatting, strict
Clippy, all workspace tests and doctests passed. Complete state/controller
refinement remains pending.
