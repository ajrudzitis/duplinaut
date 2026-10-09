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
| Delta decoding and application | Native and adversarial tests; official small and multipart payloads; focused Verus run passes | Dependency verification and final component acceptance |
| Manifest tokens and byte paths | Native tests pass; exact grammar reviewed | Semantic proofs and complete manifest parsing |
| Storage interface | Exact-size and cancellation tests pass | S3 adapter and source-consistency integration |
| Assurance tooling | Pinned tools, Duvet baseline, executable failure tests, independent review | Expand gates as components are accepted |
| Repository browser and restore | Requirements and official source-tree oracles | Implementation, proofs, and compatibility matrix |
| Terminal interface | Workspace scaffold | Browsing, restore selection, progress, and cancellation |

The [parsing requirements](docs/parsing-requirements.md) were written in RFC 2119/8174 language and independently reviewed **before implementation**. Public documentation leads format research; immutable official-source references and producer evidence resolve ambiguities.

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

`cargo xtask generate` updates generated parsers for review. `cargo xtask trace-update` explicitly updates the Duvet baseline for review and commit. Official fixture regeneration is documented in [the producer guide](tests/producer/README.md); `cargo xtask producer` currently fails explicitly until that workflow is wired into xtask.

## Backlog and milestones

Tasks are checked only when their stated evidence passes. Each completed step gets a local Git commit, and useful working checkpoints record unfinished assurance work explicitly.

- [x] Write and independently review parsing requirements before implementation.
- [x] Record immutable format provenance and explicit implementation holds.
- [x] Pin Rust, Verus, Vest, Duvet, and solver versions.
- [x] Establish reviewed Duvet extraction, snapshots, and milestone coverage gates.
- [x] Establish independently reviewed xtask generation and verification tooling.
- [x] Produce official 3.0.7 fixtures with independent source/restore oracles.
- [ ] Accept the first parser component with implementation, proofs, official tests, and independent review.
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers.
- [ ] Complete verified snapshot indexing and native Linux restoration.
- [ ] Complete native OpenPGP adapters and their [capability profile](docs/openpgp-profile.md).
- [ ] Complete the read-only S3 adapter and consistency/cancellation tests.
- [ ] Complete the browsing/restoration TUI.
- [ ] Pass the supported official-producer compatibility matrix.
- [ ] **Create GitHub Actions for builds and formal methods:** install pinned tools, check generated parsers, run Verus, enforce Duvet coverage/snapshot gates, and run native/official-fixture tests on pull requests and pushes; retain reports and verification evidence as artifacts.
- [ ] Publish the exact supported capabilities, verification claims, and trusted boundaries.
