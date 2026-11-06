# Multipart final-publication gate acceptance plan

Status: proposed; specification only. Independent specification review,
implementation, and verification are pending.

## Goal and scope

Specify, prove, and test the final decision made by
`MultipartAssembler::finish` in `crates/core/src/multipart.rs`. The gate returns
one of three outcomes: a latched prior failure wins as `Poisoned`; otherwise
an incomplete volume admission or a pending logical entry returns `Incomplete`;
only the remaining state is `Ready` to return an `AssembledArchive`.

This is the final assembler publication decision only. A `Ready` outcome means
that the existing assembler state passes this gate and may be returned. It
does not mean the filesystem has been published, and it does not establish the
correctness or provenance of the entries or prior input processing.

## Requirement and model

Trace the gate and its focused native tests to
[PARSE-MULTI-003](parsing-requirements.md#parse-multi-003), which requires
multipart completion to be established before a reconstructed file is
reported complete. The trace MUST be limited to the final decision conditions;
it MUST NOT imply that this gate proves the admission cursor or pending-entry
state was correctly produced.

Use a minimized three-boolean model:

```text
enum FinishGate { Poisoned, Incomplete, Ready }

finish_gate(failed, admission_complete, pending) =
    if failed                    { Poisoned }
    else if !admission_complete   { Incomplete }
    else if pending              { Incomplete }
    else                         { Ready }
```

The decision precedence MUST be exactly equivalent to this model for all
eight input combinations. In particular, `failed == true` MUST produce
`Poisoned` even if admission is incomplete and/or an entry remains pending.
When `failed == false`, either incomplete admission or a pending entry MUST
produce `Incomplete`; only `failed == false`, complete admission, and no
pending entry may produce `Ready`.

## Acceptance requirements

1. Add a small pure Verus-visible helper, such as
   `verified::multipart_finish::finish_gate`, over `failed`,
   `admission_complete`, and `pending`. Its postcondition MUST prove exact
   equivalence to the independent three-way model for every boolean tuple.
2. `MultipartAssembler::finish` MUST call this helper with the production
   observations `self.failed`, `self.admission.is_complete()`, and
   `self.pending.is_some()`. It MUST map `Poisoned` to exactly
   `Err(AssemblyError::Poisoned)`, `Incomplete` to exactly
   `Err(AssemblyError::Incomplete)`, and construct/return `AssembledArchive`
   only for `Ready`. The implementation review MUST verify this direct
   call-site refinement and the branch-to-result mapping.
3. A native helper test MUST enumerate all eight `(failed,
   admission_complete, pending)` combinations and assert the exact model result
   for each, including all four `failed == true` precedence cases.
4. Native tests MUST exercise `MultipartAssembler::finish` itself for each
   observable outcome:
   - A latched failure with an otherwise incomplete state, and a latched
     failure with a pending entry, return `Poisoned` rather than `Incomplete`.
   - Incomplete admission with no pending entry returns `Incomplete`.
   - Complete admission with a pending entry returns `Incomplete`.
   - Complete admission, no pending entry, and no latched failure returns an
     `AssembledArchive` containing the already completed entries.
   Existing `all_volumes_integrity_container_limits_and_poisoning`,
   `empty_volume_after_nonfinal_part_does_not_complete_logical_entry`, and
   `continuation_markers_zero_final_and_empty_volumes` are relevant production
   scenarios; strengthen or reuse them without changing their broader claims.
   A test-local assembler-state constructor MAY be used for combinations that
   valid production input cannot independently reach, but it MUST only vary
   the three inputs to the finish gate and must still invoke production
   `finish()` for the call-site assertions.
5. Existing multipart boundary, official snapshot/delta oracle, sticky failure,
   and OpenPGP admission tests MUST continue to pass unchanged in behavior.

## Duvet and verification evidence

- Link the verified helper, its eight-row truth table, and the production
  `finish()` outcome tests to PARSE-MULTI-003 only where their assertions
  exercise its final completion gate.
- Add the helper and relevant test names to the `multipart-assembly`
  `docs/assurance.toml` evidence mapping. Do not mark the complete multipart
  controller milestone accepted by adding this bounded proof.
- Run focused Verus obligations and focused multipart tests, targeted Duvet
  queries, `cargo xtask trace-update`, `cargo xtask trace`, formatting, strict
  Clippy, and full `cargo xtask ci`. Require hosted Linux assurance and Duvet
  Pages publication before marking this acceptance complete.
- Independent verifier review MUST check all eight model combinations and
  exact precedence. Independent implementation review MUST check that the
  production `finish()` calls the helper and only constructs an archive on
  `Ready`.

## Exclusions

This acceptance does not prove entry parsing/classification or content
correctness, path or manifest boundary validation, tar/container completeness,
stored digest or source identity checks, volume selection or admission-cursor
provenance, pending-part transition correctness, resource accounting or
allocator behavior, OpenPGP/crypto, snapshot-state reconstruction, filesystem
publication, or complete multipart-controller refinement. It proves only the
final three-condition decision over supplied boolean observations.

## Checklist

- [ ] Specify and verify the three-way decision for all eight boolean inputs.
- [ ] Test all helper truth-table rows and the `finish()` result mapping for
      Poisoned, Incomplete, and Ready.
- [ ] Preserve official multipart, boundary, poisoning, and OpenPGP regression
      coverage.
- [ ] Duvet links/queries, trace, formatting, strict Clippy, focused/full CI,
      hosted Linux assurance, and Duvet Pages pass.
- [ ] Independent specification, verifier, and implementation reviews accept
      the scoped evidence.
