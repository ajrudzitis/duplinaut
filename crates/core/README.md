# duplicity-core

Native deterministic format parsing and reconstruction. Delta decoding/application has independently reviewed semantic contracts checked by Verus with `--no-cheating`: 535 obligations pass in focused verification. The whole reader verification gate remains incomplete.

| Component | Specification review | Implementation | Proof | Official compatibility tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Delta decoder and application | Reviewed grammar and bounds | Implemented; 16 native tests pass | Focused verification: 535 obligations, zero errors | Official 0.8.23, 2.1.3, 3.0.7 and 3.2.1 payloads pass, including multipart | Scoped component accepted by fresh full CI |
| Manifest token decoding | Reviewed exact byte predicates | Implemented; three token tests plus official manifests pass | Exact token contracts; 155 focused obligations and fresh full-core verification pass | Official manifests from four producer versions pass | Scoped component accepted by fresh full CI |
| Byte-path validation and comparison | Reviewed root/component grammar | Implemented; three kernel groups and official reader tests pass | Exact normalization, errors and comparison; 10 focused obligations and fresh full-core verification pass | Official three-date raw-byte path oracles pass | Scoped component accepted by fresh full CI |
| Manifest framing and normalization | Reviewed fields, counts, hashes and restart semantics | Implemented; eight tests pass | Exact framing/digest helpers, last-declaration restart normalization and conservative selection checked; complete contextual parser refinement checked; fresh full-core gate passes | Plain manifests from all four producer versions pass | Scoped manifest-record component accepted by fresh full CI |
| Object names and timestamps | Reviewed canonical, legacy and short-name grammar | Implemented; six tests pass; canonical framing uses Vest DSL | Exact bounded days-before-year Verus contract drafted; full timestamp grammar/controller refinement pending | All 84 recorded official object basenames pass | Native source reviewed; checkpoint awaits independent review |
| Librsync signatures | Reviewed four magics and checksum widths | Implemented; four tests pass; header uses Vest DSL | Exact wire/cursor/unknown-size contracts; 54 focused obligations and fresh full-core verification pass | Payloads from 21 official signature archives pass | Scoped component accepted; native Iterator boundary reviewed |
| Tar containers and operations | Reviewed header, GNU/PAX and footer profiles | Implemented; thirteen tests pass; header uses Vest DSL | Exact header/arithmetic/footer: 37 focused strict obligations pass; PAX framing/keys and fractional time: 21 pass; controller pending | All 48 recorded plain gzip archives scan and classify | Native source reviewed; formal acceptance pending |
| Stored-object integrity and envelopes | Reviewed stored-byte hash, plain/gzip and private OpenPGP reader paths | Plain/gzip plus digest-first OpenPGP reader integration; official encrypted chains and evidence tests | 10 scoped OpenPGP admission cursor/token obligations pass within 1,009 core obligations; this does not prove native wrapper call order or digest/crypto/tar facts; reader controller proof pending | Official original encrypted symmetric, public-key and signed snapshots pass three-date source hash oracles | Independent pipeline and scoped cursor proof review complete |
| Multipart assembly | Reviewed ordinary 3.0.7 profile; queued-restart requirements and fixtures reviewed | Implemented; six tests pass; all stored volumes checked before publication | Controller refinement pending | Official three-volume snapshot and two-volume delta match independent source hashes | Independent native review complete; footer-bearing restart and formal acceptance pending |
| Chain planning | Reviewed explicit anchor, exact target and conservative local uniqueness | Implemented; five tests pass | Controller refinement pending | Eighteen dates across six official plain naming profiles | Independent native review complete; no restorability claim |
| Snapshot reconstruction | Reviewed ordered state and preceding-set basis | Implemented; nine test groups pass | Exact scalar budget acceptance and file allowance: five strict obligations pass; state/counter/resource refinement pending | Three official 3.0.7 backup dates match byte paths, contents and metadata | Independent native review complete; formal acceptance pending |
| Restore selection | Reviewed byte paths, ancestors and supported types | Implemented; three regression groups pass | Controller refinement pending | Native byte-path and near-prefix tests pass | Independent native review complete; no filesystem publication |
| Filesystem restoration | Independently reviewed Linux staging/publication profile | Native Linux implementation | Controller refinement pending; kernel/syscall boundary trusted | Eleven fault tests and two official integration groups pass on ext-family filesystem | Independent native review complete |

