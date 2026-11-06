# Required-volume identity acceptance plan

Status: accepted after independent specification, verifier, and implementation
review; local full CI and hosted Linux assurance/Duvet Pages pass.

## Goal and scope

Formally refine the exact post-parse predicate that decides whether one
catalog object's parsed name is the required data volume for a selected
manifest record. The current predicate is inline in
`Catalog::read_snapshot_inner` in `crates/core/src/reader.rs`, after the
existing checked-object charge and before candidate accumulation.

The predicate MUST require all of the following:

1. `ObjectName::completed()` is true.
2. The object's parsed start and end timestamps exactly equal the selected
   `SetDescriptor` start and end.
3. The parsed volume ordinal is exactly the manifest `VolumeRecord.number`.
4. A full descriptor pairs only with `ObjectKind::FullVolume`; an incremental
   descriptor pairs only with `ObjectKind::IncrementalVolume`.

This acceptance traces only to
[PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002), which requires
missing required volumes to fail closed, and
[PARSE-NAME-005](parsing-requirements.md#parse-name-005), which excludes
partial objects from completed restore sets. These links do not claim complete
reader-controller refinement.

## Implementation boundary

1. Factor the predicate used at `reader.rs`'s required-volume scan into a
   small pure helper in `crates/core/src/verified/reader_volume_scan.rs` or a
   dedicated `reader_volume_identity.rs` module. `read_snapshot_inner` MUST
   call this helper directly for each already-parsed catalog `ObjectName`.
2. The helper's Verus-visible signature MUST use the project's `ObjectName`
   and `SetDescriptor` types, plus the already-parsed manifest volume ordinal.
   It MUST NOT replace project enum values with hand-assigned integer tags.
   If verifier restrictions make a primitive-tag model necessary, the mapping
   from actual project fields to tags is a trust boundary outside this proof;
   the implementation record and claims MUST state that boundary explicitly
   and MUST NOT describe the production predicate as fully refined.
3. The specification MUST state that envelope authentication/decoding is
   downstream. Envelope does not participate in identity matching except
   through the existing `ObjectName::completed()` gate (whose contract also
   excludes PKCS7); Plain, Gzip, and OpenPGP otherwise share the same identity
   rule. The predicate does not prove an envelope is supported for the current
   read, authenticated, or decoded.
4. The helper MUST not parse names, identify source objects, inspect object
   bytes, verify manifest digests, choose chain descriptors, enumerate the
   catalog, or charge scan work. It receives parsed identity and one selected
   manifest volume number only.

## Formal contract and focused matrix

The helper's postcondition MUST be equivalent to an independent model over
the project types:

```text
name.completed()
&& name.start == descriptor.start_value()
&& name.end == descriptor.end_value()
&& name.volume == Some(required_number)
&& match (descriptor.kind_value(), name.kind) {
     (Full, FullVolume) | (Incremental, IncrementalVolume) => true,
     _ => false,
   }
```

The model and implementation MUST be independently written enough to expose
field omissions; tests MUST exercise both project variants directly rather
than use the production helper to compute expected values.

The Verus unit matrix MUST include:

- A positive full-volume identity with a full descriptor, equal start/end,
  and matching ordinal.
- A positive incremental-volume identity with an incremental descriptor,
  strictly ordered interval, and matching ordinal.
- For each positive case, individual negative cases changing only the
  descriptor/object kind pairing, start, end, or ordinal.
- An otherwise exact identity whose object is partial, which MUST not match.
- Equal full and incremental identity tuples with each Plain, Gzip, and
  OpenPGP envelope, demonstrating that envelope choice itself does not affect
  identity once `completed()` is true. This does not test or prove later
  envelope handling. PKCS7 is excluded only by the existing `completed()`
  contract; this matrix MUST NOT claim downstream PKCS7 handling.

The formal contract MUST prove exact Boolean equivalence for all inputs of
these project types, not only the listed test examples. It MUST not assert that
the object name was correctly parsed from a source key; that remains the
separate filename/parser boundary.

## Existing source-level regression

Strengthen the existing
`missing_corrupt_and_ambiguous_objects_fail_explicitly` test in
`crates/core/tests/reader.rs`; do not claim a new source scenario. Its missing
case already removes the required full-set `vol2` while retaining a different
volume candidate. Add an explicit assertion that the `Source::official`
fixture still contains the valid full-set `vol3` key after `vol2` is removed,
then preserve the assertion that reading the selected snapshot returns
exactly `ReaderError::MissingVolume { number: 2 }` and yields no snapshot.
This makes the existing regression show that a present but wrong-ordinal
candidate cannot satisfy the manifest requirement; it is not a new source
scenario and must not be overstated as proof of catalog enumeration or all
wrong-field cases.

## Duvet and verification

- Link the verified helper and its truth-table tests to PARSE-CHAIN-002 and
  PARSE-NAME-005 only. Link the strengthened existing missing-vol2/retained-
  vol3 integration test to PARSE-CHAIN-002 only: it exercises fail-closed
  missing-volume behavior, not partial-object exclusion. The annotations MUST distinguish the helper's
  post-parse identity decision from filename parsing, scanning, and envelope
  validation.
- Run the focused Verus test/obligations and reader integration test, targeted
  Duvet queries, `cargo xtask trace-update`, `cargo xtask trace`, formatting,
  strict Clippy, and the full `cargo xtask ci` gate. Local full CI passed with
  source SHA-256
  `d4d2728…`; hosted Linux assurance and Duvet Pages passed in descendant
  [run 42181068479](https://github.com/ajrudzitis/duplinaut/actions/runs/42181068479).
  The PTY stability fix in `b6cf3c6` is a separate follow-up and is not part of
  this core predicate proof.
- Independent review MUST confirm the actual production scan calls the
  verified helper with unmodified project identity fields, every predicate
  clause is present in model and implementation, envelope policy stays
  downstream, and the vol3-retained regression is the existing missing-vol2
  scenario strengthened with an explicit retained-candidate assertion.

## Exclusions

This is a post-parse required-volume identity predicate only. It does not prove
`parse_name`, manifest parsing, catalog enumeration order/completeness,
candidate uniqueness, source identity/version checks, digest verification,
multipart assembly, envelope/crypto policy, cancellation timing, resource
accounting, snapshot reconstruction, or complete reader-controller
refinement.

## Checklist

- [x] Specify the project-type identity predicate and prove exact model
      equivalence for full and incremental required volumes.
- [x] Exercise both positive cases and individual kind/interval/ordinal and
      partial-status negative cases in Verus/native tests.
- [x] Strengthen the existing missing-vol2 integration case with an explicit
      vol3-retained assertion and exact `MissingVolume { number: 2 }` result.
- [x] Focused/full tests, Duvet trace, formatting, strict Clippy, and local
      full `cargo xtask ci` pass; hosted Linux assurance and Duvet Pages pass
      in run 42181068479.
- [x] Independent specification review accepts the scoped evidence.
- [x] Independent implementation review accepts the production refinement
      and test evidence.
