# Read-only restore selection checkpoint

`RestorePlan` borrows a completed immutable `SnapshotState`. Selecting roots includes their exact component-valued paths and descendants, together with source directory ancestors required to contain them. Overlapping selections produce one entry in component order. The plan retains original byte paths and metadata references; no display decoding or owned content copies occur.

Every requested root must exist. Empty explicit selections are errors; `all` can select a genuinely empty state. Included unsupported filesystem types return an error identifying the original path. Unselected special types do not prevent restoring supported entries. Source-state consistency already establishes directory ancestors.

Independent limits cover root count, input state count, output entries, owned reference-vector metadata, and candidate checks. Output capacity is precharged and reserved before pushing, bounded by the smaller of input entries and permitted output entries. Borrowed state storage and allocator bookkeeping are excluded; pinned standard-library allocation/iteration remain trusted infrastructure. These are capacity/check-count budgets, not a formal allocator or wall-clock guarantee.

Independent native review and three regression groups cover ancestor closure, overlap deduplication, borrowed identity, missing roots, selected/unselected special types, exact limits, non-UTF-8 names and near-prefix siblings. Selection/controller refinement remains pending.

This checkpoint performs no filesystem operations. Destination containment, private staging, no-replace publication, permissions/timestamps, cancellation cleanup and filesystem race tests remain required before claiming restoration. A successful selection does not establish chain provenance, source identity, manifest authenticity or decryptability.
