# Live S3 reader acceptance plan

Status: proposed; live execution pending an approved fixture bucket/account and
captured object inventory. No bucket access or provisioning is part of this
specification task.

## Goal and scope

Exercise the actual read-only S3 adapter and reader catalog against one
externally approved S3 mirror of the checked-in official duplicity 3.0.7
`plain/` archive. Compare the complete listed history and the reconstructed
final snapshot with the checked-in producer/source oracle. The test MUST be
explicitly opt-in, use read-only credentials, and never mutate the bucket.

Use `S3Config::connect` to construct `duplicity_s3::S3Source`, then call
`Catalog::discover` and `Catalog::read_snapshot` directly. Do not use a mock
HTTP server, local source, TUI worker, filesystem restore backend, or writer
API in the live case. Existing SDK-mocked tests continue to cover deterministic
pagination, response faults, and source-change behavior.

This plan traces to [PARSE-SOURCE-001](parsing-requirements.md#parse-source-001)
(source identity consistency), [PARSE-CHAIN-001](parsing-requirements.md#parse-chain-001)
(unique contiguous chain), and [PARSE-STATE-001](parsing-requirements.md#parse-state-001)
(snapshot transitions). These are scoped to the live plain-source/catalog path
and completed snapshot. [PARSE-CHAIN-002](parsing-requirements.md#parse-chain-002)
is explicitly outside the live test's trace: this positive mirror case does not
exercise missing-set, missing-volume or ambiguous-branch error distinctions.
Those remain covered by the existing offline reader tests. The S3 fixture SHA-256 checks below compare remote bytes
with the checked-in provenance inventory; they do not claim OpenPGP ciphertext
digest behavior. No RESTORE requirement is linked: this test reconstructs a
snapshot but performs no filesystem restore.

## External prerequisites

Live acceptance MUST remain unchecked until an operator supplies and approves
all of the following:

1. An AWS account, bucket, region and dedicated immutable key prefix approved
   for this test. The prefix MUST mirror exactly the twelve files under
   `tests/fixtures/duplicity-3.0.7/plain/`; provisioning/upload is an external
   setup action, never performed by the test. No other objects or folder-marker
   keys may occur under the test prefix.
2. The approved AWS region for the bucket. The operator MUST supply this
   region explicitly to the opt-in test; SDK region-chain fallback MUST NOT
   silently select a different region for this acceptance.
3. A non-secret inventory file captured from the provisioned objects. It MUST
   record the approved region along with the other source identifiers. For each
   full S3 key it MUST record the corresponding checked-in relative fixture
   path, byte size, SHA-256 from
   `tests/fixtures/duplicity-3.0.7/provenance.json`, exact quoted ETag, version
   ID when versioning supplies one, and observed `LastModified` plus
   `ContentType`/user metadata where present. It MUST also record bucket,
   prefix, region, capture time, and whether bucket versioning is enabled. The
   region MUST equal the explicitly supplied approved region. Inventory
   metadata is fixture provenance; ETag is an identity/conditional token and
   MUST NOT be described as a content digest. Runtime assertions are limited
   to identity fields exposed through ListObjectsV2 and `S3Source::head`; the
   additional LastModified/content/user-metadata values are captured fixture
   provenance, not fields interpreted by the current reader API.
4. A read-only IAM role/profile authorized only to list the approved prefix
   and read/head its fixture objects. The live test MUST NOT have Put, Delete,
   Copy, tagging-write, ACL-write, or bucket-configuration permissions. Use the
   normal AWS SDK credential provider or an approved profile; never put
   credentials in the inventory or test arguments.
5. A protected opt-in runner/environment with network access and the approved
   bucket/account settings. The ordinary workspace CI and `cargo xtask ci`
   MUST NOT contact S3 or require AWS credentials. The live command is run
   explicitly by an authorized operator or protected manual workflow.

The repository already contains a complete local object oracle: provenance
records exactly twelve plain objects totaling 4,156,441 bytes, each object's
size and SHA-256, and `expected.json` records three snapshot dates and source
tree expectations. An implementer can add the opt-in harness, inventory
validation, offline inventory tests, and Duvet annotations without credentials.
Passing those local checks does not count as live S3 evidence; the external
fixture/account/inventory and live run are prerequisites for acceptance.

## Live acceptance requirements

1. The test MUST be ignored or otherwise disabled by default and MUST require
   `DUPLINAUT_LIVE_S3=1`, `DUPLINAUT_LIVE_S3_BUCKET`,
   `DUPLINAUT_LIVE_S3_REGION`, `DUPLINAUT_LIVE_S3_PREFIX`, and
   `DUPLINAUT_LIVE_S3_INVENTORY` before constructing an SDK client. The region
   MUST be nonempty and MUST exactly match the inventory's approved region.
   Set `S3Config.region` from this value; do not rely on SDK region-chain
   fallback to choose the test target. Credentials and optional profile
   selection may come from the standard SDK configuration chain. If opt-in is
   absent, ordinary tests MUST finish without DNS/network access or
   credential-provider calls.
   Missing/malformed inventory or absent required settings MUST fail before
   S3 connection; it MUST NOT silently skip an explicitly requested live run.
2. Configure `S3Config` with exactly the approved bucket, explicitly supplied
   region, prefix, and optional approved profile/endpoint settings. Use finite
   limits: at most 12
   listed objects, at most 64 KiB retained source metadata, and at most 2 MiB
   per stored object. Require each expected object to fit those bounds. Keep
   catalog/reader limits finite and no larger than their reviewed defaults.
   Request and stream-idle deadlines MUST be finite. The checked-in fixture's
   largest plain object is 985,977 bytes; this acceptance claims per-object
   admission and bounded reader behavior, not an aggregate network-byte cap or
   an AWS SDK retry-count proof.
3. Call `Catalog::discover(&source, "", NameConfig::default(), limits,
   &cancel)` and compare its complete `Catalog::objects()` listing with the
   inventory: exactly the same twelve full keys, no extras or missing keys,
   exact size and exact ETag. Verify the expected classifications, including
   exactly three completed manifest descriptors: the full set at
   `1700000000`, then incrementals ending at `1700086400` and `1700172800`.
   This is metadata discovery evidence, not integrity/restorability evidence by
   itself.
4. For each listed object, call `S3Source::head` and require exact key, size,
   ETag, and equality with the inventory's version ID when one is available.
   Then open each listed `StoredObject` through `ObjectSource::open`, stream it
   with a bounded buffer, and require exact inventory byte count and SHA-256
   from `provenance.json`; require true EOF and no extra byte. The S3 adapter
   must use its ETag conditional `GetObject` path. This verifies that the
   approved mirror contains the exact official fixture bytes, including
   signature archives not needed by final snapshot reconstruction.
5. Respect the API distinction: S3 `ListObjectsV2` records key, size and ETag
   but does not provide VersionId to `Catalog`; `S3Source::head` can observe
   VersionId where supported, while reader object opens are conditional on the
   listed ETag. If the external inventory supplies version IDs, require each
   current HEAD to match before and after the snapshot read and require the
   fixture prefix to be writer-quiescent for the test duration. Do not claim
   the catalog read was pinned to a VersionId when it used the ETag condition.
   Where the service lacks versioning, compare pre/post key, size and ETag and
   make no same-ETag replacement detection claim beyond the adapter contract.
6. Select the unique full anchor and call
   `catalog.read_snapshot(anchor, Timestamp(1700172800), &cancel)`. Require a
   completed snapshot and compare every entry against the `plain`, `index = 2`
   row in `tests/fixtures/duplicity-3.0.7/expected.json`: exact raw component
   sequences/boundaries, entry kind, ordinary mode bits, regular-file size and
   SHA-256, directory/file nanosecond mtime, and symlink target bytes. No
   filesystem restore occurs; compare snapshot entries and raw components in
   memory. Preserve the producer's recorded
   `symlink_mtime_ns` exclusion and do not compare symlink mtime. Require 21
   entries and 2,621,564 regular-file payload bytes as aggregate controls.
7. Do not emit credentials, SDK credential-chain details, Authorization
   headers, raw secret/profile contents, object payloads, or unredacted SDK
   errors to test output. Failures MUST identify only the safe stage and
   fixture-relative object ID/index needed for diagnosis. The test MUST make
   no bucket mutation calls; successful execution consists only of list, head,
   and conditional object reads.
8. Duvet MUST trace the live test only to PARSE-SOURCE-001, PARSE-CHAIN-001,
   and PARSE-STATE-001. Do not tag PARSE-CHAIN-002: this live positive case
   does not exercise its missing-set/volume/ambiguity error cases. Do not tag
   filesystem RESTORE requirements or imply
   the live test validates TUI, signer policy, encrypted profiles, complete AWS
   identity semantics, or controller proofs.

## Verification and evidence boundaries

- Without external credentials, validate inventory parsing (including the
  explicit region field and mismatch rejection) against a
  non-secret local fixture, prove the opt-in gate exits before client creation,
  run the SDK-mocked suite, targeted Duvet queries, `cargo xtask trace-update`,
  `cargo xtask trace`, formatting, strict S3/core Clippy, and full offline
  tests. These checks can be implemented before an AWS account is supplied.
- Once approved fixture/account/inventory inputs exist, run the explicit live
  test from an authorized network runner and preserve a redacted receipt with
  inventory digest, run time, safe bucket alias/prefix identifier, object
  counts/bytes, discovered history, snapshot oracle result, and no-write
  statement. Do not include secrets or credential-provider diagnostics.
- Require independent review of inventory-to-oracle mapping, read-only IAM
  scope, no-write call path, exact key/ETag/size/version distinctions,
  conditionally-read object hashing, finite limits, final snapshot comparison,
  redaction, and opt-in behavior. Require hosted assurance and Duvet report
  publication for code/trace gates; require the separate authorized live run
  for live acceptance. No live status may be marked complete until the external
  inputs and actual service run are recorded.

## Current progress

The default-off integration target and synthetic local gate/inventory tests
are implemented in `crates/s3/tests/live_s3.rs`. Focused and full S3 crate
tests, strict Clippy, formatting, Duvet trace and the SOURCE-001, CHAIN-001,
and STATE-001 coverage queries pass locally. The harness requires an explicit
region before inventory I/O, validates the inventory region against that
setting, and sets `S3Config.region` before the first SDK connection. Offline
tests cover absent, blank, malformed, and mismatched region cases and verify
the configured region without constructing a client. Final pinned local CI
passes with Verus partitions 2,047/3,183/1,329 and zero errors; the generated
receipt is `target/assurance/ci.json`. The ignored live case has not been run;
hosted assurance is pending. Bucket/account and region approval, the captured
twelve-object inventory, and read-only credentials are still external
prerequisites.

## Exclusions

This acceptance covers one immutable mirror of the official 3.0.7 plain
fixture and one final snapshot read. It does not test write permissions by
attempting writes, S3 pagination beyond this twelve-object fixture, concurrent
mutation races, multiple regions/providers, endpoint-compatible services,
version-specific GetObject reads, encrypted/signed profiles, historical
producer compatibility, live TUI interaction, restore publication, or formal
proof of the S3 SDK/HTTP/TLS/authentication/reader controllers. Existing
SDK-mocked tests retain coverage for protocol faults, pagination, bounds and
cancellation; this one live case does not replace them.

## Checklist

- [ ] Obtain approved read-only bucket/account/region, immutable prefix and
      captured twelve-object identity/metadata inventory containing the same
      approved region.
- [x] Add a default-off live integration test using `S3Source` and `Catalog`;
      explicitly set `S3Config.region` from `DUPLINAUT_LIVE_S3_REGION` and
      validate opt-in, region match, and inventory handling offline without
      credentials.
- [ ] Assert exact inventory/history and official object hashes, then compare
      the final complete snapshot with the checked-in source oracle.
- [x] Local mocked/offline tests, Duvet trace, formatting, strict Clippy, and
      final pinned `cargo xtask ci` pass.
- [ ] Hosted assurance and Duvet Pages pass.
- [ ] Authorized live S3 execution and independent review pass; record
      redacted evidence without secrets.
