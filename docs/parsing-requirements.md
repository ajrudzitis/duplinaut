# Duplicity reader parsing requirements

Status: initial specification reviewed by the coordinator and independent verification agent. Written before parser implementation. Unresolved formats remain held. Implementation, proof, and compatibility completion are recorded separately in the assurance ledger; this specification does not itself confer completion.

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
| DOC-TAR | [Python 3.13 tar formats](https://docs.python.org/3.13/library/tarfile.html) | ustar/GNU/PAX interoperability and current PAX producer default |
| DOC-PGP | [RFC 9580](https://www.rfc-editor.org/rfc/rfc9580), historical [RFC 4880](https://www.rfc-editor.org/rfc/rfc4880), [RFC 6637](https://www.rfc-editor.org/rfc/rfc6637) | Packet integrity, composition, v4 keys and legacy Curve25519 ECDH |
| DOC-RPGP | [rPGP 0.21.0 API](https://docs.rs/pgp/0.21.0/pgp/), [pinned implementation status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/IMPL_STATUS.md), [security status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/SECURITY_STATUS.md) | Native adapter capability and trusted upstream limitations |
| DOC-GZIP | [RFC 1952](https://www.rfc-editor.org/rfc/rfc1952) | Gzip member structure, CRC32 and uncompressed-size checks |
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
| SRC-MAIN | [dup_main.py](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/dup_main.py): `write_multivol.get_indicies`, manifest publication | `6ae35084d2c705ae3c55bb100750ae50328c46c3090dcb3631d837ae1d96023e` |

SRC-RSYNC references the official [opcode table](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/prototab.c) and [magic numbers](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/librsync.h), independently fetched and reviewed on 2026-10-07 at commit `271744de428c0177e993f95c045d70b8cf2558f2`. Their SHA-256 values are `2c7500cd1d485ef55720707607b7876fdc00a42677f53aab465b114f05eaa972` and `4974e6e51fabc0de94b03e7b640cdd21e4979a41bdf3e7bd975bc029aa655762`, respectively. The public format page omits the implicit literal opcodes, so source fallback is necessary.

At the same immutable librsync commit, [sumset.h](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/sumset.h) validates signature lengths (SHA-256 `5085b769fe4b67454ac0a099acc8af052d67f28bdcb18cfccf5c556f4738b831`); [checksum.c](https://github.com/librsync/librsync/blob/271744de428c0177e993f95c045d70b8cf2558f2/src/checksum.c) defines MD4 length 16 and BLAKE2 length 32 (SHA-256 `bea24c9e1ab77bb04a1d12e8233f0ac57f687d16603157234bb2230ec08e543c`).

Observed official producer evidence is stored in `tests/fixtures/duplicity-3.0.7/`: deterministic source-tree oracles in `expected.json`, original archive digests and exact producer/restoration argv in `provenance.json`, and container/multipart observations in `format-observations.json`. The complete source recipe is `tests/producer/generate.py`. Additional historical producer evidence is indexed by `tests/fixtures/historical-producers.json`: 0.8.23 standard/short/legacy and 2.1.3/3.2.1 standard small plain chains, totaling 15 successfully source-compared official restores. Per-version producer environment records pin the verified source archives and original unmodified extension builds. Successful official restoration establishes producer/oracle consistency, not native-reader completion.

SRC-HIST-321: official duplicity 3.2.1 source distribution SHA-256 `4284e3d8f0082a85d3dcfa634901532a1087463df7a33ead417670c0d63b9ef5`; inspected `gpg.py` SHA-256 `1d10c9a47f891bec30ee1b420c1c0ff448e65516590a5b061d13ab42fb0a3674` and `diffdir.py` SHA-256 `39cd139757729b0ef483883664935abde9ac81404fb89da7e35bc9a561873eef`. `GzipWriteFile` now calls `get_footer`; `TarBlockIter.get_footer` still pads to the next 10240-byte record, including a complete record when already aligned. Source fallback explains the independently observed change in actual official plain fixtures.

Native crypto reference: published `pgp = "=0.21.0"`, released 2026-10-03, crate SHA-256 `ae70f4d9325a391db30d115d6191a7bb67cec856aa0bba131290f0c4e09a532a`, upstream commit `c2b192e52b2e7f99777196539ea79b37f1f9ff30`. Its Rust 1.88 minimum fits the pinned Rust 1.98 toolchain. Default BZip2 support is retained; malformed-artifact compatibility, experimental forwarding, PQC, and large-RSA extensions are excluded. The native dependency graph and adapter remain trusted boundaries, not Verus-proved cryptography. See [native adapter notes](openpgp-profile.md).

Target compatibility versions are duplicity 0.8.23, 2.1.3, 3.0.7, and 3.2.1. These are native compatibility targets; original official-producer/reference evidence is recorded separately and does not establish native-reader support. Runtime parsing, decryption, decompression, indexing, and restoration are native Rust operations. Official duplicity, GnuPG, and rdiff are producers and reference implementations for tests.

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

Provenance: POLICY. Defaults: decoded object 8 GiB; reconstructed file 1 TiB; temporary storage 64 GiB; total retained metadata 256 MiB; entries/objects 10 million each; path bytes 1 MiB; path components 1024; nested envelope depth 8; gzip members 1024. All are configurable before opening a repository. Streaming operations enforce running totals before exceeding a bound. Larger historical repositories may require explicit larger limits; exceeding a default is a resource-limit error, not malformed input.

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

`K` selects archive, manifest, or signature prefix by kind. Defaults for `G` and `K` are empty; `duplicity-` is part of the long stem. This follows SRC-NAME and DOC-DUP; DOC-NAME's description of a default global `duplicity-` prefix is imprecise relative to the inspected implementation. Every long stem can have `.part` before the envelope suffix; every short stem can have `.p`. SRC-NAME's producer emits partial incremental volume names while its reader regex does not consistently record their partial status; the project deliberately classifies those emitted names as partial rather than reproducing that inconsistency. Long-stem envelope suffix is exactly empty, `.gz`, `.gpg`, or recognized-but-unsupported `.p7m`; short-stem suffix is exactly empty, `.z` (gzip), or `.g` (OpenPGP), as confirmed by the official 0.8.23 short-name corpus; arbitrary additional suffixes are rejected for recognized stems. Statistics objects are auxiliary, outside the restoration grammar, and do not supply snapshot/volume completeness evidence.

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

Provenance: DOC-NAME, SRC-TIME. The supported date profile uses real Gregorian calendar dates, years 1901..2099, hour 00..23, minute/second 00..59. The reader rejects normalization-only dates and leap-second fields 60/61 rather than reproducing Python's normalization. No locale-relative command-line time syntax is accepted.

### PARSE-NAME-007

The long timestamp parser MUST support legacy explicit-zone `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS±HH:MM` timestamps with the configured single-byte time separator in place of colons.

Provenance: SRC-TIME; valid-calendar bounds follow PARSE-NAME-004. Zone hours/minutes are 00..23/00..59, and UTC is local calendar time minus the signed offset. The separator defaults to `:` and is supplied explicitly by the caller for other archives. Its precise supported predicate is one byte in `0x21..0x7e` excluding `/`, `.`, `+`, `-`, ASCII digits, and `T`, `t`, `Z`, `z`; other values are configuration errors. This is POLICY rather than the upstream parser's unrestricted configuration. Accepted `T`/`Z` letters are case-insensitive. Short timestamps remain epoch seconds. Official 0.8.23 legacy UTC filenames with `_` separator have source/restore evidence; other separators/non-UTC legacy zones and native compatibility remain pending, distinct from this explicit reader grammar.

### PARSE-NAME-005

Objects marked partial MUST be excluded from completed restore sets.

Provenance: DOC-NAME; exclusion is POLICY. They can be displayed as interrupted artifacts.

### PARSE-NAME-006

An incremental set MUST have an end timestamp strictly later than its start timestamp.

Provenance: POLICY. Ambiguous or inconsistent identities do not authorize choosing an arbitrary object.

## Envelopes and integrity

### PARSE-ENV-001

The reader MUST decode unencrypted raw objects and gzip-enveloped objects according to their classified envelope.

Provenance: DOC-DUP, SRC-NAME, DOC-GZIP. Gzip accepts consecutive complete members up to the configured count, validates CRC32 and ISIZE for every member, and concatenates their decoded bytes. Any remaining byte that is not another valid gzip member is rejected; zero trailing bytes are not silently accepted. Official current archives contain one member. Header optional fields are bounded by decoded/input resource limits.

### PARSE-ENV-002

The reader MUST support symmetric and public-key OpenPGP objects within the explicitly documented crypto capability profile.

Provenance: DOC-DUP, DOC-PGP, DOC-RPGP; the normative capability profile is PARSE-ENV-007..016. This does not promise arbitrary GnuPG options or every historical algorithm.

### PARSE-ENV-003

The reader MUST finish the supported OpenPGP message integrity checks before accepting an object for successful restoration.

Provenance: DOC-PGP, DOC-RPGP, POLICY. SEIPDv1 prefix and complete MDC are checked before plaintext is parsed as a duplicity object. Plaintext may remain in private staging until signer, container, manifest, and restoration validation finish; the requested destination cannot be published first. MDC integrity is not signer identity.

### PARSE-ENV-004

When signer verification is configured, the reader MUST require a valid signature from an explicitly permitted signer fingerprint.

Provenance: POLICY. Wrong credentials, missing keys, unsupported algorithms, and invalid authentication are distinct errors.

### PARSE-ENV-005

Manifest volume digests MUST be checked against the original stored object bytes before a restoration using those volumes is published.

Provenance: SRC-HASH and manifest use in duplicity restoration. SHA1 and MD5 are compatibility digests, not proof of authenticity; an unsigned manifest can be replaced along with a volume.

### PARSE-ENV-006

An unknown digest algorithm MUST produce an unsupported-integrity diagnostic when no supported declared digest can validate the volume.

Provenance: POLICY. Every selected volume needs at least one supported declared digest. SHA1 requires exactly 40 hexadecimal digits, MD5 exactly 32; hex case is immaterial. Conflicting declarations of the same algorithm are rejected. Every supported declared digest is checked; an unsupported declaration can be retained for display when supported declarations still validate the volume. No unchecked success is implied.

### PARSE-ENV-007

The native reader MUST accept only SEIPDv1 authenticated encrypted-data containers within its initial OpenPGP profile.

Provenance: DOC-PGP sections 5.7 and 5.13; POLICY. Supported key wrappers are PKESK version 3 and SKESK version 4. SED/tag 9, proprietary GnuPG AEAD/tag 20, SEIPDv2, other wrapper versions, nested encrypted containers, and unknown critical packet variants are unsupported errors, never a weaker decryption fallback. SEIPDv2 is a follow-up requiring independent fixture and integrity evidence.

### PARSE-ENV-008

The adapter MUST restrict decryption and verification algorithms to the enumerated initial capability profile.

Provenance: DOC-PGP, DOC-RPGP, POLICY. Payload encryption: AES-128/192/256. Recipient keys: v4 RSA with modulus 2048..8192 bits, or v4 legacy Curve25519 ECDH (algorithm 18 with its legacy OID and RFC 6637 SHA-256/384/512 and AES wrapping parameters). Binary document signatures: v4 RSA with SHA-224/256/384/512, or legacy Ed25519 (algorithm 22) with SHA-256/384/512. Key certification/binding signatures use these same per-algorithm hash restrictions. RFC 9580 section 5.2.3.3 requires Ed25519Legacy digests of at least 32 octets; SHA-224 is rejected for that algorithm. DSA, ElGamal, other curves, SHA1/MD5 document signatures, v2/v3/v5/v6 keys, hardware/key-agent stubs, and PQC are unsupported. SHA1 for v4 fingerprints, SEIPDv1 MDC, and the separately listed S2K is compatibility usage, not permission for SHA1 document signatures. Session-key cipher policy is checked after unwrapping; a PKESK algorithm alone does not identify the payload cipher.

### PARSE-ENV-009

The adapter MUST support caller-supplied passphrases and exported supported secret keys without invoking external key agents or crypto applications.

Provenance: DOC-RPGP, POLICY. Binary and ASCII-armored v4 key exports may contain unencrypted private material or AES-protected usage-254 secret material with SHA1 integrity and simple, salted, or iterated-and-salted S2K (types 0, 1, 3), using SHA1 or SHA-224/256/384/512. The same S2K profile is supported for SKESKv4. Validate secret-material integrity before its use. Deprecated usage-255 additive-checksum protection and other S2K/protection forms are unsupported. Credentials remain separate from object identity, errors, logs, and public restoration metadata. No import modifies the user's keyring.

### PARSE-ENV-010

The adapter MUST enforce its configured ciphertext bound before materializing an encrypted object and authenticate the complete SEIPDv1 payload before releasing plaintext to message or duplicity parsers.

Provenance: DOC-RPGP `Seipdv1ReadMode::CheckFirst`, POLICY. Default stored encrypted-object bound and CheckFirst bound are 256 MiB each; account for packet framing and the library's body limit separately. Read at most the limit plus a checked one-byte overflow probe; a limit hit without proven EOF is not success. Larger explicitly configured bounds require sufficient memory budget. No automatic Streaming fallback is permitted. The decoded-object default remains 8 GiB and independently limits decompression output.

### PARSE-ENV-011

The adapter MUST consume every accepted message and compression layer to its authenticated end and reject unexpected trailing packets or bytes.

Provenance: DOC-PGP composition rules, DOC-RPGP full-read contract, POLICY. Exactly one literal-data body supplies the decoded duplicity object. Its packet filename and timestamp are untrusted and never select a restoration destination. Supported compression identifiers are 0 (uncompressed), 1 (ZIP/raw DEFLATE), 2 (ZLIB), and 3 (BZip2). Wrapping compression/signature layers are bounded by the configured nesting depth; literal bytes, decompression totals, keys, and packet counts are bounded separately. Hidden recursion inside a library wrapper cannot bypass the adapter depth contract. Partial-body packet lengths are accepted only when their complete terminal chunk is present. Container EOF, MDC success, and declared signature validation are separate checks.

### PARSE-ENV-012

Signer authorization MUST compare complete primary-certificate fingerprints supplied by the caller rather than short key IDs or unverified packet issuer hints.

Provenance: DOC-PGP, POLICY. For supported v4 certificates the fingerprint is exactly 20 bytes (40 hexadecimal digits, case-insensitive input). A matching primary key may sign directly; a signing subkey is authorized only by verified primary-to-subkey binding and verified embedded subkey-to-primary back signature with signing capability. Verify certificate self-signatures and accepted bindings before authorizing any component. The signature must cover the exact binary literal body, not decoded text. Missing or nonmatching signatures fail when signer verification is configured.

### PARSE-ENV-013

Historical signer verification MUST report its explicit offline fingerprint-authorization scope without claiming current certificate validity or Web-of-Trust validation.

Provenance: DOC-RPGP mechanism/policy distinction, POLICY. Caller authorization is by pinned primary fingerprint for offline backup recovery. Cryptographic self/binding/document verification is mandatory; current wall-clock expiry, revocation policy, user-ID identity, trust databases, and online key lookup are outside this initial authorization policy. Available creation/expiry/revocation metadata is reported for caller assessment. Expired certificate dates do not silently invalidate otherwise authorized historical data, and an authorization result is not a claim of present-day certification validity.

### PARSE-ENV-014

The adapter MUST return unsuccessful restoration for any failed integrity, key-binding, configured signer, or complete-read check without publishing staged plaintext.

Provenance: POLICY. Negative evidence includes wrong message/key passphrases, wrong recipients, wrong allowed fingerprints, tampered/truncated ciphertext, absent/invalid MDC, invalid signature, substituted unbound subkey, unsupported algorithms, trailing data, and every configured resource limit. Error classification never implies reliable differentiation between a wrong password and adversarial ciphertext when the cryptographic mechanism cannot provide it.

### PARSE-ENV-015

The crypto adapter's trusted contract and limitations MUST be recorded with the pinned dependency identities and independent positive and negative integration evidence.

Provenance: ASSURE-PROOF-003, DOC-RPGP, POLICY. OpenPGP primitives, library packet parsing, buffering/EOF behavior, and certificate verification remain upstream trusted code. Tests use original official symmetric/public-key/signed objects and publicly labeled fixture credentials; no Verus proof is claimed for the crypto implementation. Upstream documents an unresolved RSA timing/Marvin concern; supporting RSA is not evidence of resistance to local or observable decryption timing attacks.

### PARSE-ENV-016

The adapter MUST enforce configured key, session-key packet, signature, nesting, and S2K-work limits before the corresponding unbounded work or allocation.

Provenance: POLICY. Defaults: exported key input 16 MiB; supplied certificates 64; subkeys/certificate 64; session-key packets/message 64; signatures/message 64; wrapping depth 8; iterated S2K encoded byte-count at most 65011712 per derivation; attempted derivations/message at most 64. Bound totals as well as per-item sizes. Resource rejection may precede authentication; it cannot become an accepted decoded object. The library's internal recursion limit or buffer size alone does not establish these project limits.

## Manifests

Known emitted structure from SRC-MAN and actual 3.0.7 fixtures: optional `Hostname value` and `Localdir token` lines; volume sections beginning `Volume N:` with indented `StartingPath token [block]`, `EndingPath token [block]`, and `Hash algorithm hex` lines; optional `Filelist count` followed by indented `action token` records. Streaming manifests append Filelist **after** volume records; the alternate `Manifest.to_string` serialization places it before them. Both positions are supported. Field names are case-insensitive. A block number is optional positive decimal metadata and differs from a volume number. Paths denote slash-separated byte components; `.` denotes the root. LF or CRLF terminates lines; a final line can omit its terminator. Whitespace separating fields is ASCII space or tab. Duplicate singleton fields within a volume, incomplete volume records, unknown structural fields, and inconsistent file-list counts are rejected under the project profile. Blank lines are permitted between records.

Quoted tokens use surrounding double quotes and `\xHH` byte escapes. Inside quotes literal bytes are permitted except backslash, double quote, NUL, LF, and CR. Raw tabs, spaces, and single quotes are permitted there as a reader POLICY; official producers instead escape ASCII whitespace, backslash, double quote, and single quote. Unquoted tokens contain no byte in ASCII whitespace `0x09..0x0d` or `0x20`, backslash, double quote, single quote, or NUL; they are nonempty. Escape decoding is byte-preserving; it is not shell, JSON, or Unicode unescaping. In particular `\x00` is syntactically valid and path validation subsequently rejects the decoded NUL. Single-quote wrappers are unsupported. The optional Hostname remainder is opaque metadata rather than a filesystem path. Missing headers and Filelist do not prevent data reconstruction; missing required volume integrity/range records do. Filelist actions in the observed profile are `new`, `changed`, and `deleted`; the list is change-summary metadata, not an authoritative restored directory listing.

### PARSE-MAN-001

The manifest parser MUST recover ordered volume records containing volume number, starting path, ending path, optional block boundaries, and declared hashes.

Provenance: SRC-MAN. Required path fields, volume positivity, and numeric conversion are validated; repeated records remain ordered until restart interpretation.

### PARSE-MAN-002

The manifest token parser MUST decode a valid double-quoted token's `\xHH` escape to exactly the represented byte.

Provenance: SRC-MAN and the reviewed reader token grammar above. Both hexadecimal letter cases are accepted. Plain permitted bytes between escapes remain unchanged; an empty double-quoted token is syntactically valid and is validated for its field separately.

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

Provenance: SRC-MAN, SRC-MAIN, SRC-DIFF. A present EndingPath block `n` means multipart part `n` was emitted and remains incomplete at the actual last-entry boundary. A present StartingPath block describes the producer's remembered entry boundary: normally the first entry, but the second after queued restart replay. A one-entry queued replay can instead record root/None. An absent starting block does not establish that the actual first entry is ordinary or part 1; a final short multipart part also records no continuing block. Absence is not numeric zero. Known queued-start exceptions require PARSE-MAN-008..010; the ending and completion checks remain strict.

### PARSE-MAN-007

The manifest parser MUST accept an optional counted Filelist section before or after volume records without interpreting its lines as volume fields.

Provenance: SRC-MAN and official 3.0.7 fixtures. Parse action and token separately; preserve path bytes and reject malformed/count-inconsistent summaries. Filelist absence does not mean the snapshot is empty.

### PARSE-MAN-008

A supported nonempty volume MUST have an EndingPath matching its actual last classified logical-data entry path and pending multipart block state.

Provenance: SRC-MAIN, SRC-DIFF, POLICY. Use component-valued paths. A 65536-byte multipart part records its part number; an ordinary or final short multipart entry records no block. The check survives queued replay because previous-index state was already established.

### PARSE-MAN-009

A reader accepting queued-restart compatibility MUST restrict a nonempty volume's StartingPath exception to the source-derived one-entry root/None or second-entry boundary forms with retained raw restart-rewrite evidence.

Provenance: SRC-MAIN restart_position_iterator, SRC-DIFF queued __next__, original official 3.0.7 probes. The normal profile requires the actual first classified logical-data entry boundary. PAX/GNU metadata headers are not counted as logical-data entries. For a recognized rewritten volume, >=2 entries can record the second entry's path/block, and a sole queued entry can record root/None. A caller without this compatibility path reports an unsupported boundary profile rather than accepting arbitrary starts. Initial queued compatibility additionally requires a raw/gzip decoded entry stream with no footer bytes (`entry_stream_bytes == decoded_length`), as observed in original official 3.0.7 replay. Object filenames do not identify producer version. Queued encrypted or footer-bearing replay remains unsupported pending its own fixture/container review; the strict normal profile is unchanged. This exception cannot relax EndingPath, ordinal/part sequence, integrity, or complete-container checks.

### PARSE-MAN-010

A reader MUST scan every normalized volume of a restart-rewritten set when raw manifest intervals have not been certified to cover decoded entries by an object-identity-bound content index.

Provenance: POLICY derived from the observed false lower boundary after queued replay. Raw repeated-volume evidence is retained through normalization. Scanning includes required source identities, manifest digests, complete containers, and completion validation; unsafe ranges cannot omit selected content. A derived trusted index may later enable conservative pruning independently of raw StartingPath hints.

## Tar containers and duplicity entries

Tar header blocks and payload alignment use 512-byte units. Official 0.8.23 and 2.1.3 plain volumes confirm complete-entry EOF without a zero footer. Actual 3.0.7 raw/gzip data volumes end at a complete entry boundary **without zero end blocks**: `GzipWriteFile` never writes `get_footer`, even on the final volume. In contrast, `GPGWriteFile` writes zero padding of `10240 - (entry_stream_bytes mod 10240)` bytes; this can be only 512 bytes. Official 3.2.1 `GzipWriteFile` also appends that exact zero footer to raw/gzip data volumes, including a 512-byte footer observed in the small full fixture. One zero header block is therefore valid for this producer profile; a universal two-zero-block requirement is incorrect. For a nonfinal encrypted volume it can then append a top-off consisting of the beginning of that stored ciphertext object. Each stored volume is decoded separately before entries are combined. An object boundary alone does not establish logical multipart completion.

Header fields follow the common layout: name bytes 0..100, mode 100..108, uid 108..116, gid 116..124, size 124..136, mtime 136..148, checksum 148..156, type 156, linkname 157..257, magic/version 257..265, user/group names 265..329, and device numbers 329..345. Accepted magic/version bytes are POSIX `ustar\0` plus `00`, old GNU `ustar ` plus ` \0`, and eight zero bytes for V7. Only POSIX/ustar headers interpret bytes 345..500 as a path prefix and join a nonempty prefix with name using `/`; old GNU uses that region for other metadata and never treats it as a prefix. V7 has no prefix either. Interpret name/link bytes up to NUL. Numeric fields accept bounded ASCII octal surrounded by permitted spaces/NUL padding; GNU `0x80` positive or `0xff` negative base256 fields are decoded as in the inspected Python implementation. Negative sizes/modes/IDs are rejected; modification times use signed seconds but emitted negative original mtimes are clamped to zero by duplicity. Header checksum accepts the unsigned or historical signed-byte sum with its eight checksum bytes treated as spaces. Resource/platform bounds apply before use.

Supported metadata entries are GNU `L`/`K` long name/link, PAX `x` per-entry headers, and PAX `g` global headers. PAX records are `decimal-length SPACE key=value LF`, length including the entire record; malformed lengths/terminators and duplicate keys within one header are rejected. Supported semantic keys are `path`, `linkpath`, `size`, `mtime`, `uid`, `gid`, `uname`, `gname`, `atime`, `ctime`, and `hdrcharset`; sparse/other keys affecting interpretation are unsupported. Global values apply until replaced; local values override global values for the next real entry, and empty values delete inherited entries. PAX path/link values normally encode UTF-8; `hdrcharset=BINARY` preserves their original bytes. PAX `size`, `uid`, and `gid` are nonnegative decimal; time values are signed decimal seconds with optional fractional nanoseconds, rejected if nonzero precision would be lost. User/group strings and atime/ctime are display-only metadata. Local/GNU metadata records need a following real entry; a dangling override is an error. The observed Python 3.13 producer uses PAX `path` and `hdrcharset=BINARY` for long/raw-byte names; it does not use GNU `L` for those cases.

Data entry names: `snapshot/<path>`, `diff/<path>`, `deleted/<path>`, `multivol_snapshot/<path>/<part>`, and `multivol_diff/<path>/<part>`. Signature entry names: `signature/<path>`, `snapshot/<path>`, and `deleted/<path>`. A trailing separator in a directory/root entry is interpreted using the format-specific path rules. Multipart `part` numbers are positive decimal integers starting at 1; they are unrelated to stored volume numbers.

### PARSE-TAR-001

The tar parser MUST validate each header checksum and payload extent before accepting the entry it describes.

Provenance: SRC-TAR delegates to Python tarfile; DOC-TAR and the reviewed header profile above. Actual official fixtures confirm PAX path/BINARY headers and whole entry extents.

### PARSE-TAR-002

The tar parser MUST advance over each payload and its padding to the next 512-byte boundary without arithmetic overflow.

Provenance: SRC-TAR. Padding bytes are container bytes, not file content.

### PARSE-TAR-003

The reader MUST support the reviewed GNU/ustar headers, GNU long-name/long-link records, and semantic PAX overrides described in the tar profile above.

Provenance: SRC-TAR, SRC-DIFF, DOC-TAR; official 3.0.7 fixtures exercise PAX path and binary path encoding. Historical GNU and additional supported PAX variant tests remain pending; they do not authorize ignoring an override.

### PARSE-TAR-004

Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

Provenance: POLICY. An unknown encoding cannot be skipped by guessing its size or path semantics.

### PARSE-TAR-005

The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

Provenance: SRC-HASH (`GzipWriteFile`, `GPGWriteFile`), SRC-DIFF, SRC-HIST-321, official 0.8.23/2.1.3/3.0.7/3.2.1 observations. Raw/gzip data volumes accept complete-entry EOF for the inspected older profiles or the exact producer zero footer for 3.2.1. The general compatible zero-terminated input profile accepts one or more complete all-zero 512-byte blocks after complete entries; no nonzero trailing bytes are permitted. Encrypted data volumes require the exact zero footer described above; optional nonfinal top-off is validated as an exact prefix of the original stored ciphertext bytes. Final encrypted volumes permit no top-off. Signature archives accept a standard zero-terminated tar (including empty) or an exactly zero-byte decoded signature stream. Unexpected trailing bytes and dangling extended headers are errors. Any missing required stored volume or incomplete multipart entry remains an error despite valid container EOF.

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

Provenance: SRC-DIFF, SRC-MAIN, official multipart fixtures. For the inspected 3.0.7 profile every nonfinal part has exactly 65536 payload bytes; the final part is shorter, including a zero-byte part after an exact multiple. Validate these lengths and sequential numbers against manifest boundary markers. The last normalized volume needs no pending EndingPath block, every required volume must exist and pass integrity, and complete container parsing must succeed. There is no explicit final-part flag in the emitted name; valid tar EOF alone never establishes completion.

### PARSE-MULTI-004

Assembly MUST preserve pending logical-entry state across an empty volume while validating its supported empty-boundary marker shape.

Provenance: SRC-MAIN get_indicies, SRC-DIFF remembered/previous indexes, actual official empty-tail output. Start is root/None; end is the previous actual decoded boundary or root/None when no prior entry exists. Empty data produces no operation or content and cannot satisfy a missing final short part. Exceptional restart-empty shapes remain unsupported until separately reviewed.

### PARSE-MULTI-005

Repeated ordinary operations for one path within a backup set MUST retain archive order for snapshot-state interpretation.

Provenance: POLICY consistent with PARSE-STATE. Repetition does not coalesce operations or reopen a completed multipart entry.

### PARSE-MULTI-006

Assembly MUST reject an ordinary/multipart representation mixture or a second multipart logical entry for one path within the initial supported backup-set profile.

Provenance: Conservative reader POLICY, not a universal producer-format claim. Sequential numbered parts of the same pending entry remain valid. Broader repeated multipart replacements require separate source/producer evidence, state semantics, and specification review.

## Librsync signatures

The signature header is three big-endian 32-bit unsigned fields: magic, block length, strong-checksum length. It is followed by complete records, each containing a 32-bit weak checksum and the indicated strong-checksum bytes. The final original source block can be shorter than the block length; its exact length is not encoded here.

Reviewed magic values: `0x72730136` (MD4), `0x72730137` (BLAKE2), `0x72730146` (RabinKarp/MD4), and `0x72730147` (RabinKarp/BLAKE2). Reading signature metadata does not require implementing backup signature generation.

### PARSE-SIG-001

The signature parser MUST recognize the four reviewed signature magic values and reject other magic values as unsupported.

Provenance: DOC-RSYNC, SRC-RSYNC.

### PARSE-SIG-002

The signature parser MUST validate nonzero block length, supported strong-checksum length for the selected magic, and complete checksum records.

Provenance: DOC-RSYNC, pinned SRC-RSYNC `sumset.h` and `checksum.c`. MD4 variants permit strong-checksum length 1..16; BLAKE2 variants permit 1..32. Zero block length and zero checksum length are rejected rather than treated as producer default requests. The payload after the 12-byte header must be an exact multiple of `4 + strong-checksum length`; zero records are valid for an empty source file.

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

Provenance: SRC-DIFF, SRC-PATCH and official 3.0.7 source/restore oracle. Snapshot records replace content/type/metadata at their path; replacement with a nondirectory also removes any descendant state. Directory snapshots preserve descendants unless explicitly deleted/replaced. Deletion removes the selected path and descendants; a later recreation is independent of former data. Diffs replace reconstructed regular content and update metadata from their own tar record. Unmentioned paths persist. Metadata-only regular changes can still encode a delta containing only copies; they are not absent changes. A resulting nonroot entry cannot have a missing/nondirectory ancestor. Apply all entries of a set before final ancestor consistency validation, because a nondirectory replacement can precede emitted descendant deletion records in component order.

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

This table summarizes bounded reviewed checkpoints at the restart specification checkpoint; the assurance ledger remains the authority for passing evidence, source identity, and residual obligations. No row establishes complete-reader acceptance. The verification agent owns the precise mathematical contracts and reviews the source author cannot establish alone.

| Goal | Requirements | Expected evidence | Current status |
| --- | --- | --- | --- |
| VG-PARSE | PARSE-GEN, PARSE-PATH, PARSE-NAME, PARSE-MAN, PARSE-TAR, PARSE-SIG | Refines reviewed byte grammar; bounded consumption and arithmetic; finite-input termination/panic freedom | Token/path/signature/tar helpers implemented with focused accepted proofs; complete component/controller obligations remain pending |
| VG-DELTA | PARSE-DELTA | Command decoder refinement and concatenation of literal/basis slices; source/output bounds | Accepted 535-obligation delta checkpoint with official delta integration evidence; does not establish complete-reader acceptance |
| VG-ASSEMBLY | PARSE-MULTI, PARSE-MAN-004..010 | Ordered concatenation, restart normalization, conservative volume selection, completion conditions | Restart normalization/component selection has an accepted 21-obligation checkpoint; ordinary multipart software is implemented and independently tested; queued-restart compatibility is specified but pending; controller/heap/completion proofs pending |
| VG-STATE | PARSE-CHAIN, PARSE-STATE | Unique contiguous chain and path-state transitions | Reviewed 3.0.7 contract; native implementation/proofs pending |
| VG-RESTORE | PARSE-PATH, RESTORE | Valid selected paths and destination mapping; filesystem contract review and integration tests | Planned |
| VG-INTEGRATION | PARSE-ENV, PARSE-SOURCE, PARSE-BROWSE | Trusted-adapter contract review, corruption/decryption/source-change tests | S3 adapter and native hashes/plain/gzip have tested checkpoints; initial SEIPDv1 contract reviewed; crypto/browsing and aggregate reader gates remain pending |

Official-producer scenarios: full and multiple incrementals; add/change/delete/recreate/type replacement; metadata-only changes; empty files/directories; symlinks; long and byte-valued paths; whitespace/quotes/backslashes; custom prefixes; short/legacy names where producers support them; gzip/raw/symmetric/public-key/signed objects; incompressible multipart data and deltas; restart manifests; missing signatures/volumes/chain links; all selected snapshot dates. Compare source trees and official restored trees for contents, paths, types, symlink targets, modes, and supported timestamps. Preserve producer version, exact command, environment, source-tree expectation, and archive digests.

Adversarial scenarios: every opcode and width including implicit literals and zero-length commands; truncation at every field boundary; reserved commands, wrong magic, missing/trailing END, out-of-bounds copies, integer overflow, wrong passwords/keys/signers, mutated digests, malformed quoting, invalid tar checksums, malicious path components, symlink escapes, destination races, multipart loss/reordering, object replacement during reads, resource exhaustion, cancellation, and interrupted restoration. Official rdiff provides independent delta test data; it is not a runtime dependency. S3 tests include pagination and source conditions; TUI tests check that display escaping does not alter path identity.

## Profile decisions and remaining implementation holds

The initial OPEN items below have reviewed reader decisions for the 3.0.7 profile. This releases specification holds for implementing that explicit profile; it does not confer native parser, proof, or historical-producer completion.

| Former question | Reviewed decision and evidence |
| --- | --- |
| OPEN-TIME | Real Gregorian dates and bounded zones/years; exact separator predicate; reject leap-second and normalization-only inputs; SRC-TIME and explicit policy |
| OPEN-NAMES | Exact six-kind long/short stem grammar, literal prefixes, optional partial marker on all stems, exact envelope suffix; correct the inconsistent upstream partial classification as a reader policy |
| OPEN-MANIFEST | Byte token grammar; Filelist before/after volumes; explicit duplicates/fields/hash policy; restart normalization; source plus observed streaming manifests |
| OPEN-TAR | Explicit POSIX/GNU/V7 interpretation, reviewed PAX subset, envelope-specific termination; source plus official gzip/raw-path/PAX observations |
| OPEN-MULTIPART | 65536-byte nonfinal/short final chunks, inclusive ranges, manifest continuation markers, required integrity; source plus split snapshots/deltas and encrypted boundary probe |
| OPEN-SIGNATURE | MD4 lengths 1..16/BLAKE2 lengths 1..32, nonzero blocks, complete records; immutable upstream assertions/constants |
| OPEN-STATE | Ordered snapshot/diff/delete transitions and ancestor consistency; source plus source-tree/official-restoration comparisons for all three backup dates |
| OPEN-ENVELOPE | Complete gzip members, per-member CRC/ISIZE, no trailing nonmember data; RFC 1952 and one-member official fixtures |
| OPEN-CRYPTO | Pinned pgp 0.21.0; authenticated SEIPDv1 only; explicit AES/key/S2K/compression/fingerprint policy; CheckFirst bounds and upstream trust recorded; RFC/API/source review plus original official GnuPG objects |
| OPEN-LIMITS | Visible configurable byte/count/depth defaults and running-total enforcement; policy informed by actual fixture sizes |

Remaining implementation/compatibility holds:

| ID | Unresolved question | Evidence and required next action | Affected requirements |
| --- | --- | --- | --- |
| OPEN-CRYPTO-ADDITIONAL | SEIPDv2/v6, additional algorithms/key policies and full certificate-validity semantics | Acquire matching official-producer evidence and review stricter adapter contracts before expanding the supported profile | PARSE-ENV-007..016 |
| OPEN-RESTART-FOOTER | Queued replay bypasses iterator byte-offset accounting, potentially changing encrypted and later-version plaintext footer lengths | Acquire official queued/footer-bearing output and independently review its exact container rules; do not accept arbitrary trailing bytes | PARSE-TAR-005, PARSE-MAN-009, PARSE-MULTI-003 |
| OPEN-HISTORICAL | Native acceptance of original 0.8.23/2.1.3/3.2.1 small plain corpora; historical encrypted/multipart shapes, additional legacy zones/metadata | Official plain chains and 0.8.23 short/UTC-legacy names are now source/restore checked. Run native comparisons and acquire encrypted/multipart cases before broader claims; do not generalize 3.0.7 constants | Historical compatibility and untested variants of every format subsystem |
| OPEN-TAR-ADDITIONAL | Additional supported PAX fields/GNU variants and empty signature containers have no official fixture coverage yet | Generate targeted official or primary-format fixtures before accepting their implementation/testing milestones | PARSE-TAR-003,005 and metadata fidelity |

Resolving a question updates the relevant normative text, provenance, proof obligations, and test scenarios. Broader compatibility claims require producer evidence for the claimed release. Duvet snapshots and README completion records are updated together after independent review.
