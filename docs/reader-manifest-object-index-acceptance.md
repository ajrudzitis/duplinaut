# Reader Manifest-Object Index Projection Acceptance

**Status:** Local implementation, tests, Verus proof, Duvet trace, and
independent reviews passed. Hosted Linux assurance and Duvet Pages are pending.

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
as `(i, M[i], D[i])`. The model MUST require equal lengths for `M` and `D`,
and every `M[i]` in bounds for `O`. A verified position helper MUST return
the exact descriptor-table position, mapped object-table index, and descriptor
for an in-range index, and no row out of range. The helper does not dereference
`O[M[i]]`; the exact `StoredObject` reference reached by that dereference is
native-test evidence. This proves index/descriptor projection under the stated
preconditions; it does not prove that `Catalog::discover` establishes those
preconditions for all possible inputs.

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
   the native `Iterator` adapter. Prove exact in-range position, mapped object
   index, and descriptor projection plus the out-of-range result. It MUST NOT
   claim that the proof dereferences the object table or proves pointer
   identity. The native fixture test MUST verify the dereference and pointer
   identity against `Catalog::objects()`. State explicitly that Rust's
   `Iterator`, `Enumerate`, `Map`, and slice implementation internals are not
   formally verified; the public iterator is covered by the native test.
5. Add a Duvet link to `PARSE-CHAIN-001` only for the manifest associations
   and selected-chain read behavior described above. Do not tag
   `PARSE-CHAIN-002`, source identity, snapshot transition semantics, or
   resource bounds absent separate assertions.

## Verification and boundaries

- Focused native tests cover the model helper's boundaries and the public
  explicitly reversed-listing catalog fixture. The test MUST assert zero
  opens during discovery, all three public associations with indices 2/1/0,
  and the per-target manifest-only projections from the full open logs.
- Focused/full core tests, the focused Verus proof, targeted `PARSE-CHAIN-001`
  Duvet query and full trace, formatting, strict Clippy, and full local
  `cargo xtask ci` passed. Independent verifier and implementation reviews
  passed. Hosted Linux assurance and Duvet Pages publication remain required.
- The proof covers only an in-range index/descriptor projection from supplied,
  length-aligned tables whose object indices are in bounds. It does not
  dereference the object table. Native fixture tests cover discovery's
  parallel table construction, object dereference/pointer identity, and the
  standard iterator boundary for this official input. This does not establish
  universal construction invariants or complete `read_snapshot_inner`
  correctness.

## Checklist

- [x] Independent specification review accepts the model, fixture oracle,
      and exclusions.
- [x] Implement the verified projection helper and use it in the public
      manifest adapter.
- [x] Reverse-listing fixture test checks exact key/index/descriptor and
      original object reference associations.
- [x] Three selected reads match the official date entry-count oracle and
      open only manifests selected through the descriptor mapping.
- [x] Focused/full tests, Verus, targeted Duvet/full trace, formatting, strict
      Clippy, and local full CI pass.
- [x] Independent verifier and implementation reviews pass.
- [ ] Hosted Linux assurance and Duvet Pages pass.

## Local evidence

The focused projection proof and native tests pass. The full local
`cargo xtask ci` receipt records source SHA-256
`1b4f09883ef980a3a6a8300763e78a6f05f8bf28bb33170e1323357ab74c9c71` and
Verus partitions 2,047 vstd, 3,183 Vest, and 1,332 core obligations with zero
errors. Duvet `PARSE-CHAIN-001` query/full trace, formatting, strict Clippy,
and independent verifier/implementation reviews pass. Hosted Linux assurance
and Duvet Pages remain pending.
