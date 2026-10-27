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
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers. Filename checkpoints independently contract-verify exact kind/profile stem selection, candidate accumulation with a distinct-result ambiguity guard (tested at the helper boundary; current grammar may make that outcome unreachable), final classification of unrelated/malformed/valid names, ordered literal global/kind prefixes, dot-separated field boundary and borrowed-slice ranges, all six short object-kind body compositions, and all six long object-kind body compositions; positive short/long volume decoding, full/incremental short timestamp interval policy, short trailer/suffix/envelope/partial classification, positive decimal long-volume decoding, long-name timestamp endpoint/marker/order composition (including the configured timestamp separator), and long trailer/suffix/envelope/partial classification; canonical and legacy timestamp framing for exact supported lengths and punctuation, and numeric field byte slices for those forms, are independently proved and tested; canonical and legacy long timestamp parsing to exact UTC seconds under the current Gregorian epoch-offset model, with invalid separator configuration distinct from malformed bytes, is independently contract-verified. The 1901-01-01 to 1970-01-01 Unix epoch offset (25,202 days) is separately derived from the Gregorian year recurrence, whose 1900/2000/2100 century cases are also proved directly; calendar conversion uses the named offset, and executable tests cover the epoch boundary and leap-day spacing. Gregorian month lengths, leap-year rules, day-of-month bounds, total supported year/month/day validity, validated-field UTC seconds conversion, and composition of validated calendar/clock fields with bounded explicit-zone offsets are independently contract-verified and tested. Complete object-name parser/controller refinement remains open. Zone-token shape, decimal digits, offset bounds, configured separator handling, and separator-byte configuration validity are independently proved and tested. Exact base36 decoding with empty/invalid/overflow rejection, short-name base36 timestamp-to-epoch conversion, and decimal decoding with empty/invalid/overflow rejection are independently verified; completed-artifact policy for partial and unsupported PKCS#7 objects is independently verified and tested. The hour/minute/second range predicate, calendar-date validity predicate, and scalar incremental-end-after-start policy each have their own verified and tested checkpoint; numeric decoding and complete filename grammar remain separate. Shared suffix-field framing, all twelve short/long object-kind route selectors, and the exact-one-route partition across the six kinds and both profiles are independently proved and tested. Other per-kind body helpers and surrounding filename controller refinement remain open. Multipart metadata admission is independently verified.
- [x] Specify, implement, prove, and test `PARSE-NAME-008` basename preflight: the name-size limit takes precedence over NUL/slash rejection. Configured separator/prefix validation and full filename parser/controller refinement remain open.
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
