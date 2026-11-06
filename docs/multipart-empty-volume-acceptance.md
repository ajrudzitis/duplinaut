# Empty-volume multipart state acceptance plan

Status: implementation, formal proof, local tests, and independent review are
complete; hosted Linux assurance and Duvet Pages are pending.

## Goal and implementation prerequisite

Prove and test the narrow multipart transition for an accepted empty data
volume. The production path is `MultipartAssembler::push_decoded_checked` in
`crates/core/src/multipart.rs`. It validates empty-volume manifest boundaries,
preserves any in-progress logical multipart entry, advances the remembered
volume boundary, and emits no logical entry or payload.

The implementation prerequisites are met: descendant run
[42181068479](https://github.com/ajrudzitis/duplinaut/actions/runs/42181068479)
passed hosted Linux assurance and Duvet Pages with both required-volume
identity matching and the selected-chain-set adapter in its source history.
Earlier identity-only run 42178899558 failed on a PTY transient-state race;
that separate PTY follow-up was fixed in `b6cf3c6` and does not change or extend
the core prerequisite scopes. Multipart-empty-volume implementation remains
pending its own work and review.

## Requirements and transition model

Trace the empty-volume state transition to
[PARSE-MULTI-004](parsing-requirements.md#parse-multi-004). Link the explicit
no-false-completion test to
[PARSE-MULTI-003](parsing-requirements.md#parse-multi-003), which requires the
reader not to report a multipart file complete until the final-part and
container/boundary conditions hold. Do not use this case to claim complete
multipart-controller refinement.

Use the project `Boundary` and `BackupPath` values. Define `root_none` as the
boundary whose path has zero components and whose block is absent. For an
already parsed volume with no logical entries, let `previous` be the prior
accepted volume's ending boundary (if any), and `pending` be either absent or
the pending logical entry's path and next part number. Its boundary admission
MUST be equivalent to:

```text
start == root_none
&& end == previous.unwrap_or(root_none)
&& match pending {
     Some((path, next_part)) => next_part.checked_sub(1)
         .is_some_and(|last_part| end == Boundary(path, Some(last_part))),
     None => end.block == None,
   }
```

For `pending == Some((path, next_part))`, `next_part` MUST be positive. A
zero value MUST reject: failed `checked_sub(1)` is not an absent pending
boundary and MUST NOT be treated as `end.block == None`.

On acceptance, the logical assembly state MUST preserve `pending`, completed
entries, and accumulated payload byte count exactly; no part is appended or
completed. The remembered ending boundary becomes `end`. Rejection MUST leave
the externally observable assembly unpublished and must follow the existing
sticky-failure behavior of `push_stored_volume`.

This model starts after the object has passed the existing source identity,
stored digest, envelope decode, complete tar parse, and expected volume-order
checks. It MUST use project boundary/path values or an explicitly documented
refinement to them; a primitive-tag-only model is not a proof of production
boundary equality.

## Acceptance requirements

1. Factor the empty-volume boundary admission into a small pure verified
   transition or predicate used by `push_decoded_checked`. The formal contract
   MUST prove exact Boolean equivalence with the independent model above,
   including initial `previous == None`, pending and non-pending cases, and
   checked subtraction for `next_part`. The pending `next_part == 0` case MUST
   reject explicitly; a failed subtraction MUST NOT collapse into the
   no-pending case.
2. The production empty-volume branch MUST call the verified transition before
   updating `previous_boundary`. Review MUST confirm that it does not call
   `accept_entry`, append payload, clear or complete `pending`, or publish
   entries when the parsed volume has no logical entries.
3. Strengthen or extend
   `crates/core/tests/multipart.rs::continuation_markers_zero_final_and_empty_volumes`
   with these observable cases:
   - A valid initial empty volume uses root/None for both start and end.
   - A valid empty volume between a full-size nonfinal part and its zero-byte
     final part preserves continuation; after the final part, exactly one
     assembled entry contains the expected concatenated bytes and operation.
   - With the same nonfinal part followed only by a valid empty volume, final
     `finish()` returns exactly `AssemblyError::Incomplete`; the empty volume
     does not satisfy the missing final short part.
   - One-field-at-a-time malformed cases reject a non-root start, an end that
     differs from the previous boundary, an end path/block inconsistent with
     the pending entry, and a non-absent ending block when no part is pending.
   - The pure model/helper rejects `pending == Some((path, 0))`, even when
     `end.block` is absent.
4. Existing official multipart snapshot/delta oracle tests, malformed-boundary
   tests, poisoning tests, and queued-restart tests MUST continue to pass.
   Their behavior and scope MUST remain unchanged.

## Duvet and verification evidence

- Add the verified admission helper and its truth-table tests to the
  `multipart-assembly` evidence mapping in `docs/assurance.toml`, including
  `PARSE-MULTI-004`; retain the existing PARSE-MULTI-001/-002/-003/-005/-006
  mappings only for tests that exercise those clauses.
- Link the positive preservation and boundary-shape tests to PARSE-MULTI-004.
  Link the nonfinal-part-plus-empty-volume `Incomplete` assertion to
  PARSE-MULTI-003. The test annotation MUST describe only that negative
  completion condition, not full container or reader completion proof.
- Run focused Verus obligations and focused multipart tests, targeted Duvet
  queries, `cargo xtask trace-update`, `cargo xtask trace`, formatting, strict
  Clippy, and full `cargo xtask ci`. Local implementation, focused/full tests,
  Verus, Duvet, formatting, strict Clippy, and independent verifier/implementation
  reviews passed at commit `a3ac918`, source SHA-256
  `767914182c1dbb51001ec7bb3f655b8fa02f9c729a06facd867fbdbeb0982e3e`.
  Hosted Linux assurance and Duvet Pages remain pending in
  [run 42187054602](https://github.com/ajrudzitis/duplinaut/actions/runs/42187054602);
  do not mark overall acceptance complete until both hosted gates pass.
- Independent verifier review MUST check the model against the project
  `Boundary`/`BackupPath` types, initial/previous-boundary cases, and the
  pending block relation. Independent implementation review MUST confirm the
  production path invokes the helper and that no entry/payload/pending state
  is changed on an accepted empty volume.

## Exclusions

This slice does not prove tar parsing, stored digest correctness, decompression,
envelope or OpenPGP policy, volume selection/order, queued-restart exceptions,
nonempty volume boundary handling, part-number sequencing generally, metadata
or payload resource accounting, allocator behavior, complete multipart
controller refinement, snapshot reconstruction, or filesystem publication.
It makes no producer claim beyond the documented 3.0.7 empty-volume boundary
profile.

## Checklist

- [x] Hosted Linux assurance and Duvet Pages prerequisite gates for required-
      volume identity and selected-set adapter pass in run 42181068479.
- [x] Verify the empty-volume boundary admission model and project-type
      refinement.
- [x] Test initial empty acceptance, pending-state preservation across an
      empty volume, non-completion without a final part, and malformed
      boundaries.
- [x] Preserve official multipart and queued-restart oracle/regression tests.
- [x] Local Duvet links/queries and trace, formatting, strict Clippy,
      focused/full tests, full `cargo xtask ci`, and independent verifier and
      implementation reviews pass at commit `a3ac918` (source SHA-256
      `767914182c1dbb51001ec7bb3f655b8fa02f9c729a06facd867fbdbeb0982e3e`).
- [ ] Hosted Linux assurance and Duvet Pages pass in run 42187054602.
- [x] Independent specification review accepts this scope before
      implementation.
- [x] Independent verifier and implementation reviews accept the local
      evidence; hosted completion remains pending.
