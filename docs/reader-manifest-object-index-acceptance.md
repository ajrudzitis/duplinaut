# Reader Manifest-Object Index Projection Acceptance

**Status:** Proposed; implementation and independent review pending.

## Goal and scope

Verify the bounded catalog adapter that joins the manifest-object index table
to the corresponding stored manifest descriptors. `Catalog::manifests()` MUST
expose each completed manifest with its exact descriptor index, original
`StoredObject`, and parsed set interval. `read_snapshot` MUST use the same
descriptor-to-object correspondence when opening selected manifests.

This targets one open reader-controller boundary: `Catalog::discover` builds
`manifest_objects` and `descriptors` in parallel, while `Catalog::manifests`
and `read_snapshot_inner` consume the resulting index mapping. The acceptance
does not prove catalog construction for arbitrary sources, standard-library
iterator internals, chain selection, volume matching, or complete reader
refinement.

## Requirement and model

Link the observable fixture behavior to
[PARSE-CHAIN-001](parsing-requirements.md#parse-chain-001), limited to the
Full and contiguous Incremental manifest descriptors and the selected reads
that use them.

For catalog object sequence `O`, manifest-object index sequence `M`, and
descriptor sequence `D`, define the adapter result at manifest position `i`
as `(i, O[M[i]], D[i])`. The model MUST require equal lengths for `M` and
`D`, and every `M[i]` in bounds for `O`. A verified position helper MUST
return the exact pair for an in-range index and no pair out of range. This
proves the index projection under those explicit preconditions; it does not
prove that `Catalog::discover` establishes them for all possible inputs.

## Acceptance requirements

1. Start with the unchanged twelve-object official plain 3.0.7 fixture. Sort
   its `StoredObject` listing by full key in ascending byte order, then reverse
   that vector before public `Catalog::discover`. Discovery MUST preserve
   this supplied object order and remain metadata-only; the catalog's separate
   sorted vector is used for duplicate checks and MUST NOT be treated as the
   stored object/descriptor order.
2. The public `Catalog::manifests()` output MUST contain exactly three rows.
   Independently assert the set of exact `(manifest key, returned index,
   kind, start, end)` relationships:

   | Key | Index | Kind | Start | End |
   | --- | ---: | --- | ---: | ---: |
   | `backup/duplicity-full.20231114T221320Z.manifest` | 2 | Full | 1700000000 | 1700000000 |
   | `backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest` | 1 | Incremental | 1700000000 | 1700086400 |
   | `backup/duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest` | 0 | Incremental | 1700086400 | 1700172800 |

   For each row, require the returned object reference to be pointer-identical
   to the same-key object exposed by `Catalog::objects()`. The returned index
   is the preserved descriptor-table position, not chain order.
3. Through the same catalog, read the official plain targets at
   `Timestamp(1700000000)`, `Timestamp(1700086400)`, and
   `Timestamp(1700172800)`, deriving the Full anchor from `Catalog::manifests()`.
   Require successful reads and the official oracle entry counts for indices
   0, 1, and 2. Clear the source open log before each read. Filter the log to
   manifest keys (volume objects also open) and require this exact projection:
   Full only; Full plus first Incremental; Full plus both Incrementals. No
   later manifest may be opened for an earlier target. The final read MUST
   preserve the existing full snapshot oracle check.
4. The formal position helper MUST have a project-owned model independent of
   the native `Iterator` adapter. Prove exact in-range index/object/descriptor
   projection and the out-of-range result. State explicitly that Rust's
   `Iterator`, `Enumerate`, `Map`, and slice implementation internals are not
   formally verified; the public iterator is covered by the native test.
5. Add a Duvet link to `PARSE-CHAIN-001` only for the manifest associations
   and selected-chain read behavior described above. Do not tag
   `PARSE-CHAIN-002`, source identity, snapshot transition semantics, or
   resource bounds absent separate assertions.

## Verification and boundaries

- Add focused native tests for the model helper's boundaries and the public
  explicitly reversed-listing catalog fixture. The test MUST assert zero
  opens during discovery, all three public associations with indices 2/1/0,
  and the per-target manifest-only projections from the full open logs.
- Run focused/full core tests, the focused Verus proof, targeted
  `PARSE-CHAIN-001` Duvet query and full trace, formatting, strict Clippy, and
  full `cargo xtask ci`. Require independent verifier and implementation
  reviews, then hosted Linux assurance and Duvet Pages publication.
- The proof covers only an in-range projection from supplied, length-aligned
  index/descriptor tables. Native fixture tests cover the discovery-to-table
  construction and standard iterator boundary for this official input. This
  does not establish universal construction invariants or complete
  `read_snapshot_inner` correctness.

## Checklist

- [ ] Independent specification review accepts the model, fixture oracle,
      and exclusions.
- [ ] Implement the verified projection helper and use it in the public
      manifest adapter.
- [ ] Reverse-listing fixture test checks exact key/index/descriptor and
      original object reference associations.
- [ ] Three selected reads match the official date entry-count oracle and
      open only manifests selected through the descriptor mapping.
- [ ] Focused/full tests, Verus, targeted Duvet/full trace, formatting, strict
      Clippy, and local full CI pass.
- [ ] Independent verifier and implementation reviews pass.
- [ ] Hosted Linux assurance and Duvet Pages pass.
