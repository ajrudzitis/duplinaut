# Native reader orchestration checkpoint

`Catalog::discover` binds an immutable catalog to one `ObjectSource`. Listing
retains original keys, exact sizes and version/ETag tokens without cloning or
discarding identities. Discovery opens no objects: classified manifests and
diagnostics remain metadata-only evidence. Unknown/malformed names, partial
artifacts and unsupported envelopes remain visible and never substitute for
completed manifest/volume evidence. Completed OpenPGP names can appear in the
metadata catalog, while reads explicitly reject unsupported crypto.

The initial scope is one flat recognized backup key-parent collection under the
requested source prefix. Recognized artifacts from multiple parents report a
collection ambiguity; no nested-directory chain is guessed. Duplicate listed
keys and duplicate completed manifest/volume semantic identities are rejected,
including aliases across plain/gzip/OpenPGP envelopes. This conservative policy
does not choose between possible backup objects implicitly. Original descriptor
indices identify manifests in this catalog only.

`read_snapshot` takes an explicit full-manifest index and exact selected date.
The metadata-only `ChainPlan` supplies contiguous uniquely selected sets. Every
stored manifest is read through its original `StoredObject`, source conditional
checks and an additional exact-size/EOF/cancellation `CheckedReader`. Plain/gzip
metadata decoding and manifest parsing use independent bounds. Every normalized
required volume is matched by completed object kind, exact set interval and
ordinal, then read through the same identity-bound path. All normalized volumes
are scanned, including restart-rewritten sets; raw interval hints never skip
integrity evidence. Whole-object digest/envelope/tar validation, complete
multipart assembly and state reconstruction must all succeed before a private
`ValidatedSnapshot` is returned. Errors drop provisional results.

The returned state supports browsing and bounded in-memory restoration planning.
No filesystem paths are extracted or staging/publication operations performed.
It is validated under the implemented native plain/gzip checks, not formally
verified as a complete reader, authenticated to a signer or certified globally
restorable. File/directory modes and timestamps, regular contents and symlink
targets/types/modes match original official three-date fixtures and their source
oracles. Symlink mtime comparison follows the recorded official-producer
limitation: unchanged links can retain prior archive times despite source-only
mtime changes, and the official restore/source oracle excludes their mtimes.

Resource limits independently bound source listing, retained catalog capacities,
stored object buffers, decoded manifest bytes, parsed manifest metadata, extra
normalization-view metadata, object matching work, assembly and snapshot-state
transitions. Catalog and standard sorting/allocation are trusted infrastructure.
`ReaderLimits.envelope` controls manifest metadata decoding; volume envelope
limits are independently configured through `ReaderLimits.assembly.envelope`.
Cancellation is checked during discovery and per-set/per-volume/object matching,
and before returning a snapshot. Hashing, sorting, parsing and decompression
are bounded synchronous work with no internal cancellation latency guarantee.
The ObjectSource contract bounds allocations made before listing returns.
The state controller's live-transition budget excludes retained catalog,
manifest/network buffers and backend allocations. These individual bounds are
not a global heap-usage proof. Backend timeouts/identity enforcement, credentials,
manifest authenticity and intended backup collection remain trusted external
contracts. Crypto, signature-only inventory, TUI integration and filesystem
publication remain separate work.

Tests use original stored local official objects through a fake conditional-read
source and reconstruct full plus two incrementals. They also test corruption,
missing/ambiguous/duplicate objects, nested collections, changed identities,
cancellation, short/excess bodies, diagnostics, unsupported crypto and limits.
No external backup application or external write occurs at reader runtime.

## Bounded-controller proof checkpoint

`verified::reader_accounting::charge_object_check` is the scalar transition used
by the required-volume matcher before it inspects each catalog object. Verus
proves that a successful transition increases the charged count by exactly one
without exceeding the configured maximum, and that exhaustion returns no next
count. The repository loop propagates exhaustion as `ReaderError::Limit`; the
existing integration tests exercise the exact object-check boundary.

