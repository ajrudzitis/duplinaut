<p align="center">
  <img src="assets/branding/duplinaut-logo.png" width="420" alt="Duplinaut — an astronaut carrying a backup folder">
</p>

```text
 ____  _   _ ____  _     ___ _   _    _    _   _ _____
|  _ \| | | |  _ \| |   |_ _| \ | |  / \  | | | |_   _|
| | | | | | | |_) | |    | ||  \| | / _ \ | | | | | |
| |_| | |_| |  __/| |___ | || |\  |/ ___ \| |_| | | |
|____/ \___/|_|   |_____|___|_| \_/_/   \_\\___/  |_|
```

**Browse history. Recover files. Verify the reader.**

**Duplinaut** is a native Rust reader for browsing duplicity backups and restoring selected files from local archives and S3. It runs without duplicity, GnuPG, or librsync executables; the official application supplies independent test fixtures.

[Capabilities and trust boundaries](docs/capabilities.md) | [Requirements](docs/parsing-requirements.md) | [Assurance progress](docs/README.md) | [Tool setup](tools/README.md) | [Official fixtures](tests/fixtures/README.md)

## Where things stand

**Under construction.** Initial reader kernels work; complete repository browsing, restoration, and the terminal interface are still being built. Passing tests, completed proofs, and independent acceptance are recorded separately.

| Component | Current evidence | Remaining work |
| --- | --- | --- |
| Delta decoding and application | 16 tests; official historical and multipart payloads; 535 focused Verus obligations pass; independent semantic review complete | Complete-core acceptance gate |
| Manifest tokens, manifests and byte paths | Official manifests pass; exact token and path refinements checked; 79 whole-manifest obligations pass, including contextual success/error refinement | Complete-core acceptance and selection-controller proofs |
| Signatures and tar containers | Official archives pass; 54 signature and 37 tar-header/helper focused obligations pass | Complete tar/PAX controller refinement |
| Stored-object envelopes | Stored-byte SHA1/MD5 validation; bounded plain/gzip decoding; official-object integration tests | Reader-controller verification |
| Native OpenPGP reader | Private bounded decoder; borrowed reader policy; ciphertext-bound digest permit; encrypted tar completion; bounded evidence; official three-profile, three-date source oracles; reviewed edge tests; authenticated top-off rejection test; scoped Verus cursor/token proof | Native call order remains source-reviewed; independent review of the new top-off test, reader-controller proof, broader algorithms |
| Storage and S3 | Anchored local reads with change detection; 11 S3 tests using actual SDK HTTP handling; independently reviewed complete-reader late-change, cancellation and three-date restore tests | Live-service integration |
| Assurance tooling | Pinned tools, Duvet baseline, executable failure tests, independent review | Expand gates as components are accepted |
| Chain planning | 17 strict obligations pass for exact selection/errors and descriptor provenance; verified selected-position accessor; six official naming profiles | Complete-core acceptance and standard slice iterator semantics |
| Snapshot reconstruction | Three official backup dates match byte paths, contents and metadata; immutable prior-set delta bases; transactional failures | Controller/resource proofs and chain integration |
| Multipart assembly | Official snapshot/delta source oracles; bounded ordered parts; provisional output and sticky failure tests | Formal refinement and footer-bearing/encrypted queued replay |
| Repository browser and restore | Metadata-only catalog and identity-bound plain/gzip reconstruction; byte-preserving selection and reviewed native Linux publication; official three-date oracles | Frontend publication integration and controller proofs |
| Terminal interface | Local/S3 history and explicit full-scan browsing; byte-safe selection; reviewed worker-owned restore publication; 22 tests and real PTY evidence | Credential flow, controller proofs, comprehensive event-loop fault coverage |

The [parsing requirements](docs/parsing-requirements.md) were written in RFC 2119/8174 language and independently reviewed **before implementation**. Public documentation leads format research; immutable official-source references and producer evidence resolve ambiguities.

