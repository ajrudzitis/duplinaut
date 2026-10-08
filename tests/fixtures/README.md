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

These fixtures establish official-producer/reference behavior. Native-reader integration tests, proofs, and independent acceptance remain separate evidence. Other producer versions and legacy/short filename generation are pending. Regeneration instructions are in [the producer README](../producer/README.md).