This checkpoint is linked to [PARSE-GEN-005](parsing-requirements.md#parse-gen-005) and
[PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002). It proves one
per-object budget transition only. It does not prove the enclosing nested scan,
volume identity predicate, cancellation timing, global resource accounting,
allocator behavior, or complete repository reader. Independent review accepted
candidate `dd0ca3e`, and the exact-source full `cargo xtask ci` gate passed with
source SHA-256 `5c9428d96d7d8d0fa5548a9cbda82e8d79b4d7d27c849acd85a13cb532631087`.
The README milestone stays unchecked because the surrounding controller and
complete resource-accounting obligations remain open.

## Volume candidate scan proof

The required-volume matcher now calls `charge_volume_object` before reading the
current object's name to evaluate the native identity predicate. That
transition carries the checked-object count and the current catalog index into
`record_volume_candidate`. The second transition preserves no-match state,
records one matching index, and fails explicitly if another object also
matches. Existing catalog-boundary and duplicate-artifact behavior remains
covered by integration tests; a direct transition test covers the exact limit
and duplicate cases.

These contracts prove the budget-before-predicate call and one-step candidate
accumulation only. They do not prove that enumeration visits each object once
or in order, that the native object-identity predicate is correct, cancellation
timing, the entire nested matcher, or global resource accounting. The
reader-volume-scan assurance milestone remains `proved=false`; complete reader
orchestration remains unproved. Independent review accepted candidate
`414bc04`, and exact-source `cargo xtask ci` passed with source SHA-256
`ba779ed291613a2fbf65f9f8a240221f45ad2e239fe72813ce40fa9d94d3b9e8`. Verus
reported 2,047 vstd, 3,183 Vest, and 1,043 core obligations with zero errors;
Duvet, Clippy, builds, workspace tests, integration tests, and doctests passed.

## Cumulative volume-scan work reservation proof

After manifest normalization and its metadata bound, `reserve_volume_scan`
checks `current_checks + normalized_volume_count * catalog_object_count` using
checked multiplication and addition. The reader does this before entering any
per-object identity scan, then passes the reservation's final count as the
limit to each existing per-object charge transition. After the volume scans,
the observed counter must equal the reservation end. Thus an insufficient
budget rejects the entire manifest's matching work before the first volume
candidate predicate runs; exact and overflow edges have direct tests, and the
official reader budget test covers exact and one-short limits.

The proof covers reservation arithmetic and the call-site ceiling only. It does
not prove that Rust iteration covers each normalized volume and catalog object
exactly once, that matching predicates are correct, cancellation timing, other
resource dimensions, or total reader completeness. The separate
`reader-volume-work-reservation` assurance milestone remains unproved. The
candidate `2eace33` was independently accepted, and exact-source `cargo xtask
ci` passed with source SHA-256
`64cd83ec25e34ac19737117a13e8cfd9f2b3047590dd900aa1c949019725b9a4`. Verus,
Duvet, Clippy, builds, workspace tests, integrations, and doctests passed. The
broader reader controller and complete resource-accounting obligations remain
open.

## Ordered volume-scan cursor proof candidate

`start_volume_scan_cursor` binds the reservation's planned position count to the
checked product of normalized-volume and catalog-object counts. At the start of
each production nested-loop body, `advance_volume_scan_cursor` checks the
enumerated pair `(volume_index, object_index)` against the next flattened
position and advances it once. Before `assembler.finish()`, the reader requires
both cursor completion and equality between consumed checks and the reserved
end. Direct tests cover a complete lexicographic scan, an incomplete scan, and
skipped, reordered, repeated, or extra positions; the official reader test
covers preserved successful scan behavior and the exact/short work budget.

The proof covers the cursor transition and the production gates only. It does
not prove Rust iterator semantics, that the native identity predicate is
correct, cancellation timing, the entire reader controller, or global resource
accounting. The `reader-volume-scan-cursor` milestone remains unproved and
pending independent review.
