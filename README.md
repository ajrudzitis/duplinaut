# Duplicity backup reader in Rust

A planned native Rust reader for browsing duplicity backup history and restoring selected entries from local archives and S3. Backup creation and bucket mutation are outside the project.

The first deliverable was the [parsing requirements](docs/parsing-requirements.md), written and independently reviewed before implementation using RFC requirement language. The repository now contains the Cargo workspace, assurance tooling, and initial native format components. Complete browsing, reconstruction, restoration, S3 access, and the TUI remain pending. Proofs and compatibility evidence are tracked separately from executable code.

## Development milestones

Tasks are checked only when the stated evidence passes. Specification, implementation, proof, compatibility testing, and review are separate completion states; a Duvet citation alone does not establish any of them.

- [x] Draft the parsing requirements before attempting implementation.
- [x] Record primary-document and official-source provenance and unresolved format questions.
- [x] Define verification goals and milestone completion rules.
- [x] Independently review the initial specification while retaining unresolved format holds.
- [x] Establish Duvet extraction and a reviewed snapshot baseline.
- [x] Pin compatible Rust, Verus, Vest, Duvet, and solver versions and validate installed tools.
- [ ] Complete the official-producer environment matrix.
- [x] Establish and independently review xtask generation, traceability, verified-build, test, and CI gates.
- [ ] Complete one small parser component with reviewed requirements, production implementation, proofs, official fixture tests, and independent acceptance.
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers with the same evidence.
- [ ] Complete snapshot indexing and native Linux restoration with verified reconstruction and filesystem integration tests.
- [ ] Complete the read-only S3 adapter and its source consistency tests.
- [ ] Complete the browsing/restoration TUI and progress/cancellation behavior.
- [ ] Pass the supported official-producer compatibility matrix and publish the precise capability/trust profile.

## Workspace and development commands

The product crates are `duplicity-core`, `duplicity-s3`, and `duplicity-tui`; an auxiliary `xtask` crate manages development checks. The core owns deterministic parsing and reconstruction. S3 and TUI crates currently expose scaffold status; AWS dependencies remain outside the core. Parsers are authored in Vest DSL with reviewed generated Rust used at runtime.

The commands below are implemented. [Tool setup](tools/README.md) documents bootstrapping and executable overrides. Twelve tooling failure tests and independent review passed. Default builds run verification and fail on incomplete proofs or unreviewed trust; parser milestone acceptance remains separate.

| Command | Acceptance behavior |
| --- | --- |
| `cargo xtask build` | Generate/check parsers, run required verification, then compile the same production implementations |
| `cargo xtask verify` | Run Verus obligations and audit trusted assumptions/exemptions |
| `cargo xtask trace` | Run Duvet and produce requirement/evidence reports |
| `cargo xtask trace-update` | Explicitly update the Duvet baseline for review and commit |
| `cargo xtask test` | Run native tests and checked-in official-produced fixtures |
| `cargo xtask producer` | Fail explicitly until official-fixture regeneration is established |
| `cargo xtask ci` | Run the complete milestone acceptance gate |

Ordinary Cargo builds are a development convenience once code exists; they do not establish verification evidence. Required tool absence, proof failure, timeout, stale generation, missing milestone evidence, and unreviewed assumptions fail the acceptance gate.

## Verification progress

The first delta component has executable tests and official-produced delta examples; its independent semantic proof remains pending. Tooling tests exercise actual Duvet coverage/snapshot failures, stale Vest generation, incomplete proof references, missing tools, unreviewed trust, and source identity. [Documentation progress](docs/README.md), per-crate READMEs, and the [machine ledger](docs/assurance.toml) distinguish those results from proved and independently accepted components. Structural declaration checks and Duvet links do not establish semantic correctness.

A dedicated specification agent owns requirements and provenance. A software agent implements reviewed contracts. A separate verification agent owns proof obligations, adversarial tests, and acceptance findings. The coordinator reviews handoffs, trusted boundaries, and requirement changes. Changes to contracts or assumptions return through specification review.

[Duvet](https://github.com/awslabs/duvet) provides requirement traceability; [Verus](https://github.com/verus-lang/verus) and [Vest](https://github.com/secure-foundations/vest) provide the intended formal verification workflow. Cryptography, compression, hashing, networking, and filesystem adapters form explicitly documented trusted boundaries. A proof against the wrong specification is not considered completion.
