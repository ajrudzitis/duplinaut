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

## Ordered volume-scan cursor proof

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
accounting. The `reader-volume-scan-cursor` milestone remains unproved. The
candidate `f1b7244` was independently accepted, and exact-source `cargo xtask
ci` passed with source SHA-256
`e32ccd5939de104017c8fdfb7a83f9cf597166868b0d5d725000bb625a776c35`. Verus,
Duvet, Clippy, builds, workspace tests, integrations, and doctests passed. Rust
iterator semantics and the broader reader controller remain open.

## Catalog discovery metadata charge proof

`Catalog::discover` now calls a verified initialization transition after the
source listing is returned and before allocating the sorted order or catalog
arrays. It charges the listed vector's reported capacity multiplied by the
`StoredObject` entry size, plus the fixed `Catalog` shell. Before allocating
those additional arrays, it also charges the per-object key, ETag, and version
string capacities, one `CatalogObject`, one `SetDescriptor`, and the accounted
index slots. Both transitions use checked arithmetic and return `None` on
overflow or when the configured catalog metadata limit would be exceeded.
Failure is mapped to `ReaderError::Limit`; no partially advanced charge is
published by the per-object transition.

Direct tests cover exact-limit acceptance, one-byte-short rejection,
multiplication/addition overflow, and object-charge atomicity. A reader test
exercises rejection before catalog construction. The Duvet links map these
proofs and tests to [PARSE-GEN-005](parsing-requirements.md#parse-gen-005).
This proof was independently accepted after correcting its wording to describe
requested slots rather than physical capacity. It does not complete the reader
assurance milestone. Exact-source `cargo xtask ci` passed with source SHA-256
`4a113cd0adc1257dc99065bfb0d73a4980cca82a24d5d28d70118ad9cd560dfd`. Verus
reported 2,047 vstd, 3,183 Vest, and 1,053 core obligations with zero errors;
Duvet, formatting, strict Clippy, workspace tests, integrations, and doctests
passed. The broader reader resource-accounting and allocator guarantees remain
open.

The proof is limited to the checked charge transitions and their production
call order. It excludes allocation internals of `source.list`, actual allocator
behavior, the provenance or accuracy of reported string capacities, and a
total heap bound. Accordingly, the count is a conservative metadata accounting
model rather than a proof of process-wide memory consumption.

## Authentication evidence comparison charge proof

Before each certificate lookup in `record_evidence`, the reader now calls a
verified transition with the current comparison count, the existing certificate
pool length, and the configured comparison limit. The transition adds the full
pool length as a worst-case upper bound for the following `.position` scan.
The scan may short-circuit after a match, so the reserved work can exceed
comparisons actually performed; charging the full pool avoids undercharging.
Overflow or an over-limit sum returns `None`; `record_evidence`
maps that to `ReaderError::Limit` and does not update its comparison counter
for that rejected scan. Exact-limit, over-limit, and overflow cases have direct
helper tests. A reader unit test covers an exact successful pool comparison
and confirms that a rejected scan leaves the counter and published object list
unchanged.

The evidence is linked to [PARSE-GEN-005](parsing-requirements.md#parse-gen-005)
and [PARSE-ENV-022](parsing-requirements.md#parse-env-022). This candidate
proves only the scalar comparison-budget transition and its placement before
the corresponding certificate equality scan. It does not prove certificate
equality semantics, duplicate detection, or the complete `record_evidence`
operation. Metadata charging, temporary version-vector allocation, and
certificate-version admission remain separate checks; allocator behavior and
total evidence/process heap use are outside scope. The proof does not complete
authentication-evidence accounting. Candidate `ac146e6` was independently
accepted after clarifying that the pool length is a worst-case bound. The
corrected exact-source `cargo xtask ci` passed with SHA-256
`ac0b5bd838b09ed2e08bc05fcbbb8164ef88850ab6195cd5cd6a507ebdd38d9e`. Verus
reported 2,047 vstd, 3,183 Vest, and 1,056 core obligations with zero errors;
Duvet, Clippy, workspace tests, integrations, and doctests passed.

## Multipart metadata admission proof

Before calling `Manifest::volumes()` or allocating the raw-occurrence vector,
`MultipartAssembler::new` now asks a verified checked-arithmetic transition to
reserve metadata from the raw manifest-record count. The formula charges
`max(4, raw_record_count)` logical slots for twice the `VolumeRecord` reference
size (the conservative normalized-view allowance) and one byte per
raw-occurrence slot, then adds the `MultipartAssembler` shell. Overflow or a
charge above `max_metadata_bytes` returns `None`, which the constructor maps
to `AssemblyError::Limit` before it reaches normalization or assembler state
construction.

Tests cover exact-limit acceptance, one-byte-short rejection, arithmetic
overflow, and constructor rejection before ordinal validation; the source
ordering puts the verified admission result before both allocation paths. The
proof and tests are linked to [PARSE-GEN-005](parsing-requirements.md#parse-gen-005).
This proof does not complete the overall multipart assembly milestone. The
candidate `6be51ab` was independently accepted, and exact-source `cargo xtask
ci` passed with source SHA-256
`ecb088151af79575c09b50e21ec0c9171b2dde2b59fba2b488616b1a6a165c74`. Verus
reported 2,047 vstd, 3,183 Vest, and 1,055 core obligations with zero errors;
Duvet, Clippy, workspace tests, integrations, and doctests passed.

The verified claim is the reservation arithmetic and the constructor's
fail-closed use of its result. Manifest normalization semantics remain covered
by their separate proof. This candidate does not prove the allocator's actual
growth behavior or capacity provenance, allocations inside normalization,
whether allocation succeeds, or total process heap usage. The slot count is a
conservative accounting model from the raw input count, not a refinement of
`Vec`'s concrete capacity.

## Authentication evidence record metadata reservation proof

Both `record_plain` and `record_evidence` now call a verified admission
transition before the evidence-object reserve/append. The transition checks
that the current object count is below its configured limit and computes the
bounded sum of current evidence bytes, the supplied source-metadata charge,
and the version-index slot count multiplied by `size_of::<usize>()`. Plain
records pass zero version slots; OpenPGP records pass the decoded certificate
count. Checked overflow, a full object count, or a byte-limit excess returns
`None`, so those record paths fail with `ReaderError::Limit` before their
object/version-vector allocation and object append.

Direct tests cover the exact aggregate byte boundary, one-byte-short rejection,
count-limit rejection, and arithmetic overflow. Internal record-path tests
cover exact and short limits for plain and OpenPGP records; public-reader tests
confirm both envelope paths return a limit error without publishing a
snapshot. The candidate is linked to
[PARSE-GEN-005](parsing-requirements.md#parse-gen-005) and
[PARSE-ENV-022](parsing-requirements.md#parse-env-022).

The proof treats source-metadata bytes, certificate count, and version-index
element size as supplied facts. It does not prove the accuracy/provenance of
string capacities, clone capacity behavior, allocator growth or allocation
success, certificate-metadata charging later in `record_evidence`, policy
summary allocation, or whole-operation atomicity. It covers this reservation
transition and its production use only; overall authentication-evidence
accounting remains incomplete. Candidate `41bf000` was independently accepted,
and exact-source `cargo xtask ci` passed with source SHA-256
`5bf46a2bb0a6fd153917d3f88ad0e7f1d433db8dcb942169573867e9ab4fd5d9`. Verus
reported 2,047 vstd, 3,183 Vest, and 1,058 core obligations with zero errors;
Duvet, Clippy, formatting, workspace tests, integrations, and doctests passed.
