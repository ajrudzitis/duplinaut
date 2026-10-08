# Specification and assurance progress

[Parsing requirements](parsing-requirements.md) is the canonical normative document. Its immutable heading IDs are Duvet targets; reference provenance distinguishes public format documentation, official-source fallback, and reader policy. It was drafted and independently reviewed before implementation. Unresolved format questions remain implementation holds.

## Specification tasks

- [x] Write RFC 2119/8174 conventions and atomic requirement headings.
- [x] Document known object, manifest, operation-prefix, signature, and complete librsync delta grammars.
- [x] Record the exact inspected duplicity source bytes.
- [x] Define malformed-input behavior, restoration policies, verification obligations, and official-producer scenarios.
- [x] Document format uncertainties and affected implementation holds.
- [x] Independently review and accept the initial requirements with unresolved formats held.
- [ ] Establish Duvet extraction/annotation conventions and the reviewed baseline.
- [x] Pin and independently review immutable librsync opcode/magic source identities.
- [ ] Pin producer/toolchain provenance.
- [ ] Resolve each open question before implementing its affected component.
- [ ] Record per-component specification approval, implementation, proof, tests, and review in crate READMEs.

## Verification goals

Delta decoding and application, manifest token decoding, and byte-path validation now have native implementations and passing tests. Formal proofs and component acceptance are in progress. An initial specification is available for every goal; goals with open questions cannot be accepted until those questions are resolved.

| Goal | Correctness claim | Implementation | Proof | Compatibility tests | Independent review |
| --- | --- | --- | --- | --- | --- |
| VG-PARSE | Input grammar refinement, bounds, arithmetic, termination, panic freedom | Delta/token/path implemented; remaining parsers pending | In progress | Delta official 3.0.7 payload tests pass; remaining pending | In progress |
| VG-DELTA | Complete opcode decoding and exact literal/basis-slice reconstruction | Implemented | In progress | Official 3.0.7 incremental and metadata-only payloads pass | Adversarial tests pass; formal acceptance pending |
| VG-ASSEMBLY | Restart records, conservative selection, complete ordered multipart content | Pending | Pending | Pending | Pending |
| VG-STATE | Unique contiguous chains and correct path-state transitions | Pending | Pending | Pending | Pending |
| VG-RESTORE | Valid selected paths, destination containment, supported metadata fidelity | Pending | Pending; trusted filesystem boundary | Pending | Pending |
| VG-INTEGRATION | Integrity/decryption/source consistency adapter contracts | Pending | Contract review; primitives trusted | Pending | Pending |

Requirement mappings and scenarios appear in the specification. Completion records include exact source identity, tool versions, target/features, proved contracts, trusted assumptions, official fixture provenance, and successful check results. Neither annotation counts nor proof-function counts measure semantic coverage.

## Traceability and review rules

Use Duvet links from authored `.vest` and Rust source to the exact normative text beneath a requirement heading. Executable tests carry test links. Record formal proof obligations and successful Verus runs separately by requirement ID; Duvet has no documented native formal-proof category. HTML/JSON reports provide navigation; a reviewed checked-in snapshot detects coverage changes. Snapshot agreement does not prove requirement completion.

The specification author owns normative changes, the software agent owns implementation, and the verification agent owns independent assurance evidence. Generated parser semantics are reviewed against requirements because the Vest frontend is unverified. Shared executable/proof files use sequential handoffs. A weakened contract or added trusted assumption requires explicit review before it changes completion status.

Update these records with each accepted component. An unresolved question is retained with its provenance until its decision, affected requirements, proof consequences, and test expectations have all been recorded.