- [x] Create the library and verification-enabled Cargo configuration.
- [x] Implement a reviewed Vest DSL delta component and bounded fixed-basis application.
- [x] Test every literal opcode, all sixteen copy encodings, truncation, reserved opcodes, exact termination, and output limits.
- [x] Independently test byte order, all command truncation prefixes, overflow traps, and immutable basis semantics.
- [x] Restore official 3.0.7 extracted incremental and metadata-only delta payloads against independently known source contents.
- [x] Apply a large delta assembled from two official volumes and compare its result with the independent source-tree hash oracle.
- [x] Prove delta decoding and fixed-basis application against an independent wire model, including rejection and output bounds.
- [x] Test extracted deltas from five historical producer profiles against independent source contents.
- [x] Implement bounded manifest records, restart normalization, digest declarations and counted file lists; test official manifests.
- [x] Prove byte-component comparison against an independent lexicographic model.
- [x] Prove exact manifest token consumption, decoded bytes, malformed-input rejection and encoded-size limits against an independent model.
- [x] Parse object names with literal prefixes, bounded numerals and Gregorian timestamps; test every supported calendar day and original producer basename.
- [x] Read signature headers and borrowed checksum records; validate all algorithms and official signature payloads without inferring original file size.
- [x] Prove signature header/controller semantics, exact checksum exposure and cursor progression; review the thin native iterator adapter separately.
- [x] Prove exact manifest trimming, word/token boundaries, ASCII matching and decimal overflow rejection.
- [x] Prove exact manifest digest recognition, supported widths, hexadecimal validation and normalized output.
- [x] Scan native tar containers and classify duplicity operations; test GNU/PAX metadata and explicit footer profiles against official archives.
- [x] Integrate independently checked exact tar header, numeric, checksum, extent and envelope-footer kernels into runtime parsing.
- [x] Integrate checked PAX counted-record/key and signed fractional-time kernels; test exact precision and signed extrema.
- [x] Prove exact path normalization and rejection diagnostics against a functional byte grammar.
- [x] Validate supported manifest digests against stored volume bytes before decoding; enforce plain/gzip size and member limits.
- [x] Prove restart normalization preserves exactly the final-number-capped last declarations in sorted unique order.
- [x] Prove exact volume selection and inclusion of every selected descendant in a declared inclusive interval.
- [x] Prove exact path/error adaptation, optional positive block boundaries and volume record completion diagnostics.
- [x] Assemble ordinary 3.0.7 plain/gzip multipart operations with verified stored digests, complete containers, explicit bounds and sticky failure.
- [x] Add a private OpenPGP admission cursor and stage-token gate; prove normalized ordinal/finality, stage transitions and commit advancement; retain factual digest/decrypt/tar call order as trusted wrapper code.
- [x] Make the private native stored-digest gate the sole Rust-visible producer of `DigestPermit`; source review confirms it calls the stored-byte checker before the proved cursor transition. The wrapper does not formally prove call order or SHA1/MD5 correctness.
- [x] Exercise a valid authenticated OpenPGP object whose literal fails tar validation; reject it without advancing the admission cursor.
- [x] Independently test multipart snapshot/delta source oracles and rejection after provisional output.
- [x] Acquire and review original queued-restart and empty-tail producer fixtures; specify strict compatibility requirements.
- [x] Implement and independently test source-derived queued-restart raw/gzip boundary compatibility and full-scan selection.
- [x] Check whole-manifest-module arithmetic/bounds safety and exact contextual grammar refinement: 79 fresh strict obligations pass.
- [x] Prove complete manifest parser contextual success and error semantics against an independent model.
- [x] Prove exact digest duplicate/conflict detection; preserve first-conflict rejection and bounded append behavior.
- [ ] Prove complete multipart controller refinement.
- [x] Exercise the real multipart wrapper's post-digest and post-native-decode cancellation checkpoints using private, thread-local `cfg(test)` observation points and official ciphertext. The first test proves decode is not reached using wrong credentials; the second proves tar validation/commit are not reached using valid credentials. These hooks do not exist in release builds. Public-reader snapshot cancellation at those precise internal boundaries remains pending.
- [ ] Establish independent semantic proofs and adversarial tests.
- [ ] Validate actual official-producer archives and restoration.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml). Ordinary Cargo builds do not establish proof status; use `cargo xtask build`.

