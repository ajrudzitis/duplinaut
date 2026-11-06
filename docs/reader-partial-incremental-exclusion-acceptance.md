# Official Plain Reader Partial-Incremental Exclusion Acceptance

**Status:** The bounded fixture test and local scoped verification pass.
Independent implementation review, full local `cargo xtask ci`, and hosted
Linux assurance/Pages remain pending.

## Goal and scope

Show that a listed partial first-incremental manifest stays visible in the
catalog but cannot bridge the official plain chain from its full set to the
retained later incremental set. The public read MUST report the resulting
missing transition. This is one first-incremental fixture case, scoped to
`PARSE-NAME-005` and `PARSE-CHAIN-002`; it is not a general chain-controller
claim.

## Fixture mutation and identity oracle

Start with the exact twelve-object inventory in
`tests/fixtures/duplicity-3.0.7/plain/`. Rename only the first incremental
manifest key:

```text
backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest
    -> backup/duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.part
```

The renamed object MUST retain the original manifest bytes, stored size, and
content-derived ETag; the local source uses its usual synthetic version ID.
No object is added or removed, the listing remains exactly twelve objects,
and the only key-set difference is this rename. The full manifest, second
incremental manifest, every data volume, and both signature archives MUST
remain present with their original stored bytes, sizes, and fixture-derived
source identities.

The new basename is below the default 4096-byte name limit, contains neither
NUL nor `/`, and uses valid default `NameConfig`. The test-local identity is
not an S3 version/ETag claim.

## Required public observations

1. Discover through public `Catalog::discover`. The renamed first incremental
   manifest MUST remain visible in `Catalog::objects()` with
   `name().partial == true` and `DiscoveryStatus::Partial`.
2. Collect `Catalog::manifests()` and assert it contains exactly two
   descriptors: a Full descriptor with start/end `1700000000`, and the retained
   later Incremental descriptor with start `1700086400` and end `1700172800`.
   Assert the descriptor kinds and times explicitly. The later incremental is
   a non-vacuity control: the catalog retains a downstream link, while the
   first transition from the full end is missing.
3. Obtain the Full descriptor's index from `Catalog::manifests()` and call
   public `Catalog::read_snapshot` with that index and
   `Timestamp(1700172800)`. It MUST return exactly
   `Err(ReaderError::Chain(ChainError::Missing))`; no snapshot may be returned.

## Requirements

1. MUST preserve the twelve-object fixture inventory and original stored
   bytes; the only mutation is the exact first-incremental key rename to
   `.manifest.part`. Assert the renamed payload and size are unchanged and the
   later incremental manifest remains present.
2. MUST keep the partial incremental visible as `DiscoveryStatus::Partial`
   while excluding it from `Catalog::manifests()`. Annotate this exclusion to
   `PARSE-NAME-005`.
3. MUST assert the exact two completed descriptors and their set kinds/times:
   Full `(1700000000, 1700000000)` and later Incremental
   `(1700086400, 1700172800)`. This establishes that the later set is retained
   and the required first bridge is absent.
4. MUST derive the Full index from the public manifests iterator and call
   `Catalog::read_snapshot` for target `Timestamp(1700172800)`. It MUST return
   exactly `ReaderError::Chain(ChainError::Missing)` with no snapshot.
   Annotate this missing-set result to `PARSE-CHAIN-002`.
5. MUST keep the acceptance fixture-local; it requires no live S3, external
   credentials, or filesystem publication.

## Verification and review

- Add a focused integration test in `crates/core/tests/reader.rs`, distinct
  from both the removed-volume case and partial full-manifest case.
- Link the reader partial-manifest filter and its test to `PARSE-NAME-005`;
  link the public missing-chain result and test to `PARSE-CHAIN-002`. Do not
  add `PARSE-CHAIN-001` evidence unless the test separately asserts that
  requirement's full-to-incremental contiguity rule.
- Run focused queries for `PARSE-NAME-005` and `PARSE-CHAIN-002`, then the full
  Duvet trace, focused/full core tests, formatting, strict Clippy, and
  `cargo xtask ci`. Require hosted Linux assurance and Duvet Pages publication
  before acceptance is complete.
- Independent review MUST verify the exact inventory mutation, stable fixture
  payload/identity, visible Partial status, both descriptor values, iterator
  derived Full index, target timestamp, and exact `Chain(Missing)` result.

## Exclusions

This acceptance covers only the official plain first-incremental partial
manifest case and the resulting missing transition. It does not test partial
full manifests or volumes, other envelopes/profiles, arbitrary missing-set
behavior, authentication, S3 identity semantics, filesystem restoration, or
complete reader/chain-controller refinement. It makes no `PARSE-CHAIN-001`
claim and adds no Verus proof obligation.

## Checklist

- [x] Independent specification review accepts scope, fixture, and oracle.
- [x] Exact inventory rename relation and byte/identity preservation pass;
      discovery retains the Partial entry and exactly the specified Full and
      later Incremental descriptors; the iterator-derived full index returns
      exact `Chain(Missing)` at the required target.
- [x] Targeted `PARSE-NAME-005` and `PARSE-CHAIN-002` Duvet queries, full
      `cargo xtask trace`, focused test, full core suite, formatting, strict
      Clippy, and diff check pass locally.
- [x] Independent implementation/test review passes, including the
      order-independent descriptor membership assertion.
- [x] Full local `cargo xtask ci` passes with 2,047, 3,183, and 1,329 verified
      declarations and zero errors; Duvet, strict Clippy, formatting, and
      workspace tests pass.
- [ ] Hosted Linux assurance and Duvet Pages publication pass.

## Current local evidence

- `cargo test -p duplicity-core --test reader
  official_partial_first_incremental_manifest_leaves_missing_chain_transition`:
  passed.
- `cargo test -p duplicity-core`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-name-005`: passed.
- `.tools/bin/duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-chain-002`: passed.
- `DUVET="$PWD/.tools/bin/duvet" cargo xtask trace-update` and `cargo xtask
  trace`: passed.
- `cargo fmt --all -- --check`, `cargo clippy -p duplicity-core --all-targets
  -- -D warnings`, and `git diff --check`: passed.
- Full pinned CI wrote `target/assurance/ci.json` with source digest
  `3b50da120fb6e0779f4dea05ab1f69fa079f2ec8e8b629592c126d439bcabe38`.
