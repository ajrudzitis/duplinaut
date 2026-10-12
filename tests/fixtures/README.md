# Official duplicity fixtures

`duplicity-3.0.7/` contains original archives produced by official duplicity **3.0.7**, using GnuPG **2.4.9**, and independent source-tree expectations. No production reader output supplies an expected result.

- `plain/`: gzip full backup with three 1 MiB target volumes, first incremental with two volumes, and second incremental with one volume. A deterministic incompressible 2.5 MiB file forces multipart snapshots; changing its first 1.4 MiB forces a multipart librsync delta.
- `symmetric/`, `public-key/`, `signed/`: smaller full-plus-two-incremental chains exercising symmetric OpenPGP, recipient encryption, and recipient encryption with signing.
- `expected.json`: 12 source-tree snapshots with byte-valued path/component hex, types, modes, source modification times, regular-file sizes/SHA-256, and symlink targets. Backup times are 1700000000, 1700086400, and 1700172800.
- `provenance.json`: producer versions, executable identities, source-recipe digest, normalized exact argv, and SHA-256/size records for fixture artifacts.
- `format-observations.json`: actual tar/PAX/multipart shapes, plus the temporary official encrypted boundary probe's exact footer/top-off results. Probe archives are omitted to avoid duplicating large random payloads.
- `deltas/`: four verbatim `diff/plain.txt` and `diff/metadata.txt` payloads extracted from official incremental volumes. Its provenance maps every payload to the original object, member, and digest; these are not hand-authored opcode examples.
- `test-keys/`: exported public and encrypted secret test keys, permitted signer fingerprint, and passphrase. These credentials are intentionally public, synthetic, and solely for fixture tests; they are not production keys.

The source recipe includes UTF-8 and non-UTF-8 names, long paths, quotes/spaces/backslashes/control characters, component-order punctuation, symlinks, empty entries, metadata-only changes, deletion/recreation, and regular-file→directory→symlink replacement. Metadata changes also produce valid copy-only deltas.

The producer restored every snapshot with official duplicity and compared paths, contents, types, symlink targets, ordinary mode bits, and modification times with its original source tree. **Symlink modification times are excluded from the official comparison** because duplicity 3.0.7 deliberately skips symlink `utime`; source symlink times remain in the oracle for any stronger native restoration policy. This exclusion is recorded for each snapshot. Ciphertext randomness, test-key generation, gzip timestamps, hostname, and temporary Localdir make archive bytes nonreproducible; source data, expected metadata, and backup dates are deterministic.

These fixtures establish official-producer/reference behavior. Native-reader integration tests, proofs, and independent acceptance remain separate evidence. Additional original plain producer corpora are listed below. Historical encryption/multipart coverage and native acceptance remain pending. Regeneration instructions are in [the producer README](../producer/README.md).

The native [OpenPGP profile](../../docs/openpgp-profile.md) records the pinned Rust adapter capability and pending acceptance checks. The exported v4 primary signs with legacy Ed25519; its bound cv25519 ECDH subkey receives encrypted objects. Secret material is AES-128 protected with SHA1 iterated/salted S2K (count 58720256); certification/binding signatures use SHA512. The signed full-volume envelope uses PKESKv3, AES-256 SEIPDv1, ZLIB, and a binary Ed25519/SHA512 signature. An isolated official GnuPG inspection returned GOODMDC, DECRYPTION_OKAY, and VALIDSIG for the complete allowed primary fingerprint; the reproducible inspection recipe is in the adapter notes. These observations describe the original public test corpus, not successful native decryption, certificate policy, or signature verification.


## Historical plain corpora

`historical-producers.json` indexes 15 additional officially restored snapshots:

| Producer/profile | Fixture directory | Actual evidence |
| --- | --- | --- |
| 0.8.23 standard | `duplicity-0.8.23/standard/` | Small plain full + two incrementals |
| 0.8.23 short | `duplicity-0.8.23/short/` | Same chain with original `.dt.z`/`.st.z` and base36 names |
| 0.8.23 legacy | `duplicity-0.8.23/legacy/` | Same chain with UTC legacy timestamps and `_` time separator |
| 2.1.3 standard | `duplicity-2.1.3/standard/` | Small plain full + two incrementals |
| 3.2.1 standard | `duplicity-3.2.1/standard/` | Same chain with producer-added zero tar footers |

Each profile contains `plain/`, `expected.json`, `provenance.json`, `format-observations.json`, and four extracted official deltas. Its provenance references the adjacent per-version `producer-environment.json` by SHA-256. All five profiles use the same source tree and official restoration comparisons, including the documented symlink-time exclusion. The 80 hashed artifacts occupy about 184 KiB before provenance; no redundant large random historical payloads are checked in.

Original release archives were hash-verified; each original librsync extension was built without source patches under the pinned Nix Python 3.12.14/GCC 15.3.0/librsync 2.3.4 environment. Official 0.8.23 and 2.1.3 gzip volumes end at an entry boundary; 3.2.1 writes its record footer, including an observed 512-byte case. `native_reader_verified` remains false in this checkpoint's producer index: it describes producer/oracle evidence and is not a live native test-status ledger.

## Interrupted 3.0.7 producers

[The restart corpus](duplicity-3.0.7-restart/README.md) preserves one-entry and two-entry queued replay plus an empty-tail volume. Official restorations match 51 files and three root directories, including contents, modes and timestamps. Its standalone `verify.py` audits 45 hashed artifacts and normalized manifest digests without invoking this reader. The signature inventory is incomplete after the controlled source change and must not be used as a complete browsing index. Native compatibility and formal controller verification remain pending.
