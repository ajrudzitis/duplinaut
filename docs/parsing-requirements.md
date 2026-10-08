# Duplicity reader parsing requirements

Status: initial specification reviewed by the coordinator and independent verification agent. Written before parser implementation. Unresolved formats remain held. No requirement currently has implementation, proof, or compatibility-test completion evidence.

This document defines the intended native Rust reader for browsing and restoring duplicity backups. It is a project specification, not a claim that duplicity implements every validation policy below. Requirements with unresolved dependencies are provisional and cannot authorize implementation of the affected behavior until those dependencies are resolved.

## Normative conventions

Uppercase requirement keywords have the meanings in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174). Lowercase usage is ordinary prose. Each normative requirement has its own immutable heading ID. Labels, provenance, explanations, grammars, and scenario descriptions are informative unless referenced by a normative requirement.

In provenance, **DOC** means public format documentation, **SRC** means official source consulted where documentation is incomplete, and **POLICY** means a deliberate reader decision. A source-derived acceptance rule is not automatically evidence of correctness or compatibility across releases.

## Sources and compatibility profile

Public documentation is the first reference:

| Reference | Primary source | Used for |
| --- | --- | --- |
| DOC-DUP | [Duplicity manual, currently 3.2.1](https://duplicity.gitlab.io/stable/duplicity.1.html) | Backup model, signature-based listing, native metadata, encryption and prefix options |
| DOC-NAME | [Official filename documentation](https://gitlab.com/duplicity/duplicity/-/raw/dev/docs/file_naming.rst) | Long/short object names, partial markers, suffixes |
| DOC-RSYNC | [Librsync file formats](https://librsync.github.io/page_formats.html) | Integer byte order, signature and delta structures |
| DOC-VEST | [Vest project documentation](https://github.com/secure-foundations/vest) | DSL, combinators, verification guarantees and frontend trust |
| DOC-DUVET | [Duvet specifications](https://awslabs.github.io/duvet/specifications.html), [annotations](https://awslabs.github.io/duvet/annotations.html), [reports](https://awslabs.github.io/duvet/reports.html) | Markdown requirement extraction and evidence links |

Official source fallback inspected on 2026-10-07: duplicity **3.0.7**, available through Nix at `/nix/store/5j4mk0k17j37nh8a7s2pcr5z0vblx73r-duplicity-3.0.7/lib/python3.13/site-packages/duplicity/`. The hashes below identify the inspected bytes; moving upstream links are explanatory and do not replace these identities.

| Reference | Inspected file and relevant functions | SHA-256 |
| --- | --- | --- |
| SRC-NAME | [file_naming.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/file_naming.py): `prepare_regex`, `parse`, `get`, `from_base36` | `03694be151b0d8761d66989b29d00942b8437a979ec61e78bbbae1bdcc7c8229` |
| SRC-TIME | [dup_time.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/dup_time.py): `timetostring`, `stringtotime` | `66ef1ffbcc0afd408735e6005635d4299c8db4226c991831235e73d685e9f03b` |
| SRC-MAN | [manifest.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/manifest.py): `Manifest.from_string`, `VolumeInfo`, `Quote`, `Unquote` | `8ba9ee049ddfe465645da70ea4d27c12f1d16bd8f9bdaec9d5a282a78d08d8eb` |
| SRC-DIFF | [diffdir.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/diffdir.py): `sigtar2path_iter`, `TarBlockIter`, `get_footer` | `434d30652e87eaa53bf4263b91947c4442fe0e49b3c219f5e5ea7bb2888e0d9f` |
| SRC-PATCH | [patchdir.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/patchdir.py): `get_index_from_tarinfo`, `Multivol_Filelike`, `patch_seq2ropath` | `9cd4bbb8f27c1a7508c73903a448a8018e7202b344880945b3111d75619f7439` |
| SRC-HASH | [gpg.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/gpg.py): `get_hash` | `e39d6fbc645a08364fe9a9207f8e9bb3085f99ded8738a2c467630e7adaf9ddb` |
| SRC-TAR | [dup_tarfile.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/dup_tarfile.py) | `b1e7c58f707bfb71a45fcf797cd6cf5729b5600ca1a9a7e70a0ec071b987e529` |

SRC-RSYNC references the official [opcode table](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/prototab.c) and [magic numbers](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/librsync.h), independently fetched and reviewed on 2026-10-07 at commit `271744de428c0177e993f95c045d70b8cf2558f2`. Their SHA-256 values are `2c7500cd1d485ef55720707607b7876fdc00a42677f53aab465b114f05eaa972` and `4974e6e51fabc0de94b03e7b640cdd21e4979a41bdf3e7bd975bc029aa655762`, respectively. The public format page omits the implicit literal opcodes, so source fallback is necessary.

Target compatibility versions are duplicity 0.8.23, 2.1.3, 3.0.7, and 3.2.1. These are intended test targets, not currently supported or tested versions. Runtime parsing, decryption, decompression, indexing, and restoration are native Rust operations. Official duplicity, GnuPG, and rdiff are producers and reference implementations for tests.

Initial scope includes Linux restoration of regular files, directories, symlinks, modification times, and ordinary permissions. Ownership, special-file creation, ACL/xattr fidelity, hard-link preservation, backup creation, bucket mutation, incomplete-chain repair, and CMS/GPGSM `.p7m` decoding are excluded. Unsupported entries and sets remain inspectable where enough metadata is available.

## General parser behavior

### PARSE-GEN-001

A parser MUST return a structured error for malformed or truncated required input.

Provenance: POLICY. Errors distinguish malformed, truncated, unsupported, integrity-failed, and resource-limited input and identify the object and offset when available; secret data is excluded from error text.

### PARSE-GEN-002

A successful parser MUST report a consumed input range wholly contained within the supplied input.

Provenance: POLICY. Whole-object consumers additionally enforce the relevant format's termination and padding rules.

### PARSE-GEN-003

A parser MUST validate arithmetic and platform-size conversions before using derived values for indexing, allocation, or I/O.

Provenance: POLICY. Includes lengths, decimal/base36 numbers, offsets, padding, and accumulated reconstructed size.

### PARSE-GEN-004

A parser MUST terminate without panicking on every finite input within its configured resource limits.

Provenance: POLICY. Environmental I/O failures are errors; process exhaustion outside those limits is not a proved property.

### PARSE-GEN-005

The reader MUST enforce explicit configurable bounds on decoded bytes, entry counts, path lengths, nesting, metadata storage, reconstructed bytes, and temporary storage.

Provenance: POLICY. Limit failures are reported without publishing a partial restoration. Concrete defaults are unresolved in OPEN-LIMITS.

### PARSE-GEN-006

The core reader MUST perform its runtime operations without invoking duplicity, GnuPG, or librsync command-line tools.

Provenance: POLICY. Trusted Rust libraries for cryptography, compression, hashing, and I/O are identified separately from verified algorithms.

### PARSE-PATH-001

The reader MUST preserve backup path components as bytes without lossy UTF-8 conversion.

Provenance: SRC-DIFF, SRC-PATCH; POLICY for the public Rust representation. Terminal display uses an escaped representation distinct from path identity.

### PARSE-PATH-002

A restore path MUST be relative to the backup root and contain no NUL, empty interior component, dot component, or parent component after format-specific root decoding.

Provenance: SRC-PATCH rejects parent components; remaining restrictions are POLICY. The empty component sequence represents the root; `.` or an empty suffix at the operation prefix is a source-derived root marker. A single format-permitted directory suffix separator is removed before component validation.

## Object names and dates

Informative grammar, where `G` is the configured global prefix, `K` the configured type prefix, `T` a long timestamp, `t` a short timestamp, `N` a decimal volume, and `n` a base36 volume:

| Kind | Long stem after `G + K` | Short stem after `G + K` |
| --- | --- | --- |
| Full volume | `duplicity-full.T.volN.difftar` | `df.t.n.dt` |
| Incremental volume | `duplicity-inc.T.to.T.volN.difftar` | `di.t.t.n.dt` |
| Full manifest | `duplicity-full.T.manifest` | `df.t.m` |
| Incremental manifest | `duplicity-inc.T.to.T.manifest` | `di.t.t.m` |
| Full signatures | `duplicity-full-signatures.T.sigtar` | `dfs.t.st` |
| Incremental signatures | `duplicity-new-signatures.T.to.T.sigtar` | `dns.t.t.st` |

`K` selects archive, manifest, or signature prefix by kind. Defaults for `G` and `K` are empty; `duplicity-` is part of the long stem. This follows SRC-NAME and DOC-DUP; DOC-NAME's description of a default global `duplicity-` prefix is imprecise relative to the inspected implementation. A long stem can have `.part` before the envelope suffix; a short stem can have `.p`. Envelope suffix is empty, `.gz`, `.gpg`, or recognized-but-unsupported `.p7m`. Statistics objects are recognized as auxiliary; their contents are outside the reader's restoration grammar. OPEN-NAMES records cases not yet resolved.

### PARSE-NAME-001

The reader MUST classify supported names into the six object kinds in the object-name grammar with their backup dates, volume number where applicable, envelope, and partial status.

Provenance: DOC-NAME, SRC-NAME. Matching consumes the complete basename; unrelated objects are ignored during discovery, and recognized unsupported variants receive diagnostics.

### PARSE-NAME-002

Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem.

Provenance: DOC-DUP, SRC-NAME; literal matching is POLICY rather than reproducing unescaped regular expressions.

### PARSE-NAME-003

Short-name timestamps and volume numbers MUST be decoded as unsigned base36 numbers using the digits `0-9` and `a-z`.

Provenance: DOC-NAME, SRC-NAME. A short timestamp is seconds since the Unix epoch, not a compressed calendar string. Reject empty numerals, invalid digits, zero volume numbers, and numeric overflow; the latter validation choices are POLICY.

### PARSE-NAME-004

The long timestamp parser MUST recognize canonical UTC `YYYYMMDDTHHMMSSZ` dates independently of the host timezone.

Provenance: DOC-NAME, SRC-TIME. Calendar validation and legacy/leap-second handling await OPEN-TIME; no locale-relative command-line time syntax is implied by this requirement.

### PARSE-NAME-005

Objects marked partial MUST be excluded from completed restore sets.

Provenance: DOC-NAME; exclusion is POLICY. They can be displayed as interrupted artifacts.

### PARSE-NAME-006

An incremental set MUST have an end timestamp strictly later than its start timestamp.

Provenance: POLICY. Ambiguous or inconsistent identities do not authorize choosing an arbitrary object.

## Envelopes and integrity

### PARSE-ENV-001

The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.

Provenance: DOC-DUP, SRC-NAME. Gzip member/trailing-byte policy awaits OPEN-ENVELOPE.

### PARSE-ENV-002

The reader MUST support symmetric and public-key OpenPGP objects within the explicitly documented crypto capability profile.

Provenance: DOC-DUP; capability bounds await OPEN-CRYPTO. Includes supported encrypted secret-key exports, compressed messages, and supplied passphrases; this does not promise arbitrary GnuPG options or every historical algorithm.

### PARSE-ENV-003

The reader MUST finish the supported OpenPGP message integrity checks before accepting an object for successful restoration.

Provenance: POLICY. Staging may hold decoded data pending validation; the requested destination cannot be published first. Legacy unauthenticated packets await OPEN-CRYPTO.

### PARSE-ENV-004

When signer verification is configured, the reader MUST require a valid signature from an explicitly permitted signer fingerprint.

Provenance: POLICY. Wrong credentials, missing keys, unsupported algorithms, and invalid authentication are distinct errors.

### PARSE-ENV-005

Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.

Provenance: SRC-HASH and manifest use in duplicity restoration. SHA1 and MD5 are compatibility digests, not proof of authenticity; an unsigned manifest can be replaced along with a volume.

### PARSE-ENV-006

An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.

Provenance: POLICY. Missing required digests, conflicting digests, and malformed digest values await OPEN-MANIFEST. No unchecked success is implied.

## Manifests

Known emitted structure from SRC-MAN: optional `Hostname value` and `Localdir token` lines; `Filelist count` followed by indented `action token` records; volume sections beginning `Volume N:` with indented `StartingPath token [block]`, `EndingPath token [block]`, and `Hash algorithm hex` lines. Field names are accepted case-insensitively by the inspected reader. A block number is optional decimal metadata and differs from a volume number. Paths denote slash-separated byte components; `.` denotes the root.

Quoted tokens use surrounding double quotes and `\xHH` byte escapes. The producer escapes ASCII whitespace, backslash, double quote, and single quote. Unquoted tokens contain none of these bytes. Escape decoding is byte-preserving; it is not shell, JSON, or Unicode unescaping. Single-quote acceptance, unknown fields, optional headers, and historical file-list grammar remain in OPEN-MANIFEST.

### PARSE-MAN-001

The manifest parser MUST recover ordered volume records containing volume number, starting path, ending path, optional block boundaries, and declared hashes.

Provenance: SRC-MAN. Required path fields, volume positivity, and numeric conversion are validated; repeated records remain ordered until restart interpretation.

### PARSE-MAN-002

The manifest token parser MUST decode a valid double-quoted token's `\xHH` escape to exactly the represented byte.

Provenance: SRC-MAN. Both hexadecimal letter cases are accepted. Plain permitted bytes between escapes remain unchanged.

### PARSE-MAN-003

The manifest token parser MUST reject incomplete escapes, non-hexadecimal escape digits, unsupported escape forms, and unterminated quoted tokens.

Provenance: POLICY derived from SRC-MAN's asserted escape syntax. Errors replace assertions.

### PARSE-MAN-004

Manifest interpretation MUST replace earlier records of a repeated volume number with its last record and discard volume records above the last recorded volume number.

Provenance: SRC-MAN. Example `1,2,3,2` leaves volumes `1,2`, using the second record for `2`; the same rule applies when a later suffix is rewritten.

### PARSE-MAN-005

Volume selection MUST include every volume whose declared inclusive path interval can contain a selected entry or descendant, including shared multipart boundary paths.

Provenance: SRC-MAN; conservative selection is POLICY. Compare byte-component sequences, not rendered strings. Invalid or insufficient ranges require a full scan or error rather than silently omitting a needed volume.

### PARSE-MAN-006

The reader MUST retain the distinction between absent block boundaries and explicit numeric block boundaries.

Provenance: SRC-MAN. Exact block-boundary meaning awaits OPEN-MULTIPART; absence is not numeric zero.

## Tar containers and duplicity entries

Tar header blocks and payload alignment use 512-byte units. The inspected producer closes data volumes with zero padding to a 10240-byte record boundary; when already aligned it writes another zero record. Each stored data volume is decoded as its own tar container before records are combined. It is not valid to concatenate encrypted or compressed object bytes and treat them as one tar stream.

Data entry names: `snapshot/<path>`, `diff/<path>`, `deleted/<path>`, `multivol_snapshot/<path>/<part>`, and `multivol_diff/<path>/<part>`. Signature entry names: `signature/<path>`, `snapshot/<path>`, and `deleted/<path>`. A trailing separator in a directory/root entry is interpreted using the format-specific path rules. Multipart `part` numbers are positive decimal integers starting at 1; they are unrelated to stored volume numbers.

### PARSE-TAR-001

The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

Provenance: SRC-TAR; exact numeric variants and checksum compatibility await OPEN-TAR.

### PARSE-TAR-002

The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

Provenance: SRC-TAR. Padding bytes are container bytes, not file content.

### PARSE-TAR-003

The reader MUST support the reviewed GNU/ustar headers and GNU long-name/long-link records emitted by the supported producer profiles.

Provenance: SRC-TAR, SRC-DIFF; full profile awaits OPEN-TAR. Review supported PAX records separately rather than silently ignoring path or size overrides.

### PARSE-TAR-004

Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

Provenance: POLICY. An unknown encoding cannot be skipped by guessing its size or path semantics.

### PARSE-TAR-005

The tar parser MUST validate archive termination and reject non-padding data after the accepted end marker.

Provenance: POLICY. Accepted zero-padding lengths and empty signature archives await OPEN-TAR; arbitrary trailing bytes are not padding.

### PARSE-ENTRY-001

The reader MUST classify duplicity data entries using the five data-entry prefixes defined above.

Provenance: SRC-DIFF, SRC-PATCH. Snapshot content supplies replacement content; diff content is a librsync delta; deleted entries remove the path from the snapshot state.

### PARSE-ENTRY-002

The reader MUST classify signature entries using the three signature-entry prefixes defined above.

Provenance: SRC-DIFF. Preserve tar metadata separately from signature payload metadata.

### PARSE-ENTRY-003

The reader MUST reject an unrecognized duplicity operation prefix in an otherwise recognized data or signature archive.

Provenance: SRC-DIFF, SRC-PATCH. An archive entry is not a raw filesystem extraction instruction.

### PARSE-MULTI-001

Multipart assembly MUST concatenate payloads for one operation and path in consecutive part-number order starting at 1.

Provenance: SRC-DIFF for emitted numbering; stricter validation is POLICY. Parts can cross stored volume boundaries.

### PARSE-MULTI-002

Multipart assembly MUST reject missing, duplicate, reversed, mixed-operation, or interleaved parts of the same logical entry.

Provenance: POLICY. A zero-length last part is valid producer behavior when content length exactly matches the producer chunk size.

### PARSE-MULTI-003

The reader MUST establish multipart completion using the reviewed container and manifest boundary rules before reporting a reconstructed file as complete.

Provenance: POLICY; depends on OPEN-MULTIPART. There is no explicit final-part flag in the inspected emitted entry name.

## Librsync signatures

The signature header is three big-endian 32-bit unsigned fields: magic, block length, strong-checksum length. It is followed by complete records, each containing a 32-bit weak checksum and the indicated strong-checksum bytes. The final original source block can be shorter than the block length; its exact length is not encoded here.

Reviewed magic values: `0x72730136` (MD4), `0x72730137` (BLAKE2), `0x72730146` (RabinKarp/MD4), and `0x72730147` (RabinKarp/BLAKE2). Reading signature metadata does not require implementing backup signature generation.

### PARSE-SIG-001

The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

Provenance: DOC-RSYNC, SRC-RSYNC.

### PARSE-SIG-002

The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

Provenance: DOC-RSYNC, SRC-RSYNC. Permitted checksum lengths require immutable upstream evidence before implementation; see OPEN-SIGNATURE.

### PARSE-SIG-003

The reader MUST represent original regular-file size as unknown when only a signature payload provides size information.

Provenance: DOC-RSYNC, SRC-DIFF; POLICY for exposed metadata. Neither signature tar payload size nor block count supplies an exact original file size.

## Librsync deltas

The complete delta grammar is `magic command* END`, where `command` denotes only a literal or copy command, magic is big-endian `0x72730236`, and END is byte `0x00`. END cannot occur inside `command*`. All numeric arguments are unsigned big-endian. Let `widths = [1, 2, 4, 8]`.

| Opcode | Arguments and payload |
| --- | --- |
| `0x00` | No arguments; end |
| `0x01..0x40` | No length argument; exactly opcode-value literal bytes |
| `0x41..0x44` | Length argument of `widths[opcode - 0x41]` bytes; then that many literal bytes |
| `0x45..0x54` | Let `k = opcode - 0x45`; offset argument has `widths[k / 4]` bytes; length argument has `widths[k % 4]` bytes |
| `0x55..0xff` | Reserved; unsupported command |

Here `/` is integer division. The copy range addresses the fixed basis file, never the output under construction. Zero-length explicit literal or copy commands contribute no output. They still consume their opcode and numeric arguments. Strict single-stream termination is a project policy, separate from any librsync API's willingness to leave unused input.

### PARSE-DELTA-001

The delta parser MUST require the reviewed delta magic before decoding commands.

Provenance: DOC-RSYNC, SRC-RSYNC.

### PARSE-DELTA-002

The delta parser MUST decode every nonreserved opcode according to the complete delta grammar above.

Provenance: SRC-RSYNC. Covers all 64 implicit literals, four explicit literal widths, all 16 copy width combinations, and END.

### PARSE-DELTA-003

A literal command MUST append exactly its indicated payload bytes to the reconstructed output.

Provenance: DOC-RSYNC, SRC-RSYNC. Both implicit and explicit forms have the same output semantics.

### PARSE-DELTA-004

A copy command MUST append exactly the indicated byte range from the fixed basis file.

Provenance: DOC-RSYNC. Order, overlap, and repeated basis ranges do not change these semantics.

### PARSE-DELTA-005

A copy command MUST be rejected unless its offset and length describe a range wholly within the basis file.

Provenance: POLICY. Check offset at most basis length, then length at most basis length minus offset; offset equal to basis length is valid only for zero length.

### PARSE-DELTA-006

Zero-length explicit literal and valid zero-length copy commands MUST consume their encoded arguments and leave reconstructed output unchanged.

Provenance: POLICY consistent with command semantics. This preserves progress and avoids treating zero as an end marker.

### PARSE-DELTA-007

Reserved delta opcodes MUST produce an unsupported-command error.

Provenance: SRC-RSYNC.

### PARSE-DELTA-008

A complete delta stream MUST contain END immediately after its last complete command and contain no bytes after END.

Provenance: POLICY. A missing END, a truncated argument, or a truncated literal is an error; raw tar padding is outside the delta payload.

## Snapshot interpretation and browsing

### PARSE-CHAIN-001

A restorable snapshot MUST be derived from one full set followed by a uniquely selected sequence of incremental sets whose start times equal the preceding end times.

Provenance: DOC-DUP; uniqueness is POLICY. The full-set timestamp is its effective end time.

### PARSE-CHAIN-002

The reader MUST report missing sets, missing required volumes, and ambiguous chain branches without silently treating the resulting snapshot as complete.

Provenance: POLICY. Metadata-only listings explicitly distinguish discovery from validated restorability.

### PARSE-STATE-001

Snapshot interpretation MUST apply additions, replacement content, deltas, deletions, and file-type changes in chain order to the selected backup-relative path state.

Provenance: SRC-DIFF, SRC-PATCH. Exact ancestor/subtree rules and metadata-only deltas require OPEN-STATE before implementation.

### PARSE-STATE-002

A delta entry MUST use the reconstructed regular-file content for that same path from the preceding state as its basis.

Provenance: SRC-PATCH; rejecting a missing or incompatible basis is POLICY.

### PARSE-BROWSE-001

Listings derived from signature archives MUST distinguish metadata availability from volume-integrity and restorability validation.

Provenance: DOC-DUP. Signature listing can succeed even when data volumes are absent or corrupt.

### PARSE-BROWSE-002

When signature metadata cannot provide a requested listing, the reader MUST expose that a volume scan is required before starting that scan.

Provenance: POLICY. Show expected download bytes when known; unknown sizes are not displayed as zero.

### PARSE-SOURCE-001

The reader MUST detect changes to an object identity between discovery and use whenever the source provides version or conditional-read metadata.

Provenance: POLICY. S3 ETag is an identity/conditional-read token, not a replacement for a manifest digest; immutable version IDs are preferred when available.

## Restoration boundary

### RESTORE-001

A restoration MUST preserve each selected entry's backup-relative path beneath a destination that did not exist when publication began.

Provenance: POLICY. Parent directories required to contain the selection are created inside the restore tree.

### RESTORE-002

A restoration MUST write into private staging on the destination filesystem and publish the completed tree without replacing an existing destination.

Provenance: POLICY. Destination existence is checked atomically at publication, not only before downloading.

### RESTORE-003

Restoration filesystem operations MUST prevent traversal through symlinks outside the private staging tree.

Provenance: POLICY. Use directory-relative operations with controlled lookup; symlink target bytes are preserved, and symlinks are never followed while creating descendants.

### RESTORE-004

The reader MUST restore supported regular files, directories, and symlinks with ordinary permissions and supported modification times.

Provenance: DOC-DUP for available metadata; POLICY for Linux fidelity. Preserve ordinary mode bits `0o777`; do not restore ownership, setuid, setgid, or sticky bits. Exact timestamp precision is recorded per supported tar profile.

### RESTORE-005

A restore selection requiring unsupported filesystem entry types MUST fail before publication with an entry-specific diagnostic.

Provenance: POLICY. Special-file metadata can still be displayed; silently dropping selected entries is not success.

### RESTORE-006

Cancellation or a handled restoration failure MUST leave the requested destination unpublished and attempt removal of temporary plaintext.

Provenance: POLICY. Cleanup failures are reported. Process crashes and power loss can leave private staging artifacts; the guarantee is not secure erasure or crash-proof cleanup.

## Verification and traceability requirements

### ASSURE-SPEC-001

Each parser component MUST have reviewed requirements and resolved blocking questions before its implementation begins.

Provenance: User development decision. The specification author prepares requirements; the implementation and verification agents review feasibility and assurance independently.

### ASSURE-VEST-001

Parsers using Vest SHOULD be authored in the Vest DSL.

Provenance: User development decision, DOC-VEST.

### ASSURE-VEST-002

Every direct-combinator fallback MUST document the DSL limitation or suitability reason and its affected requirement IDs.

Provenance: User development decision. A fallback retains the same proof and compatibility obligations.

### ASSURE-VEST-003

The reviewed generated parser specifications MUST express the applicable normative requirements before their proofs are accepted as evidence of those requirements.

Provenance: DOC-VEST, POLICY. The DSL frontend is unverified; a proof against generated semantics alone does not validate format intent.

### ASSURE-PROOF-001

Production parsing and reconstruction MUST execute the same verified implementations used to establish accepted correctness claims.

Provenance: User assurance decision. Separate unverified algorithms do not inherit proof evidence.

### ASSURE-PROOF-002

The proof plan MUST cover parser refinement, consumption bounds, arithmetic safety, termination, panic freedom, delta output semantics, multipart order, chain continuity, state transitions, and restore-plan path validity for the supported profile.

Provenance: User assurance decision. Filesystem race prevention, crypto primitives, network behavior, and operating-system cleanup remain trusted integration boundaries with explicit contracts and tests.

### ASSURE-PROOF-003

Each trusted adapter or upstream trusted computing base assumption MUST have a reviewed rationale and identified affected correctness claims.

Provenance: POLICY. Reviewed trust applies only to the enumerated adapters and pinned upstream verification infrastructure. A newly weakened contract or added assumption cannot complete an outstanding proof obligation without specification review.

### ASSURE-PROOF-004

Project parsing and reconstruction algorithms MUST contain no assumption or admission of unproved facts, unchecked axioms, assumed specifications, external-body proof bypasses, or termination exemptions.

Provenance: POLICY. This excludes proof placeholders from production algorithms. Enumerated trusted adapters and pinned upstream infrastructure are recorded under ASSURE-PROOF-003; they cannot serve as arbitrary proof-closing escapes in core algorithms.

### ASSURE-TRACE-001

Every implemented requirement MUST have Duvet links from authored implementation and applicable test evidence to its specification heading and quoted normative text.

Provenance: User Duvet decision, DOC-DUVET. Source comments are citations, not execution evidence. Generated copies are excluded from duplicate authored coverage.

### ASSURE-TRACE-002

Formal proof obligations and successful verification results MUST be recorded separately by requirement ID with the verified source identity and tool configuration.

Provenance: POLICY. Duvet has no documented native proof category; no fictitious annotation type is introduced.

### ASSURE-TRACE-003

The acceptance gate MUST compare Duvet's derived requirement snapshot with a reviewed snapshot committed to the repository.

Provenance: User Duvet decision, DOC-DUVET. Requirement wording, scope, and evidence-link changes therefore require an explicit reviewed baseline update; snapshot agreement alone is not completion evidence.

### ASSURE-TRACE-004

Each completed milestone MUST pass Duvet implementation and test coverage checks for its declared requirement sections.

Provenance: POLICY. The machine-readable milestone declaration identifies the active scope, proof/test obligations, and pending roadmap. Final supported-profile acceptance covers every applicable supported requirement; incomplete roadmap entries remain explicitly pending and confer no assurance claims.

### ASSURE-BUILD-001

The acceptance build MUST fail on failed or incomplete required proofs, missing verification tools, stale generated parsers, or unreviewed verification exemptions.

Provenance: User verified-build decision. The intended default is `cargo xtask build`; ordinary Cargo builds can be used for development but confer no verification status.

### ASSURE-TEST-001

Compatibility tests MUST restore supported snapshots produced by official duplicity and compare contents and supported metadata with both their source tree and official restoration.

Provenance: User test decision. Checked-in fixtures support offline tests; producer regeneration fails rather than skips when required tools are unavailable.

### ASSURE-PROGRESS-001

README completion records MUST distinguish specification review, implementation, successful proofs, successful compatibility tests, and independent review for each completed component.

Provenance: User documentation decision. Requirement counts or annotation coverage percentages are not semantic proof coverage.

### ASSURE-COMMIT-001

Development MUST record meaningful validated checkpoint commits with the corresponding README task status and assurance evidence.

Provenance: User development decision. A checkpoint records its actual validation level; specification and tooling checkpoints do not imply passing parser proofs or completed compatibility support.

## Verification goals and acceptance scenarios

This table is an initial mapping of obligations, not passing evidence. The verification agent owns the precise mathematical contracts and reviews the source author cannot establish alone.

| Goal | Requirements | Expected evidence | Current status |
| --- | --- | --- | --- |
| VG-PARSE | PARSE-GEN, PARSE-PATH, PARSE-NAME, PARSE-MAN, PARSE-TAR, PARSE-SIG | Refines reviewed byte grammar; bounded consumption and arithmetic; finite-input termination/panic freedom | Planned |
| VG-DELTA | PARSE-DELTA | Command decoder refinement and concatenation of literal/basis slices; source/output bounds | Planned |
| VG-ASSEMBLY | PARSE-MULTI, PARSE-MAN-004..006 | Ordered concatenation, restart normalization, conservative volume selection, completion conditions | Planned; boundary question open |
| VG-STATE | PARSE-CHAIN, PARSE-STATE | Unique contiguous chain and path-state transitions | Planned; transition question open |
| VG-RESTORE | PARSE-PATH, RESTORE | Valid selected paths and destination mapping; filesystem contract review and integration tests | Planned |
| VG-INTEGRATION | PARSE-ENV, PARSE-SOURCE, PARSE-BROWSE | Trusted-adapter contract review, corruption/decryption/source-change tests | Planned; crypto profile open |

Official-producer scenarios: full and multiple incrementals; add/change/delete/recreate/type replacement; metadata-only changes; empty files/directories; symlinks; long and byte-valued paths; whitespace/quotes/backslashes; custom prefixes; short/legacy names where producers support them; gzip/raw/symmetric/public-key/signed objects; incompressible multipart data and deltas; restart manifests; missing signatures/volumes/chain links; all selected snapshot dates. Compare source trees and official restored trees for contents, paths, types, symlink targets, modes, and supported timestamps. Preserve producer version, exact command, environment, source-tree expectation, and archive digests.

Adversarial scenarios: every opcode and width including implicit literals and zero-length commands; truncation at every field boundary; reserved commands, wrong magic, missing/trailing END, out-of-bounds copies, integer overflow, wrong passwords/keys/signers, mutated digests, malformed quoting, invalid tar checksums, malicious path components, symlink escapes, destination races, multipart loss/reordering, object replacement during reads, resource exhaustion, cancellation, and interrupted restoration. Official rdiff provides independent delta test data; it is not a runtime dependency. S3 tests include pagination and source conditions; TUI tests check that display escaping does not alter path identity.

## Open questions and implementation holds

| ID | Unresolved question | Evidence and required next action | Affected requirements |
| --- | --- | --- | --- |
| OPEN-TIME | Legacy `YYYY-MM-DDTHH:MM:SS±HH:MM`, configurable separators, leap seconds, calendar normalization, accepted year bounds and case | SRC-TIME has permissive/asserted normalization; pin expected semantics and test legacy official fixtures rather than inherit host time behavior | PARSE-NAME-004, chain date interpretation |
| OPEN-NAMES | Partial incremental volume recognition and obsolete short-name production; exact auxiliary statistics grammar | SRC-NAME emits/recognizes partial markers inconsistently for incremental volumes; compare historical sources and fixtures before claiming all variants | PARSE-NAME-001,005 |
| OPEN-MANIFEST | Historical/optional Filelist records, single quotes, duplicate fields, unknown fields, hash absence/algorithm conflict, newline variants and manifest completeness | SRC-MAN's permissive readers and producer syntax differ; select exact profile and rejection policy from producer fixtures and reviewed source | PARSE-MAN-001..003, PARSE-ENV-005..006 |
| OPEN-TAR | Header numeric formats, signed checksums, GNU variants/PAX fields, timestamp precision, empty sigtars, exact end markers and padding | Inspect SRC-TAR and public tar standards; test actual producer containers and negative cases | PARSE-TAR-001..005, RESTORE-004 |
| OPEN-MULTIPART | Last-part detection, incomplete final volumes, block-boundary semantics and duplicate metadata | SRC-DIFF emits no explicit final-part flag; SRC-PATCH groups by path; prove completion using manifests/container endpoints after producer testing | PARSE-MAN-006, PARSE-MULTI-003 |
| OPEN-SIGNATURE | Accepted strong-checksum lengths for each magic and whether additional signature variants are required | DOC-RSYNC describes truncation; pin upstream length-validation source and producer variants | PARSE-SIG-002 |
| OPEN-STATE | Metadata-only deltas, deletion of directories and ancestor replacement, ordering constraints within a set | Review SRC-DIFF/SRC-PATCH transition functions and compare official restoration for constructed trees | PARSE-STATE-001 |
| OPEN-CRYPTO | Exact pgp library version/capabilities, legacy unauthenticated messages, OpenPGP packet variants/compression/signature policy | Establish native adapter profile with official GnuPG fixtures and RFC sources; unsupported legacy formats stay explicit | PARSE-ENV-002..004 |
| OPEN-ENVELOPE | Gzip multiple members, trailing zeros, decoded-object termination | Consult gzip standards and producer outputs before fixing accepted envelope variants | PARSE-ENV-001 |
| OPEN-LIMITS | Concrete defaults and stream/cache/staging contracts that make bounds enforceable | Measure official fixtures and design adapters before selecting defaults; all bounds remain visible/configurable | PARSE-GEN-005 |

Resolving a question updates the relevant normative text, provenance, proof obligations, and test scenarios. Broader compatibility claims require producer evidence for the claimed release. Duvet snapshots and README completion records are updated together after independent review.
