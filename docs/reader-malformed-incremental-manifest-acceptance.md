# Official Plain Reader Malformed Incremental-Manifest Acceptance

**Status:** Implementation, focused/full core verification, and independent
implementation review pass. Full local CI and hosted assurance remain pending.

## Goal and scope

Verify that a required incremental manifest with malformed content produces its
typed parser error through the public reader even after the selected Full set
has been read successfully. The reader must not continue to a later set or
return the already-built Full snapshot as if it satisfied the requested final
date. This is one in-memory mutation of the official plain Duplicity 3.0.7
fixture; checked-in fixture files and producer outputs remain unchanged.

## Fixture mutation

Start with `tests/fixtures/duplicity-3.0.7/plain/` and its exact twelve object
keys. Replace only the stored body for

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
```

with an empty byte vector. Preserve its key and update the test source's local
stored-byte size and synthetic identity consistently, so a read returns the
empty body rather than reporting an identity change. Every other key, payload,
size, and identity must remain byte-for-byte/equality unchanged. This synthetic
mutation is not an official malformed-manifest producer result or an S3
identity claim. The existing manifest parser treats an empty record stream as
`ManifestError::Malformed`.

## Required public observations

1. Call public `Catalog::discover`; require zero source opens. The mutated
   object remains visible as a completed `IncrementalManifest` for interval
   `(1700000000, 1700086400)`, because discovery classifies its valid key and
   does not parse its body.
2. Require `Catalog::manifests()` to retain exactly the three name-derived
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`. Derive the Full index from this iterator.
3. Clear the source-open log and call public `Catalog::read_snapshot` with that
   Full index and target `Timestamp(1700172800)`. Require exactly
   `ReaderError::Manifest(ManifestError::Malformed)` and no snapshot.
4. Require the exact opened-key set to contain only the valid Full manifest,
   its three required data volumes, and the mutated first-incremental manifest.
   Each of those five objects must be opened once. No first-incremental volume,
   final-incremental manifest, final-incremental volume, or unrelated object
   may be opened. The Full data is necessary setup for the selected history;
   its completed state must not be returned as a successful final snapshot.

## Requirements and verification

1. MUST return the structured parser error for malformed required input. The
   exact public result is `ReaderError::Manifest(ManifestError::Malformed)`;
   link this behavior to `PARSE-GEN-001`.
2. MUST keep discovery metadata-only and retain the name-derived descriptors
   even though the incremental body is malformed.
3. MUST fail the requested final-date read at the malformed first incremental
   manifest, without returning a Full-only snapshot or proceeding to later
   required objects.
4. MUST keep the test mutation local and preserve all unrelated official
   object contents and identities.

Add a focused integration test in `crates/core/tests/reader.rs` and a Duvet
test link to `PARSE-GEN-001` for the typed malformed-input result. Do not add
chain-success, path, crypto, or S3 requirement links: no final snapshot is
produced and no volume from the incremental sets is read.

Acceptance requires the focused test, targeted Duvet query and full trace,
focused/full core tests, formatting, strict Clippy, local `cargo xtask ci`,
independent implementation review, and hosted Linux assurance plus Duvet Pages
publication.

## Exclusions

This case covers only an empty body for the named first-incremental manifest.
It does not establish behavior for other malformed syntax, truncated volume
streams, unsupported envelopes, partial names, alternative profiles, S3
identity semantics, filesystem restoration, or complete reader/controller
refinement.

## Checklist

- [x] Independent specification review accepts the mutation, public typed
      error, descriptor observations, and exact five-object open set.
- [x] Focused integration test preserves the twelve-key inventory and changes
      only the in-memory body, size, and synthetic identity for this manifest.
- [x] Discovery opens no objects and exposes the name-derived incremental
      descriptor; the final-target read returns the exact typed error and no
      snapshot.
- [x] The failed read opens exactly the Full manifest, three Full volumes, and
      first-incremental manifest, with no later object reads.
- [x] The `PARSE-GEN-001` implementation/test query and full Duvet trace,
      focused/full core tests, formatting, strict Clippy, and diff checks pass.
- [x] Independent implementation/test review passes.
- [ ] Full local `cargo xtask ci` passes on the final tracked tree.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
