```text
 ____  _   _ ____  _     ___ ____ ___ _____ __   __
|  _ \| | | |  _ \| |   |_ _/ ___|_ _|_   _|\ \ / /
| | | | | | | |_) | |    | | |    | |  | |   \ V /
| |_| | |_| |  __/| |___ | | |___ | |  | |    | |
|____/ \___/|_|   |_____|___\____|___| |_|    |_|

             B A C K U P   R E A D E R
```

**Browse history. Recover files. Verify the reader.**

A native Rust project for browsing duplicity backups and restoring selected files from local archives and S3. The reader runs without duplicity, GnuPG, or librsync executables; the official application supplies independent test fixtures.

[Requirements](docs/parsing-requirements.md) | [Assurance progress](docs/README.md) | [Tool setup](tools/README.md) | [Official fixtures](tests/fixtures/README.md)

## Where things stand

**Under construction.** Initial reader kernels work; complete repository browsing, restoration, and the terminal interface are still being built. Passing tests, completed proofs, and independent acceptance are recorded separately.

| Component | Current evidence | Remaining work |
| --- | --- | --- |
| Delta decoding and application | 16 tests; official historical and multipart payloads; 535 focused Verus obligations pass; independent semantic review complete | Complete-core acceptance gate |
| Manifest tokens, manifests and byte paths | Official manifests pass; exact token and path refinements checked; 79 whole-manifest obligations pass, including contextual success/error refinement | Complete-core acceptance and selection-controller proofs |
| Signatures and tar containers | Official archives pass; 54 signature and 37 tar-header/helper focused obligations pass | Complete tar/PAX controller refinement |
| Stored-object envelopes | Stored-byte SHA1/MD5 validation; bounded plain/gzip decoding; official-object integration tests | Native OpenPGP and controller verification |
| Storage and S3 | Anchored local reads with change detection; 11 S3 tests using actual SDK HTTP handling; independently reviewed complete-reader late-change, cancellation and three-date restore tests | Live-service integration |
| Assurance tooling | Pinned tools, Duvet baseline, executable failure tests, independent review | Expand gates as components are accepted |
| Chain planning | 17 strict obligations pass for exact selection/errors and descriptor provenance; six official naming profiles | Complete-core acceptance and thin iterator adapter proof |
| Snapshot reconstruction | Three official backup dates match byte paths, contents and metadata; immutable prior-set delta bases; transactional failures | Controller/resource proofs and chain integration |
| Multipart assembly | Official snapshot/delta source oracles; bounded ordered parts; provisional output and sticky failure tests | Formal refinement and footer-bearing/encrypted queued replay |
| Repository browser and restore | Metadata-only catalog and identity-bound plain/gzip reconstruction; byte-preserving selection and reviewed native Linux publication; official three-date oracles | Frontend publication integration and controller proofs |
| Terminal interface | Local/S3 metadata history and explicit full-scan browsing; byte-safe paths and cancellation | Publication interaction, detailed progress and automated interaction coverage |

The [parsing requirements](docs/parsing-requirements.md) were written in RFC 2119/8174 language and independently reviewed **before implementation**. Public documentation leads format research; immutable official-source references and producer evidence resolve ambiguities.

After GitHub Pages is configured to use Actions, successful pushes to `main` publish the rendered Duvet requirements report. Open the latest **Publish Duvet report** deployment from the [assurance workflow](.github/workflows/assurance.yml) to view it; GitHub displays the report URL in the deployment summary. Pull requests keep the report as a downloadable workflow artifact without publishing a site.

## Built around evidence

Vest DSL definitions produce the parsing code used at runtime. Verus proofs connect that implementation to independently stated format and reconstruction semantics. A [documented direct-combinator fallback](crates/core/README.md#vest-dsl-and-fallback-record) handles a reviewed Vest frontend limitation.

Duvet connects requirement headings to implementation and test evidence. A committed snapshot detects traceability regressions; coverage queries enforce accepted milestones. Formal obligations and successful verification runs are indexed separately in the [assurance ledger](docs/assurance.toml). Annotation counts and proof-function counts do not establish semantic coverage.

Official duplicity 3.0.7 fixtures include full and incremental chains, multipart content, byte paths, metadata changes, deletions, symlinks, and encrypted/signed archives. Source-tree expectations were checked against official restoration. Native crypto and older-producer acceptance remain pending.

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
- [ ] Prove repository orchestration controller contracts and complete resource accounting.
- [x] Implement a local read-only source with anchored identity checks; test actual files and official three-date reconstruction.
- [x] Build read-only terminal history and snapshot browsing; exercise an official chain in a real terminal.
- [x] Add bounded terminal restore-selection previews and independently review the read-only flow.
- [x] Accept delta, manifest-token, byte-path, signatures and manifest-record components with exact contracts, fresh full verification, official tests and independent review.
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers.
- [ ] Complete verified snapshot indexing and native Linux restoration.
- [x] Independently review and adopt Linux staging/publication requirements before implementation, including indeterminate syscall outcomes.
- [x] Implement and independently review native Linux restoration; pass official three-date and fault-injection tests, enforce Duvet requirement coverage.
- [ ] Complete native OpenPGP adapters and their [capability profile](docs/openpgp-profile.md).
- [ ] Validate S3 with a live service and complete-reader consistency integration.
- [ ] Complete the browsing/restoration TUI.
- [ ] Pass the supported official-producer compatibility matrix.
- [x] **Create GitHub Actions for builds and formal methods:** install pinned tools, check generated parsers, run Verus, enforce Duvet coverage/snapshot gates, and run native/official-fixture tests on pull requests and pushes; retain reports and verification evidence as artifacts. See [workflow validation](docs/github-actions.md); the first hosted run remains pending.
- [x] Publish the Duvet HTML report from successful `main` runs through GitHub Pages, with its live URL in the deployment summary.
- [ ] Publish the exact supported capabilities, verification claims, and trusted boundaries.
