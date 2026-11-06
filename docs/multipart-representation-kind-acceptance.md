# Multipart representation-kind guard acceptance plan

Status: local implementation, proof, tests, Duvet, full CI, and independent
review are complete; hosted Linux assurance and Duvet Pages are pending.

## Goal and scope

Prove and test the small representation-kind admission decision used by
`MultipartAssembler::accept_entry` in `crates/core/src/multipart.rs`. The
decision distinguishes prior ordinary versus multipart representation for the
current path from the incoming entry's representation. It preserves repeated
ordinary operations and rejects mixing representations or restarting a
multipart logical entry within the supported backup-set profile.

The helper covers only the representation-kind marker decision at the two
production branch points: starting a new multipart entry when there is no
pending entry, and admitting an ordinary entry after the existing pending-entry
interleaving check. It MUST NOT replace the pending multipart continuation
path's operation/path/part-number validation.

## Requirement and model

Trace the guard and its exact production tests to
[PARSE-MULTI-006](parsing-requirements.md#parse-multi-006):

> Assembly MUST reject an ordinary/multipart representation mixture or a
> second multipart logical entry for one path within the initial supported
> backup-set profile.

PARSE-MULTI-005 separately requires repeated ordinary operations to retain
archive order. This plan's helper table includes the ordinary-to-ordinary
allowed result only to define the exact representation gate; its proof MUST
not claim archive ordering, which remains covered by the separate MULTI-005
behavior/tests.

Model the production Boolean marker directly: `previous == None` means no
representation has been recorded for the current path; `Some(false)` means
ordinary; `Some(true)` means multipart. The incoming Boolean
`incoming_multipart` is false for an ordinary entry and true for a multipart
part. The independent six-case admission model is:

| Prior marker | `incoming_multipart` | Admitted |
| --- | --- | --- |
| None | false | true |
| None | true | true |
| `Some(false)` | false | true |
| `Some(false)` | true | false |
| `Some(true)` | false | false |
| `Some(true)` | true | false |

The helper's Boolean result MUST exactly match this table. A multipart marker
rejects both a repeated multipart start and an ordinary replacement; an
ordinary marker rejects a multipart start but allows another ordinary
operation. A mixed representation MUST reject in either direction. The
project `ObjectKind`/`Operation` parsing that supplies `incoming_multipart` is
outside this two-Boolean proof and remains a call-site review obligation.

## Acceptance requirements

1. Add a small pure Verus-visible helper, such as
   `verified::multipart_representation::admit_representation(previous,
   incoming_multipart)`, over `Option<bool>` and `bool`. Its postcondition MUST
   prove exact equivalence with the six-row model, with tests exercising each
   row independently. The model MUST be independently written rather than
   calling the production helper to compute expected results.
2. `MultipartAssembler::accept_entry` MUST call the helper at exactly these
   representation decisions:
   - When no entry is pending and a multipart part-1 start would otherwise be
     admitted, using the current path's prior representation marker.
   - When an ordinary entry passes the existing pending-entry interleaving
     check, using the current path's prior representation marker.
   The existing callsite maps a rejected decision to exactly
   `AssemblyError::DuplicateEntry`. The helper MUST NOT be used to decide
   pending continuation path/operation equality, part numbering, entry
   completion, or map insertion.
3. Preserve production-level coverage in
   `crates/core/tests/multipart.rs::malformed_parts_lengths_markers_and_interleaving_rejected`.
   Its Duvet annotation for the representation cases MUST use the exact
   PARSE-MULTI-006 normative sentence above. Assert exact
   `AssemblyError::DuplicateEntry` for ordinary-then-multipart, multipart-
   then-ordinary, and a second multipart logical entry after the earlier one
   has completed. Retain the repeated-ordinary success assertion as its
   separate PARSE-MULTI-005 coverage and strengthen it to assert exactly two
   entries in archive order: entry 0 has payload `first`, entry 1 has payload
   `second`, and both have the expected ordinary `Snapshot` operation.
   Positive first-use ordinary and multipart cases MUST remain covered by
   production tests.
4. Existing malformed length, interleaving, part-order, boundary, official
   multipart oracle, and sticky-failure cases MUST continue to pass unchanged.

## Duvet and verification evidence

- Link the helper contract and six-case Boolean truth-table test to
  PARSE-MULTI-006.
- Keep production tests linked to PARSE-MULTI-006 only for the exact mixed-
  representation/repeated-multipart rejection assertions. Keep the allowed
  repeated ordinary operation assertion linked to PARSE-MULTI-005; do not
  attribute its ordering behavior to the representation helper.
- Add the helper and relevant test evidence to the `multipart-assembly`
  mapping in `docs/assurance.toml`. This does not accept the full multipart
  controller milestone.
- Focused Verus obligations and multipart tests, targeted Duvet queries,
  `cargo xtask trace-update`, `cargo xtask trace`, formatting, strict Clippy,
  and full local `cargo xtask ci` passed at commit `8683482`, source SHA-256
  `82eeac51e83c99f78b9fecfc918353ff7535eb3ae3c2d2947250e03e6503e18f`.
  Independent specification and implementation reviews are complete. Hosted
  Linux assurance and Duvet Pages remain pending; do not mark overall
  acceptance complete until both hosted gates pass.
- Independent verifier review MUST check each truth-table case and the
  Boolean marker semantics. Independent implementation review MUST confirm that the
  production helper is called at only the two stated branch decisions and
  that the exact error mapping and unaffected continuation path remain intact.

## Exclusions

This slice does not prove `started_paths` map-key identity, path equality or
provenance, map insertion/lookup semantics, operation or path matching for a
pending continuation, part sequencing or payload-length policy, entry
completion, archive-order preservation, the complete multipart state machine,
manifest/container boundaries, stored-object integrity, OpenPGP/crypto,
resource accounting, snapshot reconstruction, or filesystem publication. It
does not establish a universal producer rule beyond the conservative initial
supported backup-set profile stated by PARSE-MULTI-006.

## Checklist

- [x] Specify and prove the six-case Boolean marker/kind decision.
- [x] Exercise all six helper rows and production mixed/repeated multipart
      rejection cases with exact error assertions.
- [x] Preserve first-use and repeated-ordinary production behavior and all
      existing multipart regressions.
- [x] Local Duvet mapping/queries and trace, formatting, strict Clippy,
      focused/full tests, and full `cargo xtask ci` pass at commit `8683482`
      with source SHA-256
      `82eeac51e83c99f78b9fecfc918353ff7535eb3ae3c2d2947250e03e6503e18f`.
- [ ] Hosted Linux assurance and Duvet Pages pass.
- [x] Independent specification and implementation reviews accept the local
      evidence.
