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
| DOC-PAX | [POSIX pax utility format](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/pax.html), accessible [POSIX.1p rendering](https://man7.org/linux/man-pages/man1/pax.1p.html) | Extended-header record framing and byte-counted decimal record length |
| DOC-PGP | [RFC 9580](https://www.rfc-editor.org/rfc/rfc9580), historical [RFC 4880](https://www.rfc-editor.org/rfc/rfc4880), [RFC 6637](https://www.rfc-editor.org/rfc/rfc6637) | Packet integrity, composition, v4 keys and legacy Curve25519 ECDH |
| DOC-RPGP | [rPGP 0.21.0 API](https://docs.rs/pgp/0.21.0/pgp/), [pinned implementation status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/IMPL_STATUS.md), [security status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/SECURITY_STATUS.md) | Native adapter capability and trusted upstream limitations |
| DOC-LINUX-RENAME | [Linux rename(2)](https://man7.org/linux/man-pages/man2/rename.2.html) | Directory-relative no-replace publication, cross-parent directory permissions, NFS ambiguity |
| DOC-LINUX-ACL | [Linux acl(5)](https://man7.org/linux/man-pages/man5/acl.5.html) | POSIX ACL mask/mode correspondence and inherited access/default ACLs |
| DOC-LINUX-TIME | [Linux utimensat(2)](https://linuxman7.org/linux/man-pages/man2/utimensat.2.html) | Exact timestamp setting, filesystem rounding and no-follow symlink operations |
| DOC-LINUX-LINK | [Linux symlink(7)](https://man7.org/linux/man-pages/man7/symlink.7.html) | Fixed Linux ordinary symlink mode and link handling |
| DOC-LINUX-FS | [Linux statfs(2)](https://man7.org/linux/man-pages/man2/fstatfs.2.html) | Anchored filesystem identification |
| DOC-LINUX-XATTR | [Linux removexattr(2)](https://man7.org/linux/man-pages/man2/removexattr.2.html) | FD-based inherited ACL removal and error meanings |
| DOC-RUSTIX | [rustix 1.1.5](https://docs.rs/rustix/1.1.5/rustix/) | Safe directory-relative syscall wrappers; upstream/kernel filesystem trusted boundary |
| DOC-RUST-ARGS-OS | [Rust args_os](https://doc.rust-lang.org/std/env/fn.args_os.html) | Original OS argument values without Unicode validation |
| DOC-RUST-OSSTR | [Unix OsStrExt](https://doc.rust-lang.org/std/os/unix/ffi/trait.OsStrExt.html) | Original Unix path bytes |
| DOC-RUST-THREAD | [Rust thread ownership](https://doc.rust-lang.org/std/thread/fn.spawn.html), [bounded channels](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) | Immutable worker ownership, joining and channel backpressure |
| DOC-CROSSTERM-KEYS | [crossterm 0.29.0 key kinds and reporting limits](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.KeyEvent.html#structfield.kind) | Reported event kinds and legacy terminal repeat limitations |
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

Official Linux kernel source fallback, inspected on2026-10-11 because syscall documentation does not establish late local-error rollback: Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449`, tag object `f7b88edb52c8dd01b7e576390d658ae6eef0e134`. SRC-LINUX-EXT4 is [fs/ext4/namei.c](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/fs/ext4/namei.c), SHA256 `9f8d5fcbb7ca3105f514244ed0a90d8539b95a9afe3771cc2c55424e1fedd864`; SRC-LINUX-VFS is [fs/namei.c](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/fs/namei.c), SHA256 `22a87d724eb43ece32f77c144fdee6e89180446a54f0c3bfef023fc8016a6d33`. Creation/rename can modify entries before reporting late errors; VFS movement can then be skipped. See [the native Linux profile](restore-linux.md) for functions/locations and conservative uncertainty handling. A local filesystem allowlist is not an error-means-no-change proof.

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

Configured global and kind-specific filename prefixes MUST be matched literally in that order before the object stem. Volume kinds MUST select the archive prefix, manifest kinds MUST select the manifest prefix, and signature kinds MUST select the signature prefix.

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

### PARSE-NAME-008

For a valid parser configuration, the reader MUST return the name-size limit error when a basename exceeds its configured byte limit, before checking basename contents. At or below that limit, the reader MUST reject a basename containing NUL or `/` with the malformed-name error.

Provenance: POLICY. This specifies preflight error precedence and does not define configuration-validation precedence or the subsequent filename classification.

### PARSE-NAME-009

Before checking basename size or bytes, the reader MUST validate the parser configuration. The time separator MUST be in the valid set in PARSE-NAME-007. Configured global, archive, manifest, and signature prefixes MAY be empty and otherwise contain arbitrary bytes, but MUST NOT contain NUL or `/`. Invalid configuration MUST return the configuration error before basename limit or malformed-name errors; valid configuration MUST proceed to the checks in PARSE-NAME-008.

Provenance: PARSE-NAME-007 defines the time-separator set. Rejecting NUL and `/` in configured prefixes, and configuration-error precedence over input errors, are POLICY choices.

### PARSE-NAME-010

The body parser MUST return `Malformed` for body parse failures. It MAY return `Configuration` only when the configured time separator is invalid, and MUST NOT return `Limit` or `Ambiguous`.

Provenance: POLICY. The body parser receives an already-stem-selected candidate and does not enforce basename limits or resolve conflicts among candidates. Long timestamp helpers may inspect the configured separator; all body syntax and field failures use the malformed-name error.

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

### PARSE-ENV-017

OpenPGP volume integration MUST validate every supported manifest digest against the exact original stored ciphertext before invoking decryption for that volume.

Provenance: PARSE-ENV-005/006, SRC-HASH, SRC-MAIN and original encrypted fixture checks. Read through the original catalog identity, conditional source checks, exact declared size and real EOF; ciphertext never substitutes its literal plaintext for a stored-object digest. Missing/conflicting/unsupported or mismatching required digests fail without entering the volume decoder. Resource/cancellation rejection may precede integrity work. SHA1/MD5 compatibility digests still do not authenticate an unsigned manifest.

### PARSE-ENV-018

OpenPGP manifest integration MUST complete supported message integrity, framing, decoding and configured signer checks before parsing the manifest or using its volume records.

Provenance: PARSE-ENV-003/007..016. Manifests have no enclosing duplicity volume digest; do not invent one. Source exact-size/identity/EOF checks and the adapter's complete SEIPDv1 MDC are required, plus document-signature authorization when configured. Bound the final manifest literal by both the crypto decoded-byte bound and the reader's decoded-manifest bound; intermediate compression/authenticated/key bounds remain independent. An unsigned encrypted manifest can supply volume hashes after MDC checks when caller policy permits it, but confers no signer-identity claim.

### PARSE-ENV-019

A snapshot read with signer authorization configured MUST require that authorization for every required manifest and data volume throughout the selected full/incremental chain, rejecting unsigned or plain/gzip required objects without policy downgrade.

Provenance: PARSE-ENV-004/012, POLICY. Authorization is configured when `require_signature` is true or permitted full primary fingerprints are nonempty; required verification with an empty permitted list still fails closed. The same supplied policy applies to every required object, including mixed-envelope chains; no per-object omission or implicit switch to integrity-only recovery is permitted. Unused catalog artifacts and signature archives not read for reconstruction are outside this scope. A nonempty allowlist can permit different authorized primary signers on different required objects; this is not a same-signer or current-validity claim.

### PARSE-ENV-020

Reader and multipart integration MUST admit decrypted data only through a private path that itself decodes the original stored object under the supplied policy and applies its correct envelope-specific container rules.

Provenance: PARSE-TAR-005, PARSE-MAN-009/010, POLICY. No public plaintext-plus-verified flag, externally constructible receipt/decoded object, arbitrary decoder callback or caller-supplied authentication assertion may bypass required source/digest/crypto checks. Existing public plain/gzip assembly stays safe and does not accept OpenPGP as decoded plaintext. Encrypted tar receives its exact original stored ciphertext for footer/top-off comparison and final/nonfinal status from the complete normalized manifest volume sequence. Never use `Termination::Data` as an encrypted fallback. Encrypted queued-restart StartingPath exceptions remain unsupported; they do not inherit the raw/gzip no-footer compatibility exception.

### PARSE-ENV-021

The reader integration MUST borrow caller-supplied credential and authorization inputs without cloning secret exports or passphrases into catalog objects, object names, snapshots, evidence receipts, diagnostics or logs.

Provenance: PARSE-ENV-009, POLICY. A per-read borrowed context can rebuild the adapter's reference-only Credentials value for each object; the supplied password/key slices remain owned by the caller. Credential discovery, secret-storage lifetime, worker credential acquisition and secure erasure are separate caller/frontend work. Parsing/unlocking and crypto-library temporary buffers remain the separately reviewed adapter boundary, not a guarantee of zero internal key copies or secure erasure.

### PARSE-ENV-022

Snapshot authentication evidence MUST have explicit per-read object-count, retained-metadata, certificate-version and comparison-work limits, checked before its allocation or retention.

Provenance: PARSE-GEN-005, POLICY. Initial defaults are 1,000,000 required-object records, 32 MiB of retained evidence capacity/name/identity/metadata, 64 distinct certificate metadata versions and 16 million evidence comparisons. Each manifest/data-volume record is tied to its original source identity and set/role/ordinal, with no unbounded key/name or repeated certificate clone. Intern identical certificate metadata once per read; equality includes full primary fingerprint, creation and the complete ordered signature-metadata sequence, not fingerprint alone. Distinct versions remain distinct or fail the version bound rather than losing expiry/revocation observations. Borrowed ownership may replace copies only when the returned snapshot retains the owner's lifetime/identity safely. Per-object adapter buffers and existing catalog/state/assembly limits remain separately charged; no global heap or crypto-latency proof follows.

### PARSE-ENV-023

Snapshot authentication evidence MUST distinguish integrity-only objects from configured signer authorization and preserve available complete signer fingerprints and certificate creation/expiry/revocation observations without implying current validity or Web-of-Trust acceptance.

Provenance: PARSE-ENV-012/013, native adapter contract. Plain/gzip records have no OpenPGP integrity/signer claim. Successful OpenPGP objects have complete MDC checks; `authorized_signer = None` when policy was unconfigured is not proof the object was unsigned. Configured success means every presented document signature passed under a permitted certificate; the adapter's first authorized primary/component fingerprint is representative, not an exhaustive signer list. Retain certificate metadata through bounded pool references. Expiry fields retain their duration/absence interpretation and revocation fields remain observations for caller assessment, not an effective expiry or revocation policy decision. Packet filenames/timestamps and short issuer hints grant no object/destination identity or signer authority. Literal signatures do not attest filename-derived backup dates, key spelling, intended collection or freshness. Redacted evidence contains no key exports, passwords, session keys or plaintext payloads.

### PARSE-ENV-024

The reader MUST return a completed immutable snapshot and its authentication evidence only after every required manifest, normalized data volume, container, multipart completion and state transition succeeds under one unchanged policy and final cancellation check.

Provenance: PARSE-CHAIN/STATE/MULTI, PARSE-ENV-014, POLICY. Evidence and decoded entries remain provisional until complete-chain success; a late incremental, integrity, signer, source-identity, footer, bound or cancellation failure returns neither a snapshot nor a successful partial-chain receipt. Scan every normalized required volume even for restart-rewritten manifests; encrypted compatibility holds cannot be bypassed by range hints. Observe cancellation before/after source and native-crypto work, per required object/set and before return; no fixed latency or interruption of a blocking cryptographic derivation is promised. The reader performs no filesystem publication, and subsequent restore/UI layers retain their independent publication contracts.

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

For a successful PAX extended-header stream, the reader MUST process records in input order and accumulate each supported, unique, NUL-free key/value pair into its assigned one of the eleven metadata slots. An empty value remains present in its slot as a deletion marker for later global/local resolution. Existing field-specific requirements govern value grammars.

For each PAX extended-header stream, the reader MUST process records in input order and report the first failing record's error class using framing before key support, key support before duplicate/NUL admission, and admission before value syntax: malformed framing or duplicate/NUL values for a supported key classify as Pax, unsupported keys (including those with NUL values) and unsupported nonempty hdrcharset values as UnsupportedPax, and invalid decimal/time values as Numeric; empty values are accepted, and no later record changes the first error.

This rule governs only the PAX byte stream passed to ingest; global/local extension timing and complete tar-controller error behavior are specified separately.

### PARSE-TAR-004

Unsupported tar extensions or entry encodings that affect interpretation MUST produce an explicit diagnostic.

Provenance: POLICY. An unknown encoding cannot be skipped by guessing its size or path semantics.

### PARSE-TAR-005

The tar parser MUST enforce the supported envelope-specific termination profile rather than treating an incomplete header or payload as a valid object boundary.

A local PAX or GNU long-name/link override MUST require a following real entry, while a global PAX header alone MUST NOT make EOF dangling. A real entry clears pending local PAX or GNU long-name/link override state only after the entry is accepted. A dangling local PAX or GNU long-name/link override MUST produce `DanglingOverride` at EOF.

Provenance: SRC-HASH (`GzipWriteFile`, `GPGWriteFile`), SRC-DIFF, SRC-HIST-321, official 0.8.23/2.1.3/3.0.7/3.2.1 observations. Raw/gzip data volumes accept complete-entry EOF for the inspected older profiles or the exact producer zero footer for 3.2.1. The general compatible zero-terminated input profile accepts one or more complete all-zero 512-byte blocks after complete entries; no nonzero trailing bytes are permitted. Encrypted data volumes require the exact zero footer described above; optional nonfinal top-off is validated as an exact prefix of the original stored ciphertext bytes. Final encrypted volumes permit no top-off. Signature archives accept a standard zero-terminated tar (including empty) or an exactly zero-byte decoded signature stream. Unexpected trailing bytes and dangling extended headers are errors. Any missing required stored volume or incomplete multipart entry remains an error despite valid container EOF.

### PARSE-TAR-006

The tar parser MUST apply PAX extension state in header order. Each present field in a global `g` header replaces the prior global value, an empty global value deletes it, and omitted global fields persist. Each present field in a local `x` header replaces the pending local value, including empty values, while omitted local fields persist across later extension headers. Local fields MUST override global fields for the next real entry. That local state MUST be consumed only after the real entry is accepted. GNU `L`/`K` and PAX `g` headers MUST NOT consume pending local fields.

Provenance: DOC-TAR, SRC-TAR and observed GNU/PAX extension ordering. This requirement defines metadata state timing; validation of every possible tar-controller error outcome remains separate.

### PARSE-TAR-007

GNU and PAX extension headers MUST use the raw size in their own tar header to determine their payload extent. A real entry MUST use the effective PAX `size` value when present, or its raw tar header size when absent.

Provenance: DOC-TAR, SRC-TAR. Extension payload framing is independent of metadata overrides carried by preceding headers.

### PARSE-TAR-008

After selecting the raw or effective PAX size, the reader MUST return exactly that many payload bytes and advance the cursor over the padded extent to the next 512-byte boundary. It MUST reject configured entry-size excess and truncated extents with explicit errors.

Provenance: SRC-TAR and PARSE-TAR-002/007. The whole tar loop and its complete error-precedence relation remain separate.

### PARSE-TAR-009

When PAX path and GNU long-name overrides are absent, a POSIX ustar header MUST use its raw name field if the prefix is empty and MUST join a nonempty prefix to the raw name with one slash byte.

Provenance: DOC-TAR and SRC-TAR. This covers fallback header-name byte composition; NUL truncation, validation, and whole-entry controller refinement remain separate.

### PARSE-TAR-010

Raw tar text fields MUST end at the first NUL byte, or use the entire field when no NUL is present.

Provenance: DOC-TAR and the reviewed ustar header layout. This is byte truncation only; UTF-8 and path validation are specified separately.

### PARSE-TAR-011

Tar header mode MUST fit in `u32`, and header mtime MUST fit in `i64`; out-of-range values MUST be rejected as Numeric.

Provenance: SRC-TAR and the public tar header field widths. This narrows the already-decoded numeric values; octal/base-256 field decoding is specified separately.

### PARSE-TAR-012

The parser MUST choose PAX `path`, then GNU long-name, then the raw ustar prefix/name; it MUST compute that selected name's exact byte length and reject lengths above `max_path_bytes` before allocating the output name.

Provenance: SRC-TAR and PARSE-TAR-003/009. The actual output-byte composition, link limits, and complete tar controller are specified separately.

### PARSE-TAR-013

The parser MUST choose PAX `linkpath`, then GNU long-link, then the raw header linkname; it MUST compute that selected target's exact byte length and reject lengths above `max_path_bytes` before allocating the output link target.

Provenance: SRC-TAR and PARSE-TAR-003. This covers link-target source precedence, length, and allocation admission; path/name selection, target-byte validation, and whole tar-controller refinement are specified separately.

### PARSE-TAR-014

The parser MUST compute the exact byte lengths of the selected PAX `uname` and `gname` values, or their raw header fallbacks, and reject values above `max_path_bytes` before allocating the output owner names.

Provenance: SRC-TAR and PARSE-TAR-003. This covers the selected borrowed values' length and allocation admission; charset validation and whole tar-controller refinement are specified separately.

### PARSE-TAR-015

For each accepted tar entry, the output name MUST be the selected PAX path, GNU long-name, or raw ustar prefix/name, and its byte length MUST equal the precomputed length used for admission and metadata accounting.

Provenance: SRC-TAR and PARSE-TAR-009/012. This connects the selected source's byte composition to the existing checked length; path validation and complete tar-controller refinement remain separate.

### PARSE-TAR-016

For each accepted tar entry, the output link target MUST equal the selected PAX `linkpath`, GNU long-link, or raw header `linkname` bytes, and its byte length MUST equal the precomputed length used for admission and metadata accounting.

Provenance: SRC-TAR and PARSE-TAR-003/013. This connects the selected borrowed target to the copied entry value; NUL and semantic target validation and complete tar-controller refinement remain separate.

### PARSE-TAR-017

For each accepted tar entry, output `uname` and `gname` bytes MUST equal their selected PAX override or raw header fallback, and each output length MUST equal the precomputed length used for admission and metadata accounting.

Provenance: SRC-TAR and PARSE-TAR-003/014. This connects selected borrowed owner-name values to copied entry metadata; charset semantics and complete tar-controller refinement remain separate.

### PARSE-TAR-018

For each real tar entry, the reader MUST checked-add fixed entry overhead and the exact admitted output name, link-target, `uname`, and `gname` lengths, then charge that amount against the remaining metadata limit before allocating any output fields. Arithmetic overflow or an exceeded limit MUST reject the volume without returning a partial entry list.

Provenance: POLICY and PARSE-TAR-012 through PARSE-TAR-017. This proves the per-entry charge formula and admission boundary; cumulative controller refinement and extension-header accounting remain separate.

### PARSE-TAR-019

GNU `L`/`K` and PAX `x`/`g` extension headers MUST charge their raw-header payload size against the cumulative metadata limit before interpreting or retaining extension metadata. Arithmetic overflow or an exceeded limit MUST reject the volume without returning partial entries.

Provenance: POLICY and PARSE-TAR-007/018. The extension charge uses the raw extension-header size, not an effective PAX size; whole-controller accounting remains separate.

### PARSE-TAR-020

The reader MUST checked-increment the GNU/PAX extension-header count and reject the header if the increment overflows or exceeds `max_extended_headers`, before charging or interpreting extension contents.

Provenance: POLICY. This proves the extension-count transition and its parser ordering before extension metadata charging/content interpretation; whole tar-loop refinement remains separate.

### PARSE-TAR-021

Before allocating or retaining output fields for a real tar entry, the parser MUST require the accepted entry count to be below `max_entries`. If the count equals or exceeds the limit, it MUST return Limit without returning a partial entry list.

Provenance: POLICY. This proves per-entry count admission before output-field interpretation and allocation; whole tar-loop refinement remains separate.

### PARSE-TAR-022

At tar EOF, a pending local PAX/GNU override MUST produce DanglingOverride before the footer profile is checked. Otherwise, an invalid footer MUST produce Termination, and the parser MUST publish a TarVolume only when both checks succeed.

Provenance: PARSE-TAR-005 and the reviewed envelope termination profiles. This proves the local EOF decision and error precedence; whole tar-controller refinement remains separate.

### PARSE-TAR-023

GNU `L` and `K` payloads MUST contain a NUL terminator, all bytes from the first NUL onward MUST be zero, and the selected prefix length MUST NOT exceed `max_path_bytes`.

Provenance: DOC-TAR and SRC-TAR. Missing terminators or nonzero tail bytes are TextEncoding errors; a valid prefix over the configured bound is a Limit error. Complete tar-controller refinement remains separate.

### PARSE-TAR-024

After decoding the checksum field, the parser MUST reject a header whose checksum does not match before interpreting magic/version or any other numeric header field; that mismatch MUST produce Checksum.

Provenance: POLICY. This requirement captures the checksum admission gate and its parser-visible error precedence. Decoding the checksum field itself and refinement of the complete header controller remain separate.

### PARSE-TAR-025

After checksum admission, the parser MUST accept a header only when its eight magic/version bytes are exactly POSIX ustar (`ustar\0` followed by `00`), old GNU (`ustar ` followed by ` \0`), or eight zero bytes for V7. Any other magic/version byte sequence MUST produce UnsupportedHeader. Only POSIX ustar headers may interpret bytes 345 through 499 as a path prefix; old GNU and V7 headers MUST NOT use those bytes as a prefix.

Provenance: DOC-TAR and the reviewed tar header profile. This requirement fixes the accepted magic/version profiles and prefix interpretation gate; header checksum/value decoding and the remaining header-field interpretation are specified separately.

### PARSE-TAR-026

The reader MUST decode a tar numeric field as either padded ASCII octal or the reviewed GNU base-256 profile. For octal, it MUST trim only leading and trailing NUL or ASCII-space padding, interpret an all-padding field as zero, and accept only bytes `0` through `7` in the remaining digits. For base-256, it MUST recognize only a first byte of `0x80` (initial accumulator zero) or `0xff` (initial accumulator negative one), then fold each following byte in big-endian base 256. Every multiply and addition MUST be checked against the `i128` range; unsupported markers, invalid octal digits, or overflow MUST produce Numeric. Fields used as unsigned values MUST additionally reject negative results and results greater than `u64::MAX`; signed mtime narrowing remains governed by PARSE-TAR-011.

Provenance: DOC-TAR and SRC-TAR, with the project’s explicitly reviewed numeric compatibility profile above. This requirement covers scalar byte decoding and unsigned admission; it does not broaden the GNU base-256 marker set, define field-specific narrowing, or refine full header error ordering.

### PARSE-TAR-027

After checksum and header-profile admission, the parser MUST admit raw tar `size`, `uid`, `gid`, `device_major`, and `device_minor` fields as nonnegative `u64` values. A negative decoded value or a value greater than `u64::MAX` MUST produce Numeric before payload-extent selection or output-metadata retention. This raw-field admission MUST still occur when an effective PAX `size` supplies a real entry's payload extent; GNU/PAX extension payload extents continue to use their raw header size as specified by PARSE-TAR-007.

Provenance: DOC-TAR, SRC-TAR, and the reviewed tar header profile above. The public format defines these numeric header fields and PAX size replacement; the official reader profile decodes and admits raw header values before applying semantic overrides. Mode/mtime narrowing, PAX value decoding, and complete header-controller error ordering remain separate.

### PARSE-TAR-028

Before comparing a tar header checksum or interpreting magic/version and other numeric header fields, the parser MUST decode the checksum field using the numeric profile in PARSE-TAR-026 as a nonnegative `u64`. If the checksum field is malformed, negative, or outside that range, the parser MUST return Numeric. A successfully decoded checksum that does not match the header MUST continue to return Checksum as specified by PARSE-TAR-024.

Provenance: DOC-TAR and SRC-TAR establish the checksum field's numeric encoding; POLICY assigns reader-visible error classification and precedence. This requirement covers checksum-field decoding only; checksum summation and mismatch precedence remain specified by PARSE-TAR-024.

### PARSE-TAR-029

For each real tar entry, the reader MUST use the effective PAX `mtime` value when present; otherwise it MUST use the decoded raw header mtime seconds with zero nanoseconds. A present but invalid PAX `mtime` MUST produce Numeric and MUST NOT fall back to the raw header timestamp.

Provenance: DOC-TAR and SRC-TAR establish PAX mtime override semantics; the reviewed profile defines signed fractional-second decoding. Raw header mtime decoding and `i64` narrowing remain specified by PARSE-TAR-011, PAX value grammar remains separate, and this requirement does not refine extension-state selection or the complete tar controller.

### PARSE-TAR-030

For each accepted real tar entry, the reader MUST preserve the decoded raw header `mode` after the `u32` admission required by PARSE-TAR-011. A PAX `mode` key MUST be rejected as UnsupportedPax; it MUST NOT replace the raw header mode.

Provenance: DOC-TAR and SRC-TAR define the raw mode field; the supported PAX key profile and reader policy exclude `mode` as a semantic override. Numeric decoding, mode-width admission, and the complete tar-controller error ordering remain specified separately.

### PARSE-TAR-031

For each accepted character-device or block-device entry, the reader MUST preserve the decoded raw header `device_major` and `device_minor` values in the corresponding `TarEntry` fields without normalization or substitution.

Provenance: DOC-TAR defines the public `devmajor` and `devminor` metadata attributes; SRC-TAR confirms their mapping from tar header fields. Numeric decoding and unsigned admission remain specified by PARSE-TAR-026/027. This requirement does not require restoration or creation of special filesystem objects and does not refine whole-entry controller behavior.

### PARSE-TAR-032

For each accepted real tar entry, the reader MUST set `TarEntry.uid` and `TarEntry.gid` from the resolved effective PAX `uid` and `gid` values when present; when either effective value is absent, the reader MUST retain that field's decoded raw header value. Invalid present effective values MUST produce Numeric rather than falling back to the raw header value.

Provenance: DOC-TAR and SRC-TAR establish PAX owner-ID override semantics; the reviewed profile defines local/global resolution and empty-value deletion. Raw header decoding/admission remain specified by PARSE-TAR-026/027, PAX decimal grammar and local/global resolution remain separate, and this requirement does not refine complete header-controller behavior.

### PARSE-TAR-033

For each accepted real tar entry, `TarEntry.entry_type` MUST correspond to its raw type flag: NUL or `0` MUST map to regular, `1` hardlink, `2` symlink, `3` character device, `4` block device, `5` directory, and `6` FIFO. The reader MUST reject a non-regular entry when its effective payload size is greater than zero and MUST reject every other ordinary-entry type flag as `UnsupportedEntry`. GNU/PAX extension type flags are dispatched separately under PARSE-TAR-003/006/007 and are outside this mapping.

Provenance: DOC-TAR defines the standard type flags; the reviewed reader profile selects the supported subset and the zero-payload rule for non-regular entries. Complete tar-controller refinement remains separate.

### PARSE-TAR-034

After numeric decoding and raw/effective payload-size selection for an ordinary entry, the parser MUST validate the bounded payload extent before classifying its type flag. If the extent exceeds `max_entry_bytes` or is truncated, the parser MUST return `Limit` or `Truncated`, respectively, even when later classification would reject the flag or its nonempty payload as `UnsupportedEntry`. Type classification MUST occur only after the selected extent is valid.

Provenance: PARSE-TAR-001/007/008 and POLICY. This fixes reader-visible precedence between ordinary-entry extent admission and type admission; prior checksum/header/numeric validation and GNU/PAX extension dispatch remain separate.

### PARSE-TAR-035

For each real tar entry, the reader MUST set `TarEntry.atime` and `TarEntry.ctime` from their respective resolved effective PAX values when present, using the reviewed signed PAX timestamp profile. When an effective value is absent, the corresponding output field MUST be `None`. A present but invalid timestamp MUST produce Numeric; it MUST NOT be treated as absent. An empty local value MUST delete the corresponding inherited global value, so that field MUST be `None` for the next real entry.

Provenance: DOC-TAR, SRC-TAR, and the reviewed PAX profile above. PAX timestamp grammar and global/local state resolution remain specified separately; this requirement covers only output metadata selection and conversion, not extension-state timing or full tar-controller refinement.

### PARSE-TAR-036

After PAX global/local state resolution, an effective `size`, `uid`, or `gid` override MUST be a nonempty sequence of ASCII decimal digits representing a value in `0..=u64::MAX`; leading zeroes and zero MUST be accepted. A selected nonempty value containing signs, whitespace, nondigits, or exceeding `u64::MAX` MUST produce Numeric and MUST NOT fall back to the raw header field. An empty PAX record value remains a deletion marker under PARSE-TAR-006; when deletion leaves no effective override, the parser MUST use the raw header fallback. GNU/PAX extension payloads MUST continue to use their raw header size.

Provenance: DOC-TAR and SRC-TAR define these PAX fields as nonnegative decimal values; the reviewed reader profile and POLICY require checked `u64` admission and an explicit Numeric diagnostic. PAX record framing and global/local state selection remain specified by PARSE-TAR-003/006, and raw tar numeric decoding remains specified by PARSE-TAR-026/027. This requirement covers decimal conversion after effective-state resolution and real-entry fallback behavior, not extension-state timing or full tar-controller refinement.

### PARSE-TAR-037

Each resolved nonempty PAX `mtime`, `atime`, or `ctime` value MUST consist of an optional single `+` or `-`, one or more ASCII decimal integer digits, and an optional `.` followed by one or more ASCII decimal fraction digits. The integer magnitude MUST fit in `u64`. A fraction with at most nine digits MUST be interpreted exactly as nanoseconds by right-padding with zeroes; additional fraction digits MUST all be zero, or parsing MUST fail rather than lose precision. The sign MUST apply to the complete value, which MUST be normalized to `(seconds: i64, nanoseconds: u32)` with `0 <= nanoseconds < 1,000,000,000`; values outside that normalized range MUST produce Numeric. Empty integer/fraction components, malformed bytes, multiple signs or dots, and numeric overflow MUST produce Numeric.

Provenance: DOC-TAR, SRC-TAR, and the reviewed PAX timestamp profile above. This specifies value conversion after PAX state resolution; absent/deleted values, field selection, and global/local timing remain governed by PARSE-TAR-006/029/035, and complete tar-controller error ordering remains separate.

### PARSE-TAR-038

For an ordinary real entry, after checksum, header-profile, and raw numeric-field admission, effective payload-size selection, payload-extent validation, and ordinary type admission, the parser MUST enforce `max_entries` before interpreting or allocating the selected output name. If the number of previously accepted real entries is equal to or greater than `max_entries`, the parser MUST return Limit even when later selected-name interpretation would fail validation. This check MUST NOT precede the listed header, size-selection, extent, or type checks; extension-header count and metadata checks remain governed separately.

Provenance: POLICY and PARSE-TAR-021/034. This fixes the observable boundary between ordinary-entry admission and selected-name interpretation. It does not specify the ordering of raw header fields that are decoded before the count check or of other output metadata interpreted afterward.

### PARSE-TAR-039

For a successfully parsed tar stream, `TarVolume.entry_stream_bytes` MUST equal the byte offset immediately after the final complete tar record, including its header, payload, and padding, and before any accepted zero footer or encrypted top-off. When the input ends exactly after complete records with no footer, the field MUST equal the input length. When a zero footer is present, the field MUST point to its first byte; validated footer and top-off bytes MUST NOT be included.

Provenance: PARSE-TAR-005 and POLICY. The offset separates parsed entry records from envelope termination bytes for encrypted-volume top-off checks and producer compatibility; footer validation and ciphertext comparison remain separately specified. This does not accept incomplete records or alter footer profiles.

### PARSE-TAR-040

Each PAX extended-header record MUST begin with a nonempty ASCII decimal length field followed by one ASCII space. Leading zeroes are accepted by this reader profile; no canonical no-leading-zero form is required. The decoded length MUST be positive, fit in `u64`, and identify the complete record byte count starting at the first length digit and ending immediately after that record's LF, including the length field and all delimiters. The declared span MUST be wholly present in the payload and its final byte MUST be LF; the first `=` within the record body MUST delimit the key and value. Consecutive records MUST be consumed contiguously, with no skipped or unconsumed bytes. A zero, malformed, overflowing, out-of-bounds, or delimiter-inconsistent declared span MUST be rejected as malformed PAX framing under PARSE-TAR-003.

Provenance: DOC-PAX, PARSE-TAR-003, and POLICY. This specifies length interpretation and exact record boundaries; supported key selection, duplicate/NUL admission, value grammar, and first-error ordering remain governed by PARSE-TAR-003.

### PARSE-TAR-041

For a real entry, each present resolved PAX `uname` and `gname` value MUST be validated as UTF-8 when the effective `hdrcharset` is absent or selects a supported UTF-8 token (`UTF-8` or `ISO-IR 10646 2000 UTF-8`). Invalid bytes MUST produce TextEncoding. When the effective `hdrcharset` is `BINARY`, PAX owner-name values MUST be preserved byte-for-byte without UTF-8 validation. When a resolved PAX owner-name value is absent, the corresponding raw-header fallback MUST be preserved byte-for-byte and MUST NOT be validated under the PAX charset.

Provenance: DOC-PAX, PARSE-TAR-003/004/017, and POLICY. This covers charset validation of resolved PAX owner-name values and raw-header fallback; owner-name selection/length, supported charset-token admission, empty-value deletion, and global/local timing remain separately specified.

### PARSE-TAR-042

For a real entry, each nonempty resolved PAX `path` and `linkpath` value MUST be validated as UTF-8 when the effective `hdrcharset` is absent or selects a supported UTF-8 token (`UTF-8` or `ISO-IR 10646 2000 UTF-8`). Invalid bytes MUST produce `TextEncoding`. When the effective `hdrcharset` is `BINARY`, selected PAX `path` and `linkpath` values MUST be retained byte-for-byte without UTF-8 validation. When a resolved PAX path or link override is absent, including after an empty-value deletion, the next selection source (GNU long-name/long-link or raw header field) MUST remain byte-preserving and MUST NOT be validated under the PAX charset. This requirement does not relax NUL rejection for PAX records.

Provenance: DOC-PAX, PARSE-TAR-003/006/009/010/012/013, and POLICY. This specifies charset treatment only; path/link selection, length admission, NUL rejection, PAX deletion timing, and GNU/PAX extension ordering remain governed separately. Integration evidence exercises PAX `path` and `linkpath` under UTF-8 and BINARY policies, plus invalid raw-header and GNU long-link fallback bytes under UTF-8 policy.

### PARSE-TAR-043

The reader MUST recognize exactly these PAX key byte strings and map them to their corresponding semantic fields: `path`, `linkpath`, `size`, `mtime`, `uid`, `gid`, `uname`, `gname`, `atime`, `ctime`, and `hdrcharset`. Any other key MUST produce `UnsupportedPax`, including a key defined by POSIX that is outside this reader profile. Key recognition MUST NOT normalize or case-fold input bytes. Empty values for recognized keys remain deletion markers under PARSE-TAR-003/006; this requirement does not change duplicate-key, NUL, or value-grammar handling.

Provenance: DOC-PAX, PARSE-TAR-003/006, and POLICY. This fixes the accepted key-to-field map and unsupported-key outcome; record framing, duplicate handling, and value interpretation remain separately specified.

### PARSE-TAR-044

Each nonempty PAX `hdrcharset` record value MUST exactly equal one of the ASCII byte strings `BINARY`, `UTF-8`, or `ISO-IR 10646 2000 UTF-8`. Any other nonempty value MUST produce `UnsupportedPax`; matching MUST be byte-exact and MUST NOT trim, case-fold, or otherwise normalize the value. An empty `hdrcharset` value remains a deletion marker under PARSE-TAR-006 and MUST NOT be rejected as an unsupported token.

Provenance: DOC-PAX, PARSE-TAR-003/006, and POLICY. This defines token admission for nonempty `hdrcharset` values; global/local selection, deletion timing, and the charset behavior applied to owner/path/link values remain specified separately.

### PARSE-TAR-045

For each GNU L/K or PAX x/g extension header, after checksum, header-profile, and raw numeric-field admission and selection of the raw extension size, the parser MUST validate the bounded raw payload extent before enforcing `max_extended_headers`. A truncated extension payload MUST therefore produce `Truncated` even when the configured extension count is already exhausted; an extent exceeding `max_entry_bytes` MUST produce `Limit`. Once the complete bounded extent is admitted, the parser MUST enforce the cumulative extension count before metadata charging or interpreting extension contents.

Provenance: POLICY and PARSE-TAR-007/008/020. This specifies the relative order of raw extension extent, extension-count, metadata-charge, and content admission; it does not alter the error classes for those stages or refine the complete tar controller.

### PARSE-TAR-046

After checksum and header-profile admission, the parser MUST decode and admit the raw tar header `mtime` as a signed `i64` even when an effective PAX `mtime` supplies the real entry's output timestamp. A malformed raw numeric field or a decoded value outside the `i64` range MUST produce Numeric; a valid PAX override MUST NOT mask that raw-header error. This requirement does not change which timestamp is selected for output under PARSE-TAR-029.

Provenance: DOC-TAR, SRC-TAR, and the reviewed tar header profile. The raw header remains a numeric tar header field independently of the semantic PAX override. PARSE-TAR-011 specifies signed narrowing and PARSE-TAR-029 specifies output selection; this requirement explicitly covers raw-field admission under override. It does not refine error ordering among other malformed header fields or the complete tar controller.

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

### PARSE-STATE-003

`SnapshotState::entries()` MUST enumerate retained entries in bytewise lexicographic order of their backup-relative path components. Each component MUST be compared as an unsigned byte sequence using lexicographic byte order, with a proper prefix ordered before the longer sequence. Paths MUST be compared as sequences of components using the same lexicographic rule, with a proper component-sequence prefix ordered before the longer path. Component boundaries MUST remain significant; the ordering MUST NOT be derived by comparing slash-joined or rendered path strings.

Provenance: POLICY. A stable public iteration order makes listings and callers deterministic while preserving the reader's raw-byte path identity. Verification may establish the project comparator contract and native tests may exercise the standard map's observable iteration; this requirement does not claim proof of Rust's standard-library map internals.

### PARSE-STATE-004

A full set MUST contain only logical entries whose operation is `Snapshot`. If any entry in a full set has operation `Diff` or `Deleted`, the reader MUST reject the full set and MUST NOT return a `SnapshotState` for it.

Provenance: DOC-DUP, SRC-DIFF, SRC-PATCH, and official full-set fixtures. Full sets establish replacement state; delta and deletion operations belong to incremental state transitions. This requirement specifies admission only and does not prescribe a public error variant or prove the complete state-transition controller.

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

Cancellation observed before a publication attempt or a handled failure before that attempt MUST leave the restored tree unpublished at the requested destination and attempt removal of temporary plaintext from exclusively owned staging.

Provenance: POLICY. Cleanup failures are reported. Process crashes and power loss can leave private staging artifacts; the guarantee is not secure erasure or crash-proof cleanup. Any publication syscall error yields an indeterminate result without modifying cleanup (RESTORE-026); successful publication wins a racing cancellation request, and post-success wrapper cleanup warnings do not make the result an unpublished failure (RESTORE-021/RESTORE-023). Failed first outer staging-leaf creation is separately diagnosed (RESTORE-027).

### Initial native Linux filesystem profiles

Two profiles share the contracts below. `PrivateParentSameParent` uses an anchored parent with no group/other ordinary permission bits and stages an exclusive restore-root leaf directly there. It renames that leaf within the same parent and supports read-only root modes. `ProtectedParentWrapper` permits a searchable parent with no group/other write bits and stages `anchored-parent / exclusive-wrapper(0700) / root / selected entries`; it moves `root` from the wrapper FD to the parent, requiring root owner-write permission. In that profile the wrapper remains private through final root metadata application. A single staging root under a searchable parent cannot provide that privacy when its final mode allows other users to enter or write it.

The trusted boundary and pinned syscall-wrapper/kernel provenance are documented in [the native Linux profile](restore-linux.md). These requirements do not claim completed filesystem implementation or proof.

### RESTORE-007

The initial Linux backend MUST reject destination parents whose filesystem type is outside the reviewed local allowlist before creating staging.

Provenance: POLICY, DOC-LINUX-FS, DOC-LINUX-RENAME. The allowlist is `0xef53` (ext2/3/4 family), `0x9123683e` (Btrfs), `0x58465342` (XFS), and `0x01021994` (tmpfs), obtained through `fstatfs` on the anchored parent. A listed type is not a promise that every kernel/mount configuration supports the required operations or timestamp values. Unknown types, overlay, NFS, CIFS/SMB, FUSE and other remote/userspace filesystems are held. Even an allowed local filesystem can mutate an entry and then return a late error; the allowlist never establishes error-means-no-change (SRC-LINUX-EXT4/VFS). No permission-check-and-ordinary-rename fallback is permitted if `NOREPLACE` is unavailable. Native tests name the actual filesystem exercised, without claiming coverage of other families.

### RESTORE-008

The backend MUST reject a destination parent unless its opened directory is owned by the effective UID, has owner write/search permissions, and has neither group-class nor other-class write permission.

Provenance: POLICY, DOC-LINUX-ACL. Check the opened parent with `fstat`: directory type; `st_uid == geteuid()`; `(st_mode & 0300) == 0300`; `(st_mode & 0022) == 0`. An extended POSIX ACL's effective named-user/group write grants are bounded by its mask, represented by the group mode bits. Sticky bits do not substitute for these checks. Parent write/search denial discovered by a syscall is an error, not grounds for changing parent permissions. The parent pathname may contain user-selected symlinks during initial opening; all subsequent work stays relative to the fixed FD. The excluded same-UID/privileged writers can invalidate this policy and are not silently covered by an inode check followed by rename.

### RESTORE-009

The backend MUST keep every provisional restored entry behind its selected profile's private directory boundary until publication or cleanup.

Provenance: POLICY. Use bounded collision retries for an unpredictable staging leaf; never adopt an existing leaf. In `ProtectedParentWrapper`, an exclusive wrapper holds the newly created inner root, and creation mode, ACL removal and explicit mode initialization establish an effective-user-owned wrapper with ordinary mode `0700` before descendant creation, verified through its opened FD. Other UIDs may list a searchable parent's wrapper name but cannot traverse it. Never publish by moving that wrapper or expanding its permissions. In `PrivateParentSameParent`, the opened parent itself denies other UIDs traversal; its permissions are not changed, and the provisional root may safely acquire its exact final mode before same-parent publication.

### RESTORE-010

All operations on backup entries MUST use their original byte-valued single path components relative to retained directory FDs, without reopening a directory by a joined path or following a restored symlink.

Provenance: POLICY, DOC-RUSTIX, PARSE-PATH. Retain `OwnedFd` handles for the parent, wrapper when applicable, root, and every created directory through publication or cleanup; directory opens use `DIRECTORY | NOFOLLOW | CLOEXEC`. A lookup's leaf is exactly one nonempty component, excluding NUL, slash, `.` and `..`. No `AT_FDCWD`, backup-controlled absolute syscall path, display-decoded path, or `/proc/self/fd` path workaround is used. Initially created directories may need a protected, single-component `chmodat(..., 0700, empty flags)` before their readable FD can be opened when umask removed owner permissions; exclusivity and the trusted parent/private boundary establish that this leaf is the directory just created. The operation is not a general no-follow chmod primitive.

### RESTORE-011

The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.

Provenance: POLICY. Validate the destination leaf as one component distinct from the generated staging name; source-root metadata, if present, belongs to a directory. Regular entries have their complete immutable content. Symlink targets are nonempty bytes without NUL and are never interpreted as lookup paths; absolute, dangling and `..` targets may be preserved. Initial targets are limited to 4095 bytes and filesystem name bounds still apply. Reject any selected symlink whose ordinary mode differs from `0777`. Exact mtime seconds must fit the pinned backend's `Timespec` representation and nanoseconds must be below `1_000_000_000`. For `ProtectedParentWrapper` the effective restore-root ordinary mode must include owner write (`0200`) for movement between wrapper and parent; unsupported read-only root modes fail before staging, including an empty tree with supplied read-only root metadata. `PrivateParentSameParent` preserves read-only root modes because the containing parent does not change. The wrapper limitation is explicit, not permission normalization. Descendant directory modes may be `0000` or otherwise restrictive because cached FDs survive finalization.

### RESTORE-012

The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.

Provenance: POLICY, PARSE-GEN-005. Checked preflight sums include applicable wrapper and root overhead, journal/cache capacities, C-string terminators, and regular payload lengths. The FD budget accounts for the parent, applicable wrapper, root, all created directory handles and the active regular-file handle; existing descriptors elsewhere in the process can still cause a reported `EMFILE`. Retained descriptors cannot be replaced with insecure path reopening to satisfy a limit. Per-syscall partial-write/retry work is charged; exhausted forward work must not disable already reserved cleanup work. Ordinary kernel blocking and allocator/kernel internal storage are not claimed to be bounded by these budgets. The published interface documents exact defaults, charge units, and excluded borrowed snapshot storage before implementation acceptance.

### RESTORE-013

The backend MUST pre-reserve a journal state for every attempted creation and preserve possibly-created as well as definitely-created entries through handled failure cleanup.

Provenance: POLICY, SRC-LINUX-EXT4. Prepare the journal slot, parent-FD identity, original component, type, namespace-ownership state and capacity before the creation syscall. Arm `Attempted/PossiblyCreated` before that syscall; success records definite creation immediately without intervening allocation, callback, polling or other syscall. An error does not mean no entry was created. Inside an already exclusively owned private namespace, a unique planned leaf may be removed without following symlinks even if creation reported error; do not require an acquired child FD or a success-only journal bit. Failed first outer staging-leaf creation is handled by RESTORE-027 instead, because exclusive ownership was not established. Process termination can interrupt journal transitions and is outside the handled-cleanup guarantee.

### RESTORE-014

Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.

Provenance: POLICY, DOC-RUSTIX. Handle short writes by continuing at the exact unwritten offset; zero writes with remaining data are errors. Interruption retries observe cancellation and consume bounded work. Never append, truncate an existing file, reuse a preexisting leaf, or create a backup-controlled hard link. A creation umask may remove owner permissions; the returned file FD remains usable for writing and final mode operations. After content is complete, remove inherited access ACL, then apply ordinary final mode and exact mtime through the FD. No backup ownership or special mode bits are applied.

### RESTORE-015

Symlinks MUST preserve the exact supported target bytes and receive their exact mtime through a no-follow operation relative to their retained parent FD.

Provenance: POLICY, DOC-LINUX-LINK, DOC-LINUX-TIME. Create with `symlinkat`; set time with `utimensat(..., SYMLINK_NOFOLLOW)` and verify type, fixed ordinary mode `0777`, target bytes and time without dereferencing. No ownership restoration or symlink chmod is attempted. A link target outside the destination is preserved as link data; the restorer never follows it. Future consumer traversal is outside this creation-time containment contract.

### RESTORE-016

The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.

Provenance: POLICY, DOC-LINUX-ACL, DOC-LINUX-XATTR. Remove `system.posix_acl_access` and, for directories, `system.posix_acl_default` using anchored FDs. Remove each directory's default ACL before creating its children, including the wrapper when present. Verify attribute absence before final mode application and publication; an inherited named grant can otherwise regain effective access when final chmod expands the ACL mask. `ENODATA` from removal means absent; `EOPNOTSUPP` is accepted only under the reviewed local filesystem contract that POSIX ACLs are unavailable, with the corresponding absence/capability check. Other errors fail before publication. No source ACLs or unrelated xattrs are restored; security labels inherited under trusted system policy are outside backup metadata fidelity.

### RESTORE-017

Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.

Provenance: POLICY, DOC-LINUX-TIME. Final ordinary permissions are `source_mode & 0777`, without ownership, setuid, setgid or sticky restoration. Read back regular files/directories using `fstat` while their FDs remain available, and links with `statat(..., SYMLINK_NOFOLLOW)` relative to the cached parent; target read-back is separately bounded. Filesystem rounding, clamping, failed chmod or mismatched size is a prepublication fidelity error, not degraded success. Use `UTIME_OMIT` for atime; source atime/ctime, birth time, allocation layout, hard-link identity, ACLs/xattrs and ownership are not promised. A successful write syscall is not a claim of persistence after power loss.

### RESTORE-018

Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.

Provenance: POLICY. Child creation changes directory mtime. Finalization after creation preserves even non-searchable descendant directory modes without reopening them. When present, the wrapper remains `0700` and is never assigned source-root metadata; a private anchored parent is likewise never assigned source-root metadata. Finishing metadata and read-back before rename ensures publication cannot expose an unfinished tree. Moving the root updates the containing staging/published parent directory metadata and status metadata; it does not justify subsequent modification of restored mtime or ordinary mode.

### RESTORE-019

When a genuinely empty complete plan has no source-root metadata, the backend MUST publish an empty root with explicit policy mode `0700` without reporting an invented source-root mtime.

Provenance: POLICY, reviewed RestorePlan. Its actual creation-time mtime is an unspecified result, explicitly identified in the receipt; no original timestamp is claimed. A nonempty plan requiring missing source ancestor/root metadata is invalid rather than a reason to synthesize metadata.

### RESTORE-020

The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.

Provenance: POLICY, DOC-RUSTIX, DOC-LINUX-RENAME. For `ProtectedParentWrapper` the call is `renameat_with(wrapper_fd, root_leaf, parent_fd, destination_leaf, NOREPLACE)`; for `PrivateParentSameParent` it is `renameat_with(parent_fd, staging_root_leaf, parent_fd, destination_leaf, NOREPLACE)`. This rename is the publication linearization point. An existing file, directory, dangling symlink or any other destination entry is a conflict; never replace it. Unsupported rename flags fail, without fallback. After success, perform no fallible metadata operation or rollback on the published tree. Any publication syscall error enters the indeterminate state in RESTORE-026; even the reviewed local filesystem boundary does not establish that the destination was untouched or that the root remains exclusively in staging. Parent-FD anchoring, restrictive parent policy and the excluded writers are essential; an inode-check/rename pair alone is not a race-proof guarantee.

### RESTORE-021

Cancellation MUST be observed before each entry, each bounded write attempt and the final publication attempt, with successful publication taking precedence over a racing cancellation request.

Provenance: POLICY. Cancellation observed before the final publication decision triggers staging cleanup and an unpublished result. Once rename is attempted, an error yields publication uncertainty rather than an unpublished cancellation result. A request arriving after the final cancellation observation can race with rename; successful rename yields a published receipt, even if cancellation is now set. No postpublication cancellation result may claim the destination remained unpublished. This is observation-based cooperative cancellation, not a guarantee that the syscall cannot start after another thread sets a flag or that kernel I/O is interruptible within a fixed time.

### RESTORE-022

On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.

Provenance: POLICY, DOC-RUSTIX. First restore `0700` to cached created directories in parent-before-child order; an open parent FD does not bypass its search permission during relative unlink. Then remove entries in child-before-parent order using `unlinkat` with type-appropriate `REMOVEDIR`, finally removing root and any wrapper. A definitely or possibly created directory inside the owned private namespace whose FD could not be acquired remains journaled under its protected parent and may be removed as an empty directory there. RESTORE-027 restricts failed first outer-leaf cleanup. No such modifying cleanup runs after an indeterminate publication attempt. Continue independent cleanup attempts after an error; bounded diagnostics retain the original fault and all counted cleanup failures, indicating any diagnostic truncation. Never traverse `remove_dir_all`, enumerate unrelated parent entries, or remove the requested destination. Cleanup is an attempt, not secure erasure; crashes and fatal signals can leave artifacts.

### RESTORE-023

The result MUST distinguish unpublished failure/cancellation, published success, and publication-indeterminate outcomes, retaining post-success wrapper-removal faults as cleanup warnings on published success.

Provenance: POLICY. An unpublished error includes its initiating cause, cleanup outcome and any first-stage candidate uncertainty. A publication-indeterminate result identifies its attempted publication and syscall error with the pre-recorded parent/root identities and staging/destination names; it does not report a complete restore or an untouched/absent destination. Published success is recorded immediately when rename succeeds and cannot be converted to failure by a later allocation, cancellation, callback or wrapper cleanup fault; reserve the receipt's required storage beforehand. When a wrapper exists, cleanup after success touches only that now-empty wrapper, never the published root. The same-parent profile has no remaining staging entry to remove after publication. A successful publication provides completed-tree visibility and no replacement; a failed attempt can have uncertain effects. Neither outcome promises crash durability, secure deletion, protection after consumers access the published tree, or indefinite identity of the caller's textual parent path.

### RESTORE-024

Restoration assurance evidence MUST separate pure plan/controller guarantees from trusted syscall, rustix, kernel, filesystem and allocator assumptions.

Provenance: User assurance decision, ASSURE-PROOF/TRACE. Duvet implementation/test annotations attach to these stable headings after adoption; annotation presence does not establish passing evidence. Project restoration code forbids unsafe code and preserves the project's no-admission rules for any proved controller. Runtime integration and failure/race tests are required for the filesystem boundary; no existing parser/plan proof establishes syscall containment or atomic publication. Future proof goals include creation/journal correspondence, component/parent-FD mapping, charge-before-action, cleanup coverage, exact-write offsets, and publication/cancellation/indeterminate state transitions and no modifying cleanup after attempted-publication error. No filesystem/controller proof is claimed complete in this proposal.

### RESTORE-025

The backend MUST select and validate its publication profile before creating staging, without changing destination-parent permissions to qualify for either profile.

Provenance: POLICY, DOC-LINUX-ACL, DOC-LINUX-RENAME. Prefer `PrivateParentSameParent` when the common parent checks in `RESTORE-008` hold and `(parent_mode & 0077) == 0`; POSIX ACL group-mask absence also excludes effective named-user/group traversal. Otherwise use `ProtectedParentWrapper` only if its common parent checks and root-owner-write restriction hold. A caller may request a stricter explicit profile and receive a diagnostic if unsupported. The parent security permissions and trusted process/mount boundary remain valid throughout the operation; no claim covers hostile same-UID/privileged permission changes. Profile names appear in the successful receipt and native evidence so a public-parent wrapper result is not mistaken for tested read-only-root support.

### RESTORE-026

If a publication rename is attempted and returns an error, the backend MUST report publication uncertainty and refrain from modifying cleanup of the restore root, its descendants or its staging wrapper.

Provenance: POLICY, SRC-LINUX-EXT4/VFS. Initial policy treats every publication errno conservatively, including errors which a future reviewed recovery contract may classify more narrowly. Do not chmod, unlink, rename again, rollback, or remove staged names that might alias published inodes; retained FDs are not permission to modify a potentially published tree. Close/release FDs normally. Preserve reserved diagnostics, candidate names and recorded parent/root identities for future recovery. Read-only observations may be reported but cannot silently promote uncertainty to published success or authorize deletion: VFS cache and underlying namespace updates can disagree after a late error. This rule does not exclude metadata/I/O faults to preserve a convenient no-change assumption. Recovery and independently proved outcome resolution are a separate held profile.

### RESTORE-027

If creation of the first outer staging leaf returns an error other than a definite existing-entry collision, the backend MUST report its candidate as uncertain and leave that candidate untouched because exclusive ownership was not established.

Provenance: POLICY, SRC-LINUX-EXT4. Do not adopt, chmod, open for modification, remove, or retry/reuse an uncertain candidate on arbitrary creation errors. A definite `EEXIST` collision may be skipped with bounded selection of a different leaf, always leaving the existing entry untouched. Success, followed by private-boundary validation, establishes the owned namespace needed for descendant creation and attempted-entry cleanup under RESTORE-013/022. An outer creation error occurs before any backup plaintext descendant is written, but can leave a metadata artifact; neither a late errno nor an inode observation alone proves it is safe to delete a potentially preexisting entry. Caller-selected unrelated entries, including the requested destination, remain untouched.

## Native terminal restoration interaction

Initial scope: local filesystem restoration of an already reconstructed immutable plain/gzip snapshot on Linux, through the adopted native backend. Destination configuration is supplied at process startup; no editable terminal path or credential widget is required. These frontend requirements add no cryptographic, global-resource, syscall, or crash-recovery guarantees beyond their referenced core contracts.

### RESTORE-UI-001

The frontend MUST accept a restoration destination only as a complete pair of `--restore-parent PATH` and `--restore-name NAME`, preserving their original OS bytes and rejecting duplicate, missing, oversized or invalid destination values before starting restoration.

Provenance: POLICY, DOC-RUST-ARGS-OS, DOC-RUST-OSSTR. With neither option, browsing and read-only previews remain available. Either option alone is a configuration error, not an inferred default. Linux NAME is exactly one nonempty component, excluding NUL, slash, `.` and `..`; it can contain non-UTF-8 bytes, spaces, backslashes and option-like leading text. PATH is nonempty and contains no NUL. Initial byte limits are 65,536 for PATH and 4,096 for NAME, checked before frontend copies/escaping. OS argument acquisition itself remains standard-library/process infrastructure. Option names and S3 configuration values require explicit valid UTF-8 where their API requires it; never convert all arguments through Unicode or decode destination escapes. No automatic restoration occurs at startup. Destination eligibility remains the backend's decision.

### RESTORE-UI-002

The frontend MUST derive a restoration preview from the exact completed immutable snapshot and selected original byte-valued roots, including precisely the entries and source directory ancestors accepted by its `RestorePlan`.

Provenance: RESTORE-001/005, reviewed RestorePlan. Marked roots select their subtrees; no marks explicitly means the whole snapshot. Deduplicate overlapping selections through the core plan. Preserve snapshot contents/metadata by immutable ownership; do not reconstruct a newer snapshot, replace the source, substitute displayed strings, or deep-copy payloads between preview and execution. A selected unsupported type or invalid plan yields a preview error and no executable confirmation.

### RESTORE-UI-003

The frontend MUST present a concrete restoration preview identifying the exact snapshot target, configured parent and new leaf, marked/all selection, included ancestors, resulting entry count, total regular payload bytes, no-replacement policy and supported metadata policy before offering publication confirmation.

Provenance: POLICY. The destination and every selected root are byte-escaped, with full text accessible through bounded scrolling; counts include required ancestors. Distinguish zero payload from unknown values rather than overflow/wrapping. Explain that ordinary modes, supported mtimes and raw symlink targets follow the backend; ownership/ACL/xattr copying and crash durability are not promised. Identify a genuinely empty plan's policy root `0700` and unspecified root mtime when applicable. Automatic profile/parent/destination validation occurs after confirmation and before staging; the preview never promises that those checks or subsequent filesystem I/O will succeed. The parent pathname is caller-selected and may resolve through symlinks or later be rebound.

### RESTORE-UI-004

The frontend MUST start at most one restoration operation only when its displayed successful `r` preview receives the exact ASCII acknowledgment `restore` followed by Enter, and MUST clear that acknowledgment and disarm confirmation when its snapshot, roots or destination changes or the user leaves that preview.

Provenance: POLICY, DOC-CROSSTERM-KEYS. The dedicated seven-byte maximum acknowledgment field starts empty for each successful preview. Ignore acknowledgment/confirmation input while preparing; consume pending terminal events until none are immediately available before displaying/arming the ready preview. A bounded drain may continue across event-loop iterations and leaves the preview unarmed while pending input remains. Enter with an empty/mismatched field never starts restoration. Backspace edits only this fixed acknowledgment; it is neither destination nor credential input. In this preview, character keys such as `r` edit the acknowledgment instead of triggering another preview. Ignore events explicitly reported as Repeat/Release, but legacy Unix terminals can report repeats as Press; do not claim physical-new-key detection or physical typing origin. The required observed multi-character sequence prevents a held Enter or a single held character from supplying acknowledgment. `p` remains read-only; history Enter retains the distinct volume-scan action. Snapshot changes/navigation, a new scan, marks, Esc, errors, completed requests and stale replies clear the field/disarm. Confirmed inputs move as one immutable request and clear the acknowledgment immediately; subsequent Enter events never enqueue duplicates.

### RESTORE-UI-005

The frontend MUST execute restoration through the adopted native `restore` API with its reviewed limits and profile policy, without implementing its own path extraction, destination replacement, staging cleanup or filesystem eligibility fallback.

Provenance: RESTORE-007..027, native backend review. Initial frontend scope uses `RestoreLimits::default()` and automatic profile selection; there is no unsafe override or filesystem/permission workaround option. Backend invalid-destination, unsupported OS/filesystem/profile, fidelity, resource and syscall results remain explicit diagnostics. On non-Linux platforms reject restoration configuration/action as unsupported while retaining supported browsing. Native preflight may reject a confirmed request before creating staging; this is not a preview success claim being converted into backend success.

### RESTORE-UI-006

The frontend MUST retain one frozen operation's exact snapshot, roots, destination and bounded plan summary across preview and confirmation, and keep at most one active scan, preview or restoration with bounded request/reply queues.

Provenance: POLICY, DOC-RUST-THREAD. Shared immutable snapshot ownership, such as `Arc<ValidatedSnapshot>`, permits worker-local borrowed plan creation without copying payloads. A confirmation worker may recreate the deterministic plan from that same snapshot and roots, but must compare its counts/payload summary before entering restoration. Request and reply queues each have capacity at most one; stale replies cannot mutate another operation's state. Source/catalog replacement, new scans and selection changes are disabled during active restoration. Worker/thread/channel failure before dispatch disarms confirmation and reports an error, never a successful publication.

### RESTORE-UI-007

The frontend MUST run plan preparation and filesystem restoration outside the terminal event/render loop, with an operation-specific cancellation token and explicit running/cancellation-requested states.

Provenance: POLICY, RESTORE-021. Plan construction is bounded synchronous work and has no internal cancellation promise; observe cancellation before and after it and before entering the backend. While a restore runs, Esc requests cancellation and waits for its result. `q` likewise requests cancellation instead of immediately exiting; after the result is visible a subsequent `q` exits. No fixed cancellation latency, progress percentage or interruptible-kernel guarantee is claimed. Preview abandonment/other-operation cancellation must never reuse a cancelled token for a later restoration.

### RESTORE-UI-008

The frontend MUST preserve the backend's published, unpublished and publication-indeterminate outcome distinction, including successful-publication precedence over racing cancellation and all reported cleanup warnings or uncertainty.

Provenance: RESTORE-021/023/026/027. Published results state completed publication and separately identify empty-wrapper cleanup warnings. Unpublished results identify initiating cause, cleanup attempts/failures/budget/truncation and any uncertain first-outer-leaf candidate; they do not claim all staging artifacts were removed. Indeterminate results state that publication was attempted and effects cannot be established, never displaying an untouched/absent destination, cancelled restoration or complete success instead. Reaching an exact byte count or setting a cancellation token supplies no substitute result.

### RESTORE-UI-009

The frontend MUST refrain from automatic retry, rollback, destination inspection-based outcome promotion or modifying cleanup after publication uncertainty or an unavailable worker outcome.

Provenance: RESTORE-026/027. Preserve diagnostic names/identities for inspection without chmod, unlink, rename, overwrite, recovery or another restore in the same session after a publication-indeterminate result. An unpublished result containing an uncertain first staging candidate also disables another restore in that session; leave the candidate untouched. Only an ordinary definitely unpublished outcome permits a new `r` preview and fresh confirmation. Explicit user restart/manual recovery is outside the initial flow and confers no outcome-resolution assurance.

### RESTORE-UI-010

The frontend MUST render destination, selected paths, staging/candidate names and diagnostics using reversible terminal-safe byte escaping, preserving complete backend identity and warning fields in the accessible final result.

Provenance: PARSE-PATH-001, reviewed terminal escaping. Printable ASCII except backslash displays literally; backslash doubles and every other byte uses `\xNN`. Never use display text as an input identity or render control bytes/ANSI from names/errors. Final receipts retain parent/root device-inode identities, profile, filesystem magic, original destination/staging names and empty-root-mtime policy. Label the supplied parent pathname as the requested path; an anchored parent identity does not prove that this textual pathname still locates the published root after rebinding. Viewport clipping does not discard the underlying receipt or silently truncate required diagnostics.

### RESTORE-UI-011

The frontend MUST bound owned restoration request, preview and receipt metadata before allocating it, retaining original payload ownership without payload copies.

Provenance: POLICY. Initial independent frontend bounds are 1,024 roots, 4 MiB of owned root component/vector capacity, 16 million plan-candidate comparisons, 20 MiB of escaped preview text and 1 MiB of final rendered diagnostics. Charge all simultaneous frontend-owned root copies if any; snapshot payload/state and core/backend budgets are separate. Check arithmetic and escape expansion before allocations; reject excess rather than truncate identity/security-relevant text. Bounded queues, operation metadata, result ownership and lifecycle are reviewed explicitly. Allocator, terminal renderer and OS argument storage remain trusted infrastructure rather than a global heap proof.

### RESTORE-UI-012

On a controlled terminal I/O failure or shutdown during active restoration, the frontend MUST request cancellation, close or signal its worker request path, attempt terminal-state restoration, drain and join the worker without bounded-channel deadlock, retain its typed result, and attempt escaped final-result reporting through an available non-raw output sink before controlled exit.

Provenance: POLICY, DOC-RUST-THREAD. Close all frontend-owned request senders or use an equivalent shutdown signal that prevents the worker from waiting indefinitely for another request after completion. Drain a reply or retain an equivalent result path before joining a worker that could be blocked sending it. Terminal failure never authorizes detaching and forgetting a possibly publishing operation. Teardown and output faults are retained/reported separately without discarding a known backend outcome or fabricating uncertainty. Reporting uses fallible writes, not panic-on-output-failure helpers; successful external delivery requires an available sink, since terminal and stderr can both be unwritable. A worker panic/disconnection without a final backend result is identified as `restoration outcome unavailable; inspection required`, with configured destination and available operation metadata, rather than a fabricated backend unpublished/cancelled result. Disable retries/modifying recovery. Fatal signals, process aborts, crashes and allocator-abort termination remain outside controlled-shutdown assurance; no receipt-recovery/delivery guarantee covers those cases.

### RESTORE-UI-013

The frontend MUST keep backup source access read-only and avoid collecting or exposing credentials, backup payload bytes or secret material through the restoration interaction, diagnostics or automatic logs.

Provenance: PARSE-SOURCE-001, POLICY. The worker restores already validated in-memory entries and needs no further source request or bucket write. Initial destination flags contain paths, not passwords/keys; use the existing SDK credential provider outside this restoration flow. Do not dump environment/configuration/argv/SDK credential errors or file content to construct a receipt. User-requested path/identity diagnostics are preserved through escaping. No automatic receipt file, clipboard action, upload, bucket mutation or external-message delivery is part of this profile.

### RESTORE-UI-014

Frontend restoration acceptance MUST separately record state-machine/CLI tests, native worker/result integration, real terminal interaction evidence, backend native evidence and outstanding formal controller obligations.

Provenance: ASSURE-PROOF/TRACE, RESTORE-024. Duvet links target these canonical headings; passing annotations/counts or parser proofs cannot establish frontend lifecycle, kernel behavior, cancellation or complete restoration. Required scenarios include original-byte destinations/roots, stale previews/replies, acknowledgment reset/mismatch/preparation input/held Enter or single-character repeats, repeated confirmations, worker/terminal/output failure and channel backpressure, all three outcomes, cleanup warnings/uncertain candidates, success racing cancellation, unsupported configuration, bounded metadata and no source writes. Real PTY evidence checks displayed preview/result, key sequences and terminal-state restoration; pure tests alone do not establish that the actual terminal communicates the required choices.

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
