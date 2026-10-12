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

Independent native review found no semantic blocker, and all 100 workspace tests,
strict clippy and Duvet regression checks pass at this checkpoint. Formal controller
refinement and resource-model proofs remain pending. The underlying delta kernel has its existing exact-output proofs;
those do not constitute a proof of this new state controller.

Infrastructure references used for this capacity/ownership review:
[Arc ownership API](https://doc.rust-lang.org/std/sync/struct.Arc.html),
[pinned Rust 1.98 byte-vector growth](https://raw.githubusercontent.com/rust-lang/rust/1.98.0/library/alloc/src/raw_vec/mod.rs),
and [pinned BTree node layout](https://raw.githubusercontent.com/rust-lang/rust/1.98.0/library/alloc/src/collections/btree/node.rs).
A zero-output delta needs no content buffer; the minimum eight-byte growth
allowance applies to nonempty output.
