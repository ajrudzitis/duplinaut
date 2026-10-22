# Specification and assurance progress

[Parsing requirements](parsing-requirements.md) is the canonical normative document. Its immutable heading IDs are Duvet targets; reference provenance distinguishes public format documentation, official-source fallback, and reader policy. It was drafted and independently reviewed before implementation. Unresolved format questions remain implementation holds.

## Specification tasks

- [x] Write RFC 2119/8174 conventions and atomic requirement headings.
- [x] Document known object, manifest, operation-prefix, signature, and complete librsync delta grammars.
- [x] Record the exact inspected duplicity source bytes.
- [x] Define malformed-input behavior, restoration policies, verification obligations, and official-producer scenarios.
- [x] Document format uncertainties and affected implementation holds.
- [x] Independently review and accept the initial requirements with unresolved formats held.
- [x] Establish Duvet extraction/annotation conventions and the reviewed baseline.
- [x] Pin and independently review immutable librsync opcode/magic source identities.
- [x] Pin producer/toolchain provenance for the recorded official fixture matrix.
- [ ] Resolve each open question before implementing its affected component.
- [x] Record per-component specification approval, implementation, proof, tests, and review status in crate READMEs, including incomplete obligations.

## Verification goals

Native plain/gzip reading now covers reviewed parsers, metadata-only chain planning, original-object reconstruction, local/S3 sources, and read-only terminal browsing and selection previews. Focused proof checkpoints are recorded separately from successful native tests and independent source review. A fresh full-workspace verification run now passes the current contracts; remaining controller refinement and complete component/reader acceptance are still pending. No row claims a verified complete reader or complete filesystem-controller proof.

| Goal | Correctness claim | Implementation | Proof | Compatibility tests | Independent review |
| --- | --- | --- | --- | --- | --- |
| VG-PARSE | Input grammar refinement, bounds, arithmetic, termination, panic freedom | Delta/token/path, names, manifests, signatures and tar implemented | Exact whole-manifest contextual success/error refinement: 79 strict obligations pass; accepted token/path/signature/tar helper checkpoints; name and tar-controller refinement and complete component acceptance pending | Official historical names, manifests, signature payloads and plain/gzip tar archives pass; broader profiles pending | Native parser and focused semantic contracts reviewed; complete formal acceptance pending |
| VG-DELTA | Complete opcode decoding and exact literal/basis-slice reconstruction | Implemented | Exact contracts accepted: 535 focused obligations and fresh full-core verification pass | Official 0.8.23, 2.1.3, 3.0.7 and 3.2.1 payloads pass | Independent semantic review complete; scoped delta component accepted |
| VG-ASSEMBLY | Restart records, conservative selection, complete ordered multipart content | Ordinary and reviewed no-footer queued-restart 3.0.7 plain/gzip assembly implemented | Restart normalization and conservative volume selection checked; multipart controller/resource refinement pending | Official snapshot/delta, queued-restart/empty-tail and adversarial assembly tests pass; footer-bearing replay pending | Independent native review complete; formal acceptance pending |
| VG-STATE | Unique contiguous chains and correct path-state transitions | Ordered state reconstruction, explicit metadata chain planning and identity-bound repository reading implemented | Exact chain selection/errors and original descriptor provenance: 17 strict obligations pass; thin iterator adapter, state/resource and repository-controller refinement pending | All three official 3.0.7 dates match source/restore oracles; historical metadata chains and adversarial state/reader budget cases pass | Independent native state, chain and repository review complete; full formal acceptance pending |
| VG-RESTORE | Valid selected paths, destination containment, supported metadata fidelity | Read-only byte-path selection and required-ancestor plans implemented; reviewed native Linux publication implemented | Selection/filesystem-controller refinement pending; filesystem operations remain a separate trusted boundary | Native byte-path/near-prefix/ancestor tests and official snapshot selection pass; terminal preview smoke passes; eleven fault tests and two official filesystem integration groups pass on ext-family filesystem | Independent selection/preview and native filesystem reviews complete; formal acceptance pending |
| VG-INTEGRATION | Integrity/decryption/source consistency adapter contracts | Bounded source API, anchored local adapter, S3, stored-byte digest/plain/gzip and private OpenPGP reader composition implemented | Adapter contracts reviewed; crypto primitives/backend trusted; repository/local-controller and aggregate reader proofs pending | Exact-size/EOF source tests, local file/change/interruption tests, eleven actual-SDK loopback S3 tests, official plain and encrypted object reconstruction pass; terminal escaping/CLI and manual PTY smoke pass | Source/local/S3, native crypto adapter and OpenPGP reader composition reviewed; live S3, automated terminal interaction and complete-reader proof pending |

