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

## Bounded-controller proof checkpoint (candidate; review pending)

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
allocator behavior, or complete repository reader. Independent review and the
full acceptance gate remain pending; the README milestone stays unchecked.
