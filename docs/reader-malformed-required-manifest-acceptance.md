# Official Plain Reader Malformed Required-Manifest Acceptance

**Status:** Implementation and independent review pass. Full local
`cargo xtask ci` passes; its receipt records source SHA-256
`6fb3322ba4e415b301432ff4f4c454cc0b24e25fb7bebf9430af98acdbe66d83`;
Verus partitions 2,047/3,183/1,329 have zero errors. Hosted Linux assurance
and Duvet Pages publication remain pending.

## Goal and scope

Verify that a required manifest whose content is malformed produces its typed
parser error through the public reader and cannot be bypassed by falling back
to later sets or volume data. Discovery remains metadata-only because the
object key is still a valid completed Full-manifest name. This is one
test-local body mutation of the official plain Duplicity 3.0.7 fixture; no
fixture files or producer outputs are changed.

## Fixture mutation

Start from `tests/fixtures/duplicity-3.0.7/plain/` with its exact twelve object
keys. In the in-memory test source, replace only the body for:

```text
backup/duplicity-full.20231114T221320Z.manifest
```

with an empty byte vector. Keep its key and the other eleven official object
keys unchanged. Update the test source's local stored-byte size and synthetic
identity fields consistently for the empty body so the source returns the
mutated body as requested rather than reporting an identity change. The
remaining fixture object payloads, sizes, and identities are unchanged. This
is a deliberately malformed in-memory test object, not an official fixture
compatibility result or an S3 version/ETag claim.

The manifest parser rejects empty input as `ManifestError::Malformed` because
it contains no volume records. Discovery MUST NOT open the body and MUST still
classify the object from its valid key as a completed Full manifest.

## Required public observations

1. Call public `Catalog::discover`. Require zero source opens. Require the
   malformed-body object to remain visible with parsed kind `FullManifest`,
   timestamp `1700000000`, and completed status. Its name metadata does not
   validate its body.
2. Require `Catalog::manifests()` to retain exactly the three name-derived
   descriptors, independent of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and final Incremental
   `(1700086400, 1700172800)`.
3. Derive the Full descriptor index from the public iterator, clear the
   source-open log, and call public `Catalog::read_snapshot` at
   `Timestamp(1700172800)`. Require exactly
   `ReaderError::Manifest(ManifestError::Malformed)` and no snapshot.
4. Require the read's exact opened-key set to contain only the malformed Full
   manifest key above. No data volume, incremental manifest, or other object
   may be opened; the reader MUST NOT fall back to another set or produce a
   partial snapshot.

## Requirements

1. MUST return a structured malformed-input error when parsing the selected
   required Full manifest's empty body. The exact public result MUST be
   `ReaderError::Manifest(ManifestError::Malformed)`. Link this behavior to
   `PARSE-GEN-001`.
2. MUST keep discovery metadata-only: valid key classification may expose the
   name-derived Full descriptor without reading or accepting the manifest
   body. The public snapshot operation MUST parse the body before reading any
   required volume and MUST NOT silently fall back or return a snapshot.
3. MUST assert that exactly the required malformed Full manifest was opened;
   no other key may be opened after its malformed body is returned.
4. MUST keep the mutation test-local and make no claim about official
   malformed-manifest producer output, S3 object identity, filesystem
   restoration, or complete reader/controller refinement.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`. Preserve
  the official key set and all unrelated fixture object contents; mutate only
  the in-memory Full-manifest body and update that test source's local size
  and identity metadata consistently.
- Add a Duvet test link to `PARSE-GEN-001` for the exact public structured
  error. Do not link chain-success, snapshot-path, crypto, or S3 requirements:
  this path returns no snapshot before volume data is read.
- Run the focused test, targeted Duvet query/full trace, focused/full core
  tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted Linux
  assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the zero-byte mutation, stable name-derived
  catalog descriptors, exact typed error, absence of fallback, and the
  one-key open set.

## Exclusions

This acceptance covers only an empty body for the official Full manifest key.
It does not test other malformed manifest syntax, truncated volume data,
unsupported digests, partial/unsupported object names, alternate profiles,
S3 identity semantics, filesystem restoration, or formal/complete
reader-controller refinement.

## Checklist

- [x] Independent specification review accepts the body mutation, public
      error, metadata-only discovery, and one-key open set.
- [x] Discovery opens no source objects and retains the three name-derived
      descriptors.
- [x] The final-target read returns exact
      `ReaderError::Manifest(ManifestError::Malformed)`, no snapshot, and
      opens only the selected Full manifest.
- [x] Focused test and `cargo test -p duplicity-core` pass; Duvet
      `PARSE-GEN-001` implementation/test query and full `cargo xtask trace`
      pass; formatting, strict core Clippy, and diff checks pass.
- [x] Independent implementation/test review passes.
- [x] Full local `cargo xtask ci` passes on the final tracked tree after the
      shared parser annotation; the successful receipt and Verus counts are
      recorded above.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