The fresh `cargo xtask verify` checkpoint at commit `f5b0042` verified 2,047 vstd, 3,183 Vest and 994 core obligations with zero errors, using an isolated empty target and the full-summary gate. Its receipt records source SHA-256 `6eb290bb9bcf7da934dbccd9b3bad270642f419c131b7753f02bb75bf173faab`, default features, locked dependencies and project `--no-cheating`. It records tests as **not run** and accepts only the tooling milestone. This is successful verification of the contracts present at that source identity; it does not close missing semantic/controller obligations, enable pending component milestones, or establish full CI/complete-reader acceptance. Later source changes require a new acceptance run.

Detailed boundaries and residual obligations appear in [chain planning](chain-planning.md), [reader orchestration](reader-orchestration.md), [local source](local-source.md), [restore selection](restore-plan.md), and the [core](../crates/core/README.md), [S3](../crates/s3/README.md) and [TUI](../crates/tui/README.md) records. Budget tests establish specific retained-capacity/work/payload limits; they do not establish a global heap or cancellation-latency proof. Manual terminal smoke runs do not establish automated interaction or live-service acceptance.

Requirement mappings and scenarios appear in the specification. Completion records include exact source identity, tool versions, target/features, proved contracts, trusted assumptions, official fixture provenance, and successful check results. Neither annotation counts nor proof-function counts measure semantic coverage.

## Traceability and review rules

Use Duvet links from authored `.vest` and Rust source to the exact normative text beneath a requirement heading. Executable tests carry test links. Record formal proof obligations and successful Verus runs separately by requirement ID; Duvet has no documented native formal-proof category. HTML/JSON reports provide navigation; a reviewed checked-in snapshot detects coverage changes. Snapshot agreement does not prove requirement completion.

The specification author owns normative changes, the software agent owns implementation, and the verification agent owns independent assurance evidence. Generated parser semantics are reviewed against requirements because the Vest frontend is unverified. Shared executable/proof files use sequential handoffs. A weakened contract or added trusted assumption requires explicit review before it changes completion status.

Update these records with each accepted component. An unresolved question is retained with its provenance until its decision, affected requirements, proof consequences, and test expectations have all been recorded.

The complete local `cargo xtask ci` gate also passed at commit `cebb18a`, source SHA-256 `e8135edea855491bb7a0f5d1e74325add60cf93bee863e7d9fbcb70293c77720`: fresh formal verification, generated-parser identity, Duvet coverage/snapshot enforcement, formatting, Clippy and workspace/declared milestone tests. Its receipt remains tooling-only; missing controller contracts and other pending milestones remain open. GitHub-hosted execution is still pending.

The first scoped parser acceptance includes delta-wire and manifest-token. Their enabled gates passed complete local CI at commit `9af428f`, source SHA-256 `611a668f654385575f187a092bf0c8b14eeec974cc2528f4dbae1919140a574d`, with 999 strict core obligations and zero errors plus declared official/native tests and Duvet coverage. This does not enable the other pending parser or complete-reader milestones.

The OpenPGP reader composition was independently accepted at `f12658d`; its exact adopted source completed the full unfiltered `cargo xtask ci` gate at root commit `51aff1b`, source SHA-256 `ec346a3cf38be74ecee0d291450ca0dd9edbbebac229036b23c751f4e49f7116`. Duvet matched 138 requirements and 462 references, Verus/Vest reported 2,047 vstd, 3,183 Vest and 999 core obligations with zero errors, and formatting, strict Clippy and all workspace/doc tests passed. This accepts the scoped crypto-reader behavior and its native integration tests; it does not prove the reader controller, global resource accounting, or the remaining explicitly listed adversarial and compatibility cases.
