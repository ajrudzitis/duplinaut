# Official Plain Reader Partial-Volume Exclusion Acceptance

**Status:** Specification accepted after independent review. Implementation,
Duvet evidence, and CI remain pending.

## Goal and scope

Show that one listed partial volume from the official plain 3.0.7 fixture stays
visible as a partial catalog object but cannot satisfy a required completed
volume during the public `Catalog::read_snapshot` operation. The full backup's
required ordinal 2 is the only changed artifact; complete volume 3 remains as a
non-vacuity control. This exercises `PARSE-NAME-005` at the reader boundary and
the exact missing-volume outcome required by `PARSE-CHAIN-002`.

## Fixture mutation and identity oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. The test MUST retain every original
object except for a single rename of the full-set volume 2 key:

```text
backup/duplicity-full.20231114T221320Z.vol2.difftar.gz
    -> backup/duplicity-full.20231114T221320Z.vol2.difftar.part.gz
```

The replacement MUST use the original volume 2 stored bytes unchanged. Its
size and content-derived ETag therefore remain exactly those of the original
object; the local `Source` harness supplies its normal synthetic version ID.
No object is deleted or added, so the listing remains twelve objects and the
only key-set difference is the stated rename. All other keys, stored bytes,
sizes, and fixture-derived identities MUST remain unchanged.

Before the read, assert the catalog contains the renamed key with a parsed name
whose `partial` field is true and `DiscoveryStatus::Partial`. Also assert the
complete full-set volume 3 key is still present with its original bytes and a
completed name. These checks ensure the read failure is attributable to the
partial replacement of ordinal 2, not to an absent fixture or an incomplete
source setup.

The renamed basename remains below the default 4096-byte name bound, contains
no NUL or `/`, and is parsed with valid default `NameConfig`. The test source
MUST return the same `StoredObject` identity from listing and opening the
renamed key. Its version ID is synthetic and its ETag is derived from the
unchanged fixture bytes; this is test-local source metadata and makes no claim
about S3 version/ETag behavior.

## Requirements

1. MUST use the official plain fixture inventory and perform only the single
   volume-2 key rename above. Assert the exact twelve-key inventory relation,
   byte equality and unchanged stored size for the renamed artifact, plus
   unchanged volume 3 presence. The full volume-3 assertion is a required
   non-vacuity control.
2. MUST discover the catalog through public `Catalog::discover`, then assert
   the renamed volume-2 entry is visible, has `partial == true`, and reports
   `DiscoveryStatus::Partial`. Annotate the partial exclusion behavior to
   `PARSE-NAME-005`.
3. MUST call public `Catalog::read_snapshot` for the full anchor and exact
   2023-11-14 target (`Timestamp(1700000000)`). It MUST return exactly
   `Err(ReaderError::MissingVolume { number: 2 })`; the call MUST return no
   snapshot. Annotate this required-volume failure to `PARSE-CHAIN-002`.
4. MUST ensure the partial artifact cannot act as completed evidence: the
   control must fail if it returns a snapshot or a different error, and MUST
   not silently satisfy volume ordinal 2 from the renamed object.
5. MUST keep this acceptance fixture-local. No live S3 access, external
   credentials, or filesystem publication is required.

## Verification and review

- Add a focused integration case in `crates/core/tests/reader.rs`, distinct
  from the existing case that removes volume 2 entirely.
- Add Duvet links for the reader's partial-completion filter and its test under
  `PARSE-NAME-005`, and for the exact `MissingVolume { number: 2 }` result and
  test under `PARSE-CHAIN-002`. Run focused queries and the complete trace.
- Run the focused test and full core suite, formatting, strict Clippy, and
  `cargo xtask ci`; require hosted Linux assurance and Duvet Pages publication
  before acceptance is complete.
- Independent review MUST verify the fixture inventory changes only by the
  specified rename, that original bytes/size are reused, volume 3 remains, the
  catalog exposes a partial object, and the public read reports exactly
  ordinal 2 missing.

## Exclusions

This acceptance covers only one partial full-set data volume at ordinal 2 in
the official plain profile. It does not test partial manifests, partial
incrementals, other suffix/envelope profiles, general missing-volume behavior,
the complete reader/controller, volume-content validation, S3 identity
semantics, or filesystem restoration. It adds no new Verus proof obligation
and does not claim a formal proof of reader-level partial exclusion.

## Checklist

- [x] Independent specification review accepts scope, mutation, and oracle.
- [ ] Focused partial-volume reader test and Duvet links pass independent
      review.
- [ ] Focused/full tests, Duvet queries/trace, formatting, strict Clippy, and
      local `cargo xtask ci` pass.
- [ ] Independent implementation/test review passes.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.
