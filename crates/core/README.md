# duplicity-core

Native deterministic format parsing and reconstruction. Delta decoding/application has independently reviewed semantic contracts checked by Verus with `--no-cheating`: 535 obligations pass in focused verification. The whole reader verification gate remains incomplete.

| Component | Specification review | Implementation | Proof | Official compatibility tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Delta decoder and application | Reviewed grammar and bounds | Implemented; 16 native tests pass | Focused verification: 535 obligations, zero errors | Official 0.8.23, 2.1.3, 3.0.7 and 3.2.1 payloads pass, including multipart | Independent semantic review complete; whole-workspace gate pending |
| Manifest token decoding | Reviewed exact byte predicates | Implemented; three tests pass | Focused strict verification: 155 obligations, zero errors | Official manifests pass through this decoder | Independent semantic review complete; complete-core gate pending |
| Byte-path validation and comparison | Reviewed root/component grammar | Implemented; three tests pass | Exact normalization, rejection diagnostics and comparison: 10 focused strict obligations pass | Archive fixture integration pending | Independent semantic review complete; complete-core gate pending |
| Manifest framing and normalization | Reviewed fields, counts, hashes and restart semantics | Implemented; eight tests pass | Exact framing/digest helpers, last-declaration restart normalization and conservative selection checked; complete parser state pending | Plain manifests from all four producer versions pass | Software and helper contracts reviewed; complete formal acceptance pending |
| Object names and timestamps | Reviewed canonical, legacy and short-name grammar | Implemented; six tests pass; canonical framing uses Vest DSL | Calendar and classification refinement pending | All 84 recorded official object basenames pass | Native source reviewed; formal acceptance pending |
| Librsync signatures | Reviewed four magics and checksum widths | Implemented; four tests pass; header uses Vest DSL | Header/controller/cursor refinement: 54 focused strict obligations pass | Payloads from 18 official signature archives pass | Independent semantic review complete; complete-core gate pending |
| Tar containers and operations | Reviewed header, GNU/PAX and footer profiles | Implemented; thirteen tests pass; header uses Vest DSL | Exact header/arithmetic/footer: 37 focused strict obligations pass; PAX framing/keys and fractional time: 21 pass; controller pending | All 39 recorded plain gzip archives scan and classify | Native source reviewed; formal acceptance pending |
| Stored-object integrity and plain/gzip envelopes | Reviewed stored-byte hash and envelope requirements | Implemented; four tests pass | Digest and decompression libraries are trusted adapters; controller proof pending | Recorded official manifest hashes and gzip objects pass | Native source reviewed; complete formal acceptance pending |
| Multipart assembly | Reviewed ordinary 3.0.7 profile; queued-restart evidence pending | Implemented; six tests pass; all stored volumes checked before publication | Controller refinement pending | Official three-volume snapshot and two-volume delta match independent source hashes | Independent native review complete; restart and formal acceptance pending |
| Reconstruction and restoration | Requirements written; open questions remain | Pending | Pending | Pending | Pending |

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
- [x] Independently test multipart snapshot/delta source oracles and rejection after provisional output.
- [x] Acquire and review original queued-restart and empty-tail producer fixtures; specify strict compatibility requirements.
- [ ] Implement and independently test queued-restart boundary compatibility.
- [x] Check whole-manifest-module arithmetic/bounds safety: 61 strict obligations pass; complete grammar semantics remain separate.
- [ ] Prove complete manifest parser contextual semantics.
- [ ] Prove complete multipart controller refinement.
- [ ] Establish independent semantic proofs and adversarial tests.
- [ ] Validate actual official-producer archives and restoration.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml). Ordinary Cargo builds do not establish proof status; use `cargo xtask build`.

## Vest DSL and fallback record

The quoted-token parser and all byte predicates use the Vest DSL. Unquoted tokens require nonempty repetition that stops before an invalid byte and leaves the delimiter to the caller. In Vest 0.2.1, a trailing DSL `Vec` generates `RepeatTillEnd`, which requires complete input consumption. Adding a `Nothing` boundary generates `RepeatTillEnd` in the specification and `Star` in executable parsing; independent review caught that disagreement despite passing native tests. The implementation therefore uses the direct `Star(UnquotedByteFmt)` combinator and rejects zero consumed bytes. The generated byte predicate remains DSL-authored. This fallback affects PARSE-MAN-002 and PARSE-MAN-003 and retains their semantic proof and test obligations. Generated Rust is never patched to conceal the discrepancy.

## Envelope trust boundary

Stored-volume decoding checks SHA1 and MD5 declarations before decompression, rejects conflicting declarations, and checks every supported declared digest. Plain and concatenated gzip objects have explicit stored-byte, decoded-byte and member limits; gzip checksums and true stream termination are required even at an exact output limit. OpenPGP and PKCS7 remain explicitly unsupported at this checkpoint.

[RustCrypto SHA1](https://docs.rs/sha1/0.11.0/sha1/) and [MD5](https://docs.rs/md-5/0.11.0/md5/) primitives and [flate2](https://docs.rs/flate2/1.1.10/flate2/) decompression are trusted dependencies, not formally verified implementations. The reviewed flate2 gzip header reader caps filename/comment buffers at 65,535 bytes; gzip extra fields have a 16-bit length. Native tests cover corruption, truncation, trailing bytes, concatenated members, exact output bounds, reserved flags, header CRC rejection and maximal optional header fields. These tests do not establish complete reader verification.
