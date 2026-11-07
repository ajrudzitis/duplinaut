# Official Plain Reader Incremental-Volume Digest-Failure Acceptance

**Status:** Specification accepted after independent review. No implementation
or verification evidence is claimed.

## Goal and scope

Verify through the public plain reader that a selected incremental volume is
checked against the digest declared by its manifest before the reader can
return a snapshot. The test starts from the official Duplicity 3.0.7 plain
fixture and changes one stored byte in memory. The checked-in fixture and
producer output remain unchanged.

## Fixture mutation

Start from `tests/fixtures/duplicity-3.0.7/plain/` with its exact twelve object
keys. Mutate only the first-incremental volume-1 body:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
```

Flip one byte in its stored payload while preserving its length. Before
discovery, update only this test source object's local identity metadata to
match the mutated body (including a recomputed ETag and distinct synthetic
version ID). Keep the object key, size, every other key/body/size/identity, and
the official first-incremental manifest unchanged. The manifest must continue
to declare the original stored-byte digest, so the source identity check
succeeds and the manifest digest check detects the mutation. This synthetic
mutation is not an official producer result or an S3 versioning claim.

## Required public observations

1. Call public `Catalog::discover` and require zero source opens. Derive the
   Full anchor from `Catalog::manifests()`; retain the exact three
   name-derived descriptors for the official chain.
2. Clear the source-open log and call public `Catalog::read_snapshot` using the
   Full anchor at `Timestamp(1700172800)`. Require exactly
   `ReaderError::Assembly(AssemblyError::Envelope(EnvelopeError::DigestMismatch))`
   and no snapshot. The check is for the stored bytes, before gzip decoding.
3. Require exactly these six source keys to have been opened, once each:
   - the Full manifest;
   - Full data volumes 1, 2, and 3;
   - the first-incremental manifest; and
   - the mutated first-incremental volume 1.

   No first-incremental volume 2, final-incremental manifest or volume, or
   unrelated object may be opened. The successful Full read establishes that
   the test reaches the selected incremental data, while the digest failure
   prevents the final snapshot from being returned.

## Requirements and verification

1. MUST validate the selected volume's manifest-declared digest against the
   original stored object bytes before decoding or using that volume. Link the
   public-reader integration test to `PARSE-ENV-005`.
2. MUST return the typed digest-mismatch error above and no snapshot when the
   stored volume bytes do not match the unchanged manifest declaration.
3. MUST preserve all official fixture objects except for the single test-local
   body mutation and its local source identity metadata update.

Add a focused integration test in `crates/core/tests/reader.rs` with a Duvet
test annotation for `PARSE-ENV-005`. Do not link the official-source restore
oracle, chain-success, S3, or filesystem-restoration requirements: this is a
negative in-memory integrity mutation and returns no snapshot.

Acceptance requires the focused test, targeted Duvet query and full trace,
focused/full core tests, formatting, strict Clippy, local `cargo xtask ci`,
independent implementation review, and hosted Linux assurance plus Duvet Pages
publication.

## Exclusions

This case covers only a same-length stored-byte mutation of the first
incremental volume 1 on the official plain profile. It does not establish
behavior for all digest algorithms, malformed digest declarations, corrupt
gzip streams whose digest is recomputed, encrypted profiles, alternate
producer versions, source identity races, S3 operations, filesystem
restoration, or complete reader/controller refinement.

## Checklist

- [x] Independent specification review accepts the mutation, identity setup,
      typed digest error, and exact six-key open set.
- [ ] Focused test preserves the twelve-key inventory and changes only the
      selected volume body plus test-local identity metadata.
- [ ] Discovery performs no source opens; final read processes the Full set,
      then fails on first-incremental volume 1 with no snapshot.
- [ ] The six-key opened set is exact, with no later incremental objects.
- [ ] Duvet `PARSE-ENV-005`, focused/full core tests, formatting, strict Clippy,
      and local `cargo xtask ci` pass.
- [ ] Independent implementation review and hosted Linux assurance plus Duvet
      Pages publication pass.