## Vest DSL and fallback record

The quoted-token parser and all byte predicates use the Vest DSL. Unquoted tokens require nonempty repetition that stops before an invalid byte and leaves the delimiter to the caller. In Vest 0.2.1, a trailing DSL `Vec` generates `RepeatTillEnd`, which requires complete input consumption. Adding a `Nothing` boundary generates `RepeatTillEnd` in the specification and `Star` in executable parsing; independent review caught that disagreement despite passing native tests. The implementation therefore uses the direct `Star(UnquotedByteFmt)` combinator and rejects zero consumed bytes. The generated byte predicate remains DSL-authored. This fallback affects PARSE-MAN-002 and PARSE-MAN-003 and retains their semantic proof and test obligations. Generated Rust is never patched to conceal the discrepancy.

## Envelope trust boundary

Stored-volume decoding checks SHA1 and MD5 declarations before decompression, rejects conflicting declarations, and checks every supported declared digest. Plain and concatenated gzip objects have explicit stored-byte, decoded-byte and member limits; gzip checksums and true stream termination are required even at an exact output limit. OpenPGP is available only through the reader's borrowed crypto context and private digest-first assembly path; the public plain/gzip assembler still rejects OpenPGP plaintext assertions. PKCS7 remains unsupported.

[RustCrypto SHA1](https://docs.rs/sha1/0.11.0/sha1/) and [MD5](https://docs.rs/md-5/0.11.0/md5/) primitives and [flate2](https://docs.rs/flate2/1.1.10/flate2/) decompression are trusted dependencies, not formally verified implementations. The reviewed flate2 gzip header reader caps filename/comment buffers at 65,535 bytes; gzip extra fields have a 16-bit length. Native tests cover corruption, truncation, trailing bytes, concatenated members, exact output bounds, reserved flags, header CRC rejection and maximal optional header fields. These tests do not establish complete reader verification.

- [x] Reconstruct bounded immutable snapshot states and compare three official backup dates with source-tree oracles.
- [ ] Prove snapshot-state controller and live resource model.

- [x] Independently review state semantics; test deleted delta bases, near-prefix subtree isolation and final-validation atomicity.
- [x] Test exact successful payload, file, entry and live payload limits, including empty states and one-unit rejection boundaries.
- [x] Add exact success/rejection threshold tests for state metadata and live metadata budgets, including retained archive storage, path scratch and the final ancestor stack.

- [x] Plan metadata-only contiguous chains with explicit anchors and exact dates; reject missing links and ambiguous branches.
- [x] Integrate original object identities, manifest/volume validation and snapshot states in a bounded plain/gzip reader.
- [x] Prove chain-controller deterministic selection/errors and borrowed provenance: 17 fresh strict obligations pass; completed-name predicate passes separately.
- [ ] Prove repository-reader refinement and thin chain iterator adaptation; complete same-source core acceptance.

- [x] Select immutable restore entries, required source ancestors and exact byte paths; reject selected unsupported types.
- [x] Implement and independently review native Linux filesystem publication; see [limits and test evidence](../../docs/restore-linux-implementation.md).
- [ ] Prove restore-selection and filesystem-controller refinement.

The delta and manifest-token component gates passed complete local `cargo xtask ci` at commit `9af428f`, source SHA-256 `611a668f654385575f187a092bf0c8b14eeec974cc2528f4dbae1919140a574d`: 999 strict core obligations, zero errors, required official/native tests, Duvet coverage and all tooling gates. The receipt accepts tooling, manifest-token and delta-wire only. Other component/controller gates remain pending.

Byte-path, signatures and manifest-records gates also passed fresh complete CI at commit `181b83e`, source SHA-256 `04f3d99a40b413151bb25c916de61dc9750204886c957acc00056d6e16f50cc3`, with 999 strict core obligations and zero errors. Required official tests and per-requirement Duvet coverage passed. Accepted gates are tooling plus those three components, delta-wire and manifest-token. Allocation models, queued-restart controllers and complete-reader proofs remain pending.
