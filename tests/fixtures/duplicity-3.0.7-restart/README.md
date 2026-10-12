# Official duplicity 3.0.7 restart boundary fixtures

This package preserves original producer bytes for three plain full backup sets. Each was restored with official duplicity, and its restored paths, types, ordinary permissions, modification times, sizes, and SHA-256 contents matched an independent source-tree oracle. Native-reader compatibility and formal proof acceptance remain separate.

| Profile | Source files | Observed volume 2 |
| --- | --- | --- |
| `one-entry/` | 17 | Raw manifest `[1,2,2]`; replacement starts root/None, ends f016/None, and contains only snapshot/f016 |
| `two-entry/` | 18 | Raw manifest `[1,2,2]`; replacement starts f017/None despite its first entry f016, followed by f017 |
| `empty-tail/` | 16 | Complete volume 2 gzip member decodes to zero bytes; starts root/None and ends at preceding f015/None |

Every profile's `objects/` contains all original stored objects, including the complete unedited raw manifest with duplicate restart records and original signature archive. `expected.json` uses byte-path/component hex and independent source metadata/content hashes. `provenance.json` preserves object identities, normalized exact CLI commands, original exploration/oracle digests, and the separate reproduction recipe identity. `boundary-observations.json` records actual entries and decoded lengths. Restart profiles also preserve the original interrupted partial manifest and actual CLI logs. The old partial manifest may refer to a subsequently rewritten volume; verify final hashes against the LAST raw record and discard an obsolete higher-volume suffix.

Stored objects and original manifests were not renamed internally, rewritten, normalized, recompressed, or stripped. Their Hostname/Localdir fields retain the original producer's test environment; they are data, not executable restoration paths. Command provenance uses `${WORK}`, `${DUPLICITY}`, and `${GPG}` substitutions, with tool/source identities separately recorded in `producer-identities.json`. No user's backup credentials or private production keys are involved.

## Controlled interruption and signature limitation

The original recipe creates 200 deterministic 65500-byte files to make the process remain active after volume 2. It stops/kills only its own producer process once that upload exists. On resumption, already retained boundary file f015 is transiently removed, forcing official `restart_position_iterator` to queue f016 after passing the missing index. The unretained source tail is intentionally removed so the resulting set stays small. The retained file is restored to the final source before comparing the independent oracle; volume 1 already contains it. Producer Python/C code and archive contents are untouched.

The final signature archives for these controlled restart sets omit f015 even though retained data contains it and official data restoration recovers it. Those original signature bytes are included for provenance but are **not an authoritative complete data inventory** in these source-changing fixtures. This package establishes data restoration/boundary behavior only; it does not establish signature-based browsing correctness or a consistent snapshot of a concurrently changing source. Filelist summaries are advisory as well.

Queued compatibility is bounded by observed raw/gzip entry-aligned EOF: `entry_stream_bytes == decoded_length`, with no footer bytes. Object names cannot identify producer version. Queued encrypted or footer-bearing replay requires separate evidence; accepting these fixtures never authorizes arbitrary trailers, missing volumes, unchecked hashes, or incomplete multipart data.

## Audit and reproduction

Run the standalone read-only audit with Python 3.11 or newer:

```sh
python3 verify.py
```

It verifies every package artifact hash, original stored-object hashes, final restart-normalized SHA1 declarations, deterministic source hashes, decoded data metadata, and the exact boundary shapes. It uses standard-library gzip/tar independently of project Rust and extracts no filesystem paths. `package-provenance.json` intentionally excludes itself from its artifact list; its final SHA-256 is supplied with the package handoff.

Reproduce profile semantics using explicitly selected official tools and a new output directory:

```sh
python3 producer/generate_restart.py \
  --duplicity /path/to/official/duplicity-3.0.7 \
  --gpg /path/to/official/gpg \
  --profile two-entry --output /tmp/new-restart-fixture
```

Repeat for `one-entry` and `empty-tail`. The portable recipe was actually run for all three profiles and independently passed official restore/source comparisons; normalized run receipts and CLI logs are stored separately in `producer/reproduction-evidence/`. Original one/two source metadata oracles are also preserved unedited for conversion/provenance audit. It recreates the controlled producer behavior, not the frozen original archive bytes: gzip timestamps, hostname/Localdir, and process timing can differ. Initial interruption races are checked and fail rather than claiming success from a completed backup. No official app is a dependency of the native reader runtime.