Successful pushes to `main` publish the rendered Duvet requirements report. [View the latest Duvet report](https://aleksrudzitis.com/duplinaut/) or open the latest **Publish Duvet report** deployment from the [assurance workflow](.github/workflows/assurance.yml). Pull requests keep the report as a downloadable workflow artifact without publishing a site.

## Built around evidence

Vest DSL definitions produce the parsing code used at runtime. Verus proofs connect that implementation to independently stated format and reconstruction semantics. A [documented direct-combinator fallback](crates/core/README.md#vest-dsl-and-fallback-record) handles a reviewed Vest frontend limitation.

Duvet connects requirement headings to implementation and test evidence. A committed snapshot detects traceability regressions; coverage queries enforce accepted milestones. Formal obligations and successful verification runs are indexed separately in the [assurance ledger](docs/assurance.toml). Annotation counts and proof-function counts do not establish semantic coverage.

Official duplicity 3.0.7 fixtures include full and incremental chains, multipart content, byte paths, metadata changes, deletions, symlinks, and encrypted/signed archives. Source-tree expectations were checked against official restoration. The native OpenPGP reader composition for the recorded 3.0.7 symmetric, public-key and signed profiles is independently accepted; broader historical crypto support and complete-reader formal acceptance remain pending.

Cryptography, compression, hashing, networking, and filesystem operations have explicit trusted adapter boundaries. Their integration tests and contract reviews are distinct from proofs of deterministic core algorithms. Specification, software, and verification agents work separately, with frozen executable handoffs for proof work.

## Workspace

| Crate | Responsibility |
| --- | --- |
| [`duplicity-core`](crates/core) | Byte-preserving parsing, backup history, reconstruction, and restore contracts |
| [`duplicity-s3`](crates/s3) | Read-only S3 access; AWS dependencies stay outside the core |
| [`duplicity-tui`](crates/tui) | Terminal browsing and restoration |
| [`xtask`](xtask) | Generation, verification, traceability, and acceptance checks |

## Development

Start with the [pinned tool setup](tools/README.md), then use:

```sh
cargo xtask doctor       # Check tool and dependency identities
cargo xtask build        # Verify, then build production code
cargo xtask test         # Native tests and official-produced fixtures
cargo xtask trace        # Duvet snapshot and accepted-milestone coverage
cargo xtask ci           # Full acceptance checks
```

`build` verifies by default. Missing tools, stale generated parsers, failed proofs, incomplete required evidence, or unreviewed assumptions fail the gate. The full core proof gate remains in progress; an ordinary Cargo build is a development check and carries no verification claim.

`cargo xtask generate` updates generated parsers for review. `cargo xtask trace-update` explicitly updates the Duvet baseline for review and commit. `cargo xtask producer` regenerates official fixtures with explicit tools and a new output directory, as documented in [the producer guide](tests/producer/README.md).

## Backlog and milestones

Tasks are checked only when their stated evidence passes. Each completed step gets a local Git commit, and useful working checkpoints record unfinished assurance work explicitly.

- [x] Write and independently review parsing requirements before implementation.
- [x] Record immutable format provenance and explicit implementation holds.
- [x] Pin Rust, Verus, Vest, Duvet, and solver versions.
- [x] Establish reviewed Duvet extraction, snapshots, and milestone coverage gates.
- [x] Establish independently reviewed xtask generation and verification tooling.
- [x] Produce official 3.0.7 fixtures with independent source/restore oracles.
- [x] Produce historical official fixtures for 0.8.23, 2.1.3 and 3.2.1, including short and legacy names.
- [x] Implement and independently review delta semantic contracts; pass focused strict verification and official payload tests.
- [x] Implement bounded manifests and test official producer records.
- [x] Integrate exact token, path, signature and tar helper refinement checkpoints.
- [x] Validate stored-byte volume digests and decode bounded plain/gzip envelopes against official objects.
- [x] Implement and review the read-only S3 adapter; test conditional reads, bounds, cancellation and persistent stream failures.
- [x] Exercise complete-reader S3 SDK transfers against official source and restore oracles, including late mutation and cancellation; independently review the test harness.
- [x] Integrate metadata-only discovery and bounded plain/gzip chain reconstruction against original official objects.
- [x] Independently review repository orchestration; test late incremental failures and cancellation.
- [x] Test exact reader object-comparison, decoded-manifest and normalization-metadata budget boundaries.
- [ ] Prove repository orchestration controller contracts and complete resource accounting. Scoped checked-stream size admission, window/progress/EOF decisions and failure latching, local validated-read offset commit, per-object comparison charging, unique candidate accumulation, cumulative work reservation, ordered volume and selected-set completion cursors, final publication readiness, catalog metadata charging, and authentication-evidence source/object/version/policy count, byte, and certificate-slot reservations are independently accepted; iterator semantics, cancellation timing, and complete resource accounting remain open.
- [x] Implement a local read-only source with anchored identity checks; test actual files and official three-date reconstruction.
- [x] Build read-only terminal history and snapshot browsing; exercise an official chain in a real terminal.
- [x] Add bounded restore previews, frozen confirmation and worker-owned publication; independently review the terminal lifecycle, run 22 tests and verify a real PTY restore.
- [x] Accept delta, manifest-token, byte-path, signatures and manifest-record components with exact contracts, fresh full verification, official tests and independent review.
- [x] Contract-refine the typed `body()` handler selection for all six object kinds and both name profiles; per-handler field parsing and complete filename classification remain open.
- [x] Prove successful `body()` results preserve the selected object kind and short/long profile across all twelve routes; route-wide body-field composition and outer classification remain open.
- [x] Independently review and verify the raw `ShortFullManifest` body route: successful exact dot/suffix decomposition composes with its semantic field model; failure behavior and the remaining routes stay open.
- [x] Independently review and verify the raw `ShortFullSignatures` body route: successful exact dot/suffix decomposition composes with its semantic field model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw `ShortFullVolume` body route: successful exact dot/suffix decomposition composes with its short timestamp, positive base36 volume, and trailer/suffix models; error outcomes and other routes remain open.
- [x] Refine `ShortFullVolume` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine `ShortFullSignatures` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine `ShortFullManifest` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine `ShortIncrementalManifest` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine `ShortIncrementalSignatures` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine `ShortIncrementalVolume` error results: every helper error is exactly `Malformed`, including with an invalid time separator, while successful outputs remain tied to the existing raw semantic body model. The valid/malformed edge matrix is tested; the reverse raw-model-to-success implication, other routes, and complete filename grammar remain open.
- [x] Refine long `FullManifest` body errors: every helper error is exactly `Malformed`, including invalid separator configuration, while successful outputs remain tied to the existing raw body model. Canonical, legacy, malformed and invalid-separator cases are tested; the reverse raw-model-to-success implication and complete filename grammar remain open.
- [x] Independently review and verify the raw `ShortIncrementalManifest` body route: successful exact dot/suffix decomposition composes with the increasing timestamp interval and trailer/suffix models; error outcomes and other routes remain open.
- [x] Independently review and verify the raw `ShortIncrementalSignatures` body route: successful exact dot/suffix decomposition composes with its increasing timestamp interval and signature trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw `ShortIncrementalVolume` body route: the successful result contract composes exact dot/suffix decomposition with its increasing timestamp interval, positive base36 volume, and trailer/suffix model. A direct six-profile plus nontrivial base36 result matrix is tested; the reverse implication, error completeness, and other routes remain open.
- [x] Independently review and verify the raw long `FullManifest` body route: successful exact dot/suffix decomposition composes with its canonical/legacy timestamp and manifest trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw long `FullSignatures` body route: successful exact dot/suffix decomposition composes with its canonical/legacy timestamp and signature trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw long `FullVolume` body route: successful exact dot/suffix decomposition composes with its canonical/legacy timestamp, positive decimal volume, and volume trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw long `IncrementalManifest` body route: successful exact dot/suffix decomposition composes with its configured timestamp separator, increasing interval, and manifest trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw long `IncrementalSignatures` body route: successful exact dot/suffix decomposition composes with its configured timestamp separator, increasing interval, and signatures trailer/suffix model; error outcomes and other routes remain open.
- [x] Independently review and verify the raw long `IncrementalVolume` body route: successful exact dot/suffix decomposition composes with its configured timestamp separator, increasing interval, positive volume, and volume trailer/suffix model; error outcomes and other routes remain open.
- [x] Prove tar header checksum admission against the exact unsigned-byte and signed-byte checksum sums; test valid examples for both profiles and a checksum mismatch. Whole tar/PAX controller refinement remains open.
- [x] Prove ordinary tar entry-type admission and explicit diagnostics for unsupported encodings; test the supported type matrix, payload-bearing non-regular types, and unknown tags. GNU/PAX extension, header, extent, and full parser refinement remain open.
- [x] Prove the tar entry payload-slice and padded next-cursor adapter against the extent model; test 512-byte boundary sizes and truncated payload diagnostics. Whole-loop cursor invariants and complete tar parser refinement remain open.
- [x] Prove one-slot PAX global/local precedence and empty-local deletion, wire the helper into native PAX resolution, and test an official path override plus adversarial local replacement/deletion. Complete PAX controller and whole tar parser refinement remain open.
- [x] Prove effective ordinary-entry PAX size selection and decimal decoding, connect it to extent sizing, and test global/local overrides, empty-local fallback, invalid values and the official PAX path fixture. Complete PAX record/controller and whole tar parser refinement remain open.
- [x] Prove effective ordinary-entry PAX mtime selection through the verified time model, preserving raw header-second fallback; test signed fractions, precision/range errors, global/local/empty-local behavior and official header-mtime agreement. Extension-record mtime and full PAX/tar controller refinement remain open.
- [x] Prove optional ordinary-entry PAX atime/ctime decoding through the verified time model, preserving absence; test signed fractions, invalid precision/range, global/local/empty-local deletion, and official absence. Extension-record times and full PAX/tar controller refinement remain open.
- [x] Prove ordinary-entry PAX uid/gid decimal override selection, header fallback, and invalid-value rejection; test zero, maximum `u64`, global/local values, deletion, and nonzero header fallback for both fields. Extension-record IDs, PAX map/controller refinement, and the whole tar parser remain open.
- [x] Prove resolved PAX `path` takes precedence over a pending GNU long-name, while an absent effective PAX path selects GNU metadata before the raw header fallback; test both extension orders, local-empty fallback, and the official PAX path fixture. Extension-loop ordering/controller behavior and complete path/header decoding remain open.
- [x] Prove resolved PAX `linkpath` takes precedence over a pending GNU long-link, while an absent effective PAX link selects GNU metadata before the raw header fallback; test both extension orders and local-empty fallback. Extension-loop ordering/controller behavior and complete link/header decoding remain open.
- [x] Prove single-record PAX key admission preserves the known-key index, duplicate and NUL rejection, unknown-key diagnostic, and acceptance of empty values as stored deletion markers. Record framing, value grammars, complete map accumulation, and the PAX controller remain open.
- [x] Prove effective PAX uname/gname selection and the resolved BINARY-versus-UTF-8 policy branch; test fallback/override/local deletion/global resume, valid and invalid UTF-8 PAX values, binary byte preservation, and existing raw-header fallback. Rust's UTF-8 decoder and complete PAX controller remain outside the proof.
- [x] Prove exact admission of the three supported nonempty PAX `hdrcharset` tokens and wire unsupported values to `UnsupportedPax`; test accepted spellings, near-misses, and empty-local deletion of a global BINARY value. Record iteration, UTF-8 decoding, and the complete PAX controller remain open.
- [x] Prove 11-slot PAX global/local resolution composition using the verified per-slot rule; test inherit/delete/override for every slot and check all supported fields through the native entry metadata output. PAX framing, map ingestion, extension timing, and whole-tar controller refinement remain open.
- [x] Independently review and verify the `parse_name` candidate recognition controller: one scan composes all twelve kind/profile entries with ordered configured-prefix matching and exact stem remainders, returning the aligned remainder table and an existential recognized-stem result that `parse_name` reuses.
- [x] Independently prove and test candidate accumulation and final-result composition: successful `ObjectName` values fold left to right through the ambiguity-guarded merge model, then recognized-state finalization is composed with that fold. The collector relates successful body results to the body semantic model. The post-preflight controller helper requires the separately verified valid-preflight model and composes its result with a witness sequence whose twelve slots match the candidate table and body outcome model, whose successful values map to the ambiguity-guarded fold, and whose result is finalized by the recognized-stem state. The public parser invokes this helper only after preflight succeeds; exact parser error outcomes and raw-grammar iff refinement remain open.
- [x] Independently prove and test the body error variant domain across all twelve kind/profile routes: the focused malformed-leading-field matrix returns `Malformed`, and six valid long-route examples exercise invalid separators. The contract restricts errors to `Malformed` or `Configuration` only for an invalid separator; exact raw-grammar iff refinement remains open.
- [x] Prove and test body-outcome collection refinement: each of the twelve table-aligned remainder slots is passed to its matching `body()` route; successful outcomes satisfy the shared raw-body semantic model, while malformed and representative invalid-separator outcomes obey the body-error domain model. The tests cover all 12 malformed collection slots, all 12 valid success routes, and a representative configuration error. Exact malformed-vs-configuration error completeness and raw-grammar iff refinement remain open.
- [x] Prove the post-preflight controller composition on valid-preflight inputs: an abstract witness sequence is constrained by all twelve candidate prefix/stem models and body outcome contracts, successful entries map to the ambiguity-guarded accumulation model, and the actual result equals recognized-state finalization. The public parser invokes the verified controller helper after the separately verified preflight; exact parser error completeness and raw filename grammar iff remain open.
- [x] Prove and test reverse success refinement for the short FullManifest field parser: a valid field-level semantic model must produce the exact `ObjectName` result, and tests assert decoded fields from independently written raw body strings. Scanner-to-field raw-body completeness, other routes, and body-error completeness remain open.
- [x] Prove and test reverse success refinement for the short FullSignatures field parser: a valid field-level semantic model produces the exact `ObjectName` result, and tests assert decoded fields from independently written raw body strings. Scanner-to-field raw-body completeness, other routes, and body-error completeness remain open.
- [x] Prove and test reverse success refinement for the short FullVolume field parser: a valid timestamp, positive base36 volume, and trailer model produce the exact `ObjectName` result, with expected fields asserted for independently written raw body strings. Scanner-to-field raw-body completeness, other routes, and body-error completeness remain open.
- [ ] Prove and test reverse success refinement for the short IncrementalManifest field parser: a valid increasing base36 timestamp interval and trailer model must produce the exact `ObjectName` result, with expected fields asserted for independently written raw body strings. Scanner-to-field raw-body completeness, other routes, and body-error completeness remain open.
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers. Filename checkpoints independently contract-verify exact kind/profile stem selection, candidate accumulation with a distinct-result ambiguity guard (tested at the helper boundary; current grammar may make that outcome unreachable), final classification of unrelated/malformed/valid names, ordered literal global/kind prefixes including exact selection of archive, manifest, or signature prefix for each object kind, dot-separated field boundary and borrowed-slice ranges, all six short object-kind body compositions, and all six long object-kind body compositions; positive short/long volume decoding, full/incremental short timestamp interval policy, short trailer/suffix/envelope/partial classification, positive decimal long-volume decoding, long-name timestamp endpoint/marker/order composition (including the configured timestamp separator), and long trailer/suffix/envelope/partial classification; canonical and legacy timestamp framing for exact supported lengths and punctuation, and numeric field byte slices for those forms, are independently proved and tested; canonical and legacy long timestamp parsing to exact UTC seconds under the current Gregorian epoch-offset model, with invalid separator configuration distinct from malformed bytes, is independently contract-verified. The 1901-01-01 to 1970-01-01 Unix epoch offset (25,202 days) is separately derived from the Gregorian year recurrence, whose 1900/2000/2100 century cases are also proved directly; calendar conversion uses the named offset, and executable tests cover the epoch boundary and leap-day spacing. Gregorian month lengths, leap-year rules, day-of-month bounds, total supported year/month/day validity, validated-field UTC seconds conversion, and composition of validated calendar/clock fields with bounded explicit-zone offsets are independently contract-verified and tested. Complete object-name parser/controller refinement remains open. Zone-token shape, decimal digits, offset bounds, configured separator handling, and separator-byte configuration validity are independently proved and tested. Exact base36 decoding with empty/invalid/overflow rejection, short-name base36 timestamp-to-epoch conversion, and decimal decoding with empty/invalid/overflow rejection are independently verified; completed-artifact policy for partial and unsupported PKCS#7 objects is independently verified and tested. The hour/minute/second range predicate, calendar-date validity predicate, and scalar incremental-end-after-start policy each have their own verified and tested checkpoint; numeric decoding and complete filename grammar remain separate. Shared suffix-field framing, all twelve short/long object-kind route selectors, and the exact-one-route partition across the six kinds and both profiles are independently proved and tested. Other per-kind body helpers and surrounding filename controller refinement remain open. Multipart metadata admission is independently verified.
- [x] Specify, implement, prove, and test `PARSE-NAME-008` basename preflight: the name-size limit takes precedence over NUL/slash rejection. Configured separator/prefix validation and full filename parser/controller refinement remain open.
- [x] Specify, implement, prove, and test `PARSE-NAME-009` configuration preflight: invalid separators and NUL/slash in prefixes return `Configuration` before basename limit or byte errors; valid configuration proceeds to `PARSE-NAME-008`. Full filename parser/controller refinement remains open.
- [x] Independently review and verify the `parse_name` candidate-table invariant: its Verus contract fixes all twelve kind/profile pairs exactly once, with long before short for each kind, and tests cover exact contents, uniqueness, and ordering. Candidate prefix/stem recognition is now composed separately; body outcomes and end-to-end classification remain open.
- [x] Independently verify the byte-prefix stem helper used by `parse_name`: its contract proves the exact borrowed remainder for matches and rejection for differing bytes or a too-short input; tests cover all twelve stems, empty prefixes/remainders, and near misses. Body parsing and complete filename classification remain open.
- [ ] Complete verified snapshot indexing and native Linux restoration. Componentwise charge subtraction, component-boundary prefix classification, bounded regular-file write-offset advancement, chunk selection and composed chunk progress, regular-file size comparison, reverse finalization cursor, the scalar forward-work decrement, creation-journal transitions with a possibly-created cleanup test, cleanup-attempt admission, outer staging result authority, one-shot publication authority, publication cleanup authority, ACL absence evidence and success gating, multidimensional preflight-budget arithmetic, the cleanup schedule bound, paired name/metadata charging, streaming preflight-summary accumulation, and lossless timestamp validation are independently verified; ordered-map refinement, complete state/resource accounting, traversal/fact provenance, parent-FD mapping, syscall refinement, and filesystem-controller proofs remain open.
- [x] Independently review and adopt Linux staging/publication requirements before implementation, including indeterminate syscall outcomes.
- [x] Implement and independently review native Linux restoration; pass official three-date and fault-injection tests, enforce Duvet requirement coverage.
- [x] Implement and independently review the private bounded OpenPGP adapter against all 27 official crypto objects and nine independent content oracles; document its trusted boundary.
- [x] Compose OpenPGP with reader identity, ciphertext digest checks, global signature policy, encrypted tar parsing and whole-chain evidence; official three-profile/three-date tests and independent pipeline review pass. Formal reader-controller/resource proofs, remaining edge-case matrices and broader historical algorithm support remain open.
- [x] Independently review the scoped OpenPGP reader edge tests: public-reader source/body-length failures, signer-downgrade rejection, decoded/evidence caps, certificate metadata, exact post-digest/post-decode cancellation checkpoints, and both private and public nonfinal ciphertext top-off rejection. The public Catalog regression updates the manifest digest for the re-encrypted object, then confirms the authenticated malformed top-off fails without publishing a snapshot.
- [x] Independently review public-reader adversarial tests for wrong symmetric credentials and a same-size mutated official encrypted manifest; both fail closed through typed native OpenPGP errors with no snapshot.
- [x] Independently review public-reader wrong-signer authorization and unsupported v3 PKESK algorithm regressions; both return typed errors and no snapshot through `Catalog::read_snapshot_with_crypto`. Broader crypto profiles and controller coverage remain pending.
- [ ] Validate S3 with a live service and complete-reader consistency integration.
- [ ] Complete the browsing/restoration TUI.
- [ ] Automate full terminal event-loop resize, cancellation and output-fault coverage; finish frontend controller proofs and encrypted credential flow.
- [ ] Pass the supported official-producer compatibility matrix.
- [x] **Create GitHub Actions for builds and formal methods:** install pinned tools, check generated parsers, run Verus, enforce Duvet coverage/snapshot gates, and run native/official-fixture tests on pull requests and pushes; retain reports and verification evidence as artifacts. The hosted assurance gate and first GitHub Pages publication both passed. See [workflow validation](docs/github-actions.md).
- [x] Publish the Duvet HTML report from successful `main` runs through GitHub Pages, with its live URL in the deployment summary.
- [x] Publish the exact supported capabilities, verification claims, and trusted boundaries in [the capability guide](docs/capabilities.md); remaining project-wide assurance gaps are stated explicitly.
