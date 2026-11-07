# Official Plain Reader Unsupported-Volume-Digest Acceptance

**Status:** Specification and implementation independently reviewed. Focused
verification, full core tests, Duvet trace, formatting, and strict Clippy pass.
Full local `cargo xtask ci` passes. Hosted Linux assurance and Duvet Pages
publication remain pending.

## Goal and scope

Verify through the public plain reader that a required manifest with no
supported digest declaration for a volume produces an explicit
unsupported-integrity error before that volume is opened. The test mutates
only the first-incremental manifest in memory, retaining the official fixture
files and all volume payloads. It complements the separate stored-byte
mismatch acceptance: here the payload is unchanged, but its only declared
digest algorithm is unsupported.

## Fixture mutation

Start from `tests/fixtures/duplicity-3.0.7/plain/` and its exact twelve object
keys. In the stored body for

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
```

find the first `Hash SHA1 ` declaration under `Volume 1:` and replace only the
four algorithm bytes `SHA1` with `SHA2`. Preserve all other bytes, including
the 40 hexadecimal digest digits and the second volume's declaration. The
mutation is same-length. Update only the test source's identity metadata for
the changed manifest (recompute its ETag and use a distinct synthetic version
ID); its stored size remains unchanged. Preserve every other object's key,
body, size, and identity. The mutated manifest input must contain this
syntactically valid unsupported-only digest declaration for volume 1. Manifest
parsing is expected to return `ManifestError::UnsupportedIntegrity` because no
supported digest is available for that volume. The test must assert the
transformed bytes and source identity directly.

This is a test-local policy mutation, not an official producer output or S3
identity claim. The original first-incremental volume 1 bytes stay unchanged.

## Required public observations

1. Call public `Catalog::discover` and require zero source opens. The same
   twelve-key inventory still yields the three official name-derived set
   descriptors; derive the Full anchor through `Catalog::manifests()`.
2. Clear the source-open log and call public `Catalog::read_snapshot` at
   `Timestamp(1700172800)`. Require exactly
   `ReaderError::Manifest(ManifestError::UnsupportedIntegrity)` and no
   snapshot. The first incremental manifest is rejected while parsing its
   declarations, before any of its volumes are opened.
3. Require exactly these five keys to have been opened once each: the Full
   manifest, Full volumes 1/2/3, and the mutated first-incremental manifest.
   No first-incremental volume, final-incremental manifest/volume, or
   unrelated object may be opened. Thus the valid Full set is read before the
   unsupported-integrity error, and the reader returns no partial final
   snapshot.

## Requirements and verification

1. MUST produce an unsupported-integrity diagnostic when a required volume has
   no supported declared digest. The public result MUST be the exact typed
   `ReaderError::Manifest(ManifestError::UnsupportedIntegrity)` above. Link
   the integration test to `PARSE-ENV-006`.
2. MUST preserve the official volume bytes and make the manifest mutation
   same-length, so the observed outcome comes from unsupported digest
   selection rather than source identity change, size mismatch, or payload
   corruption.
3. MUST keep discovery metadata-only and reject the first incremental
   manifest before opening any of its volumes, any later incremental objects,
   or returning a snapshot.

Add a focused integration test in `crates/core/tests/reader.rs` and annotate it
for Duvet `PARSE-ENV-006`. Do not link `PARSE-ENV-005`: this case has no
supported digest declaration to compare against bytes. Do not link malformed
input, official restore, chain-success, S3, or filesystem-publication
requirements.

Acceptance requires the focused test, targeted Duvet query and full trace,
focused/full core tests, formatting, strict Clippy, local `cargo xtask ci`,
independent implementation review, and hosted Linux assurance plus Duvet Pages
publication.

## Exclusions

This case covers only an unknown-only digest declaration for first-incremental
volume 1 in the official plain profile. It does not test unknown digests when a
supported digest is also present, malformed/conflicting declarations, other
volume ordinals, alternate profiles, S3 identity races, filesystem
restoration, or complete reader/controller refinement.

## Checklist

- [x] Independent specification review accepts the exact same-length manifest
      mutation, unsupported-digest preconditions, typed error, and five-key open
      set.
- [x] Focused test proves only the first Volume 1 algorithm token changes to
      `SHA2`; payload bytes and all other fixture objects remain unchanged.
- [x] Discovery opens no source objects; final read fails on the first
      incremental manifest with exact `Manifest(UnsupportedIntegrity)` and no
      snapshot or volume reads from that set.
- [x] The five-key source-open set is exact; no later incremental objects open.
- [x] Duvet `PARSE-ENV-006` query and full trace, focused/full core tests,
      formatting, strict Clippy, and independent implementation review pass.
- [x] Full local `cargo xtask ci` passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
