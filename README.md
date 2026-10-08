# Duplicity backup reader in Rust

A planned native Rust reader for browsing duplicity backup history and restoring selected entries from local archives and S3. Backup creation and bucket mutation are outside the project.

The first deliverable is the [parsing requirements](docs/parsing-requirements.md), written before implementation using RFC requirement language. They include source provenance, precise known formats, verification goals, and unresolved questions that hold affected implementation work. This repository currently contains the initial specification and development records; no reader, verified parser, or runnable TUI is complete.

## Development milestones

Tasks are checked only when the stated evidence passes. Specification, implementation, proof, compatibility testing, and review are separate completion states; a Duvet citation alone does not establish any of them.

- [x] Draft the parsing requirements before attempting implementation.
- [x] Record primary-document and official-source provenance and unresolved format questions.
- [x] Define verification goals and milestone completion rules.
- [x] Independently review the initial specification while retaining unresolved format holds.
- [ ] Establish its Duvet extraction and reviewed snapshot baseline.
- [ ] Pin compatible Rust, Verus, Vest, Duvet, solver, and official-producer environments.
- [ ] Establish xtask generation, traceability, verified-build, test, and CI gates.
- [ ] Complete one small parser component with reviewed requirements, production implementation, proofs, official fixture tests, and independent acceptance.
- [ ] Complete filename, manifest, tar, signature, delta, and multipart parsers with the same evidence.
- [ ] Complete snapshot indexing and native Linux restoration with verified reconstruction and filesystem integration tests.
- [ ] Complete the read-only S3 adapter and its source consistency tests.
- [ ] Complete the browsing/restoration TUI and progress/cancellation behavior.
- [ ] Pass the supported official-producer compatibility matrix and publish the precise capability/trust profile.

## Intended workspace and commands

The planned product crates are `duplicity-core`, `duplicity-s3`, and `duplicity-tui`; an auxiliary `xtask` crate manages development checks. The core owns deterministic parsing and reconstruction, while AWS dependencies remain in the S3 adapter. Vest DSL definitions are preferred; reviewed direct combinators are the fallback.

The following commands are intended interfaces and are not implemented yet:

| Command | Intended acceptance behavior |
| --- | --- |
| `cargo xtask build` | Generate/check parsers, run required verification, then compile the same production implementations |
| `cargo xtask verify` | Run Verus obligations and audit trusted assumptions/exemptions |
| `cargo xtask trace` | Run Duvet and produce requirement/evidence reports |
| `cargo xtask test` | Run native tests and checked-in official-produced fixtures |
| `cargo xtask producer` | Regenerate official archives and compare source, native restore, and official restore |
| `cargo xtask ci` | Run the complete milestone acceptance gate |

Ordinary Cargo builds are a development convenience once code exists; they do not establish verification evidence. Required tool absence, proof failure, timeout, stale generation, missing milestone evidence, and unreviewed assumptions fail the acceptance gate.

## Verification progress

All correctness goals are currently **planned**, with no proofs or integration test passes recorded. [Documentation progress](docs/README.md) maps the goals to requirement IDs and review status. Per-crate READMEs will record each component's implementation, proof, test, and acceptance evidence when those crates are introduced.

A dedicated specification agent owns requirements and provenance. A software agent implements reviewed contracts. A separate verification agent owns proof obligations, adversarial tests, and acceptance findings. The coordinator reviews handoffs, trusted boundaries, and requirement changes. Changes to contracts or assumptions return through specification review.

[Duvet](https://github.com/awslabs/duvet) provides requirement traceability; [Verus](https://github.com/verus-lang/verus) and [Vest](https://github.com/secure-foundations/vest) provide the intended formal verification workflow. Cryptography, compression, hashing, networking, and filesystem adapters form explicitly documented trusted boundaries. A proof against the wrong specification is not considered completion.
