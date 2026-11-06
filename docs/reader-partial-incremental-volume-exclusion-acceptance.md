# Official Plain Reader Partial First-Incremental Volume Acceptance

**Status:** Specification accepted after independent review. Implementation,
focused verification, and hosted assurance are pending.

## Goal and scope

Verify that a partial first-incremental data volume remains visible in
discovery but cannot satisfy the completed restore set. The official first
incremental's volume 2 remains as a later-volume control, and reading through
the first incremental date MUST report its missing volume 1. This is one
fixture mutation, scoped to partial-artifact exclusion and required-volume
completeness; it is not a general reader/controller claim.

## Fixture mutation and inventory oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only first-incremental volume 1:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gz
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.part.gz
```

The renamed object MUST retain its original stored bytes, size, and fixture
source identity. The local source identity is synthetic; no S3 identity claim
is made. No object is added or removed, the listing remains exactly twelve
objects, and the only key-set difference is this rename. All three manifests,
the full-set volumes, first-incremental volume 2, the later-incremental data
volume, and all signature archives MUST remain present with original bytes,
sizes, and identities.

The renamed name is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses default `NameConfig`. It parses as an
`IncrementalVolume` for interval `(1700000000, 1700086400)`, ordinal 1, with
the partial marker.

## Required public observations

1. Call public `Catalog::discover` and assert the renamed object remains in
   `Catalog::objects()` with parsed kind `IncrementalVolume`, start/end
   `1700000000`/`1700086400`, volume `Some(1)`, `name().partial == true`, and
   `DiscoveryStatus::Partial`.
2. Assert the retained first-incremental volume 2 remains visible as a
   completed `IncrementalVolume` for the same interval with volume `Some(2)`.
   It is the non-vacuity control showing that volume 1 is the absent required
   ordinal while another volume from the set remains available.
3. Assert `Catalog::manifests()` retains exactly the official three completed
   descriptors, independently of iterator order: Full
   `(1700000000, 1700000000)`, first Incremental
   `(1700000000, 1700086400)`, and later Incremental
   `(1700086400, 1700172800)`.
4. Derive the Full descriptor index from the public iterator and call
   `Catalog::read_snapshot` at `Timestamp(1700086400)`. It MUST return exactly
   `Err(ReaderError::MissingVolume { number: 1 })`; no snapshot may be
   returned.

## Requirements

1. MUST preserve the exact twelve-object fixture inventory, stored bytes,
   sizes, and fixture source identities. The sole mutation is renaming
   first-incremental volume 1 to `.difftar.part.gz`.
2. MUST keep the renamed volume visible with `name().partial == true` and
   `DiscoveryStatus::Partial`, and exclude it from satisfying the completed
   restore set. Link partial-object exclusion and its test to
   `PARSE-NAME-005`.
3. MUST assert the completed volume-2 control and exact three manifest
   descriptors, without assuming iterator order.
4. MUST derive the Full index from the public manifests iterator, read target
   `Timestamp(1700086400)`, and require exactly
   `ReaderError::MissingVolume { number: 1 }` with no snapshot. Link this
   required-volume result and test to `PARSE-CHAIN-002`.
5. MUST make no S3 identity, filesystem-publication, cryptographic, or
   complete-reader-controller claim.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from the partial full-volume and unsupported first-incremental-volume cases.
- Add Duvet links for partial-volume exclusion to `PARSE-NAME-005` and for the
  exact missing-volume result to `PARSE-CHAIN-002`.
- Run focused Duvet queries for both requirements, full trace, focused/full
  core tests, formatting, strict Clippy, and `cargo xtask ci`. Require hosted
  Linux assurance and Duvet Pages publication before acceptance is complete.
- Independent review MUST verify the single-key mutation, exact inventory and
  byte/size/identity preservation, parsed partial name, completed volume-2
  control, descriptor membership, iterator-derived Full index, target, and
  exact `MissingVolume { number: 1 }` result.

## Exclusions

This acceptance covers only first-incremental volume 1 marked partial in the
official plain fixture. It does not test partial final-incremental manifests,
partial full artifacts, other volume ordinals, other envelopes or profiles,
arbitrary missing-volume behavior, S3 semantics, filesystem restoration,
cryptography, complete reader/controller refinement, or a formal reader
proof.

## Checklist

- [x] Independent specification review accepts the scope, fixture mutation,
      and exact public oracle.
- [ ] Exact twelve-object mutation and byte/size/identity preservation pass;
      discovery retains the partial volume and completed volume 2.
- [ ] The three descriptors remain present independent of order; the
      iterator-derived Full read at target `1700086400` returns exact
      `MissingVolume { number: 1 }`.
- [ ] Targeted Duvet queries/full trace, focused/full core tests, formatting,
      strict Clippy, and local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
