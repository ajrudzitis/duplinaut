# Official fixture producer

`generate.py` creates deterministic source trees, runs an official duplicity executable for full and two incremental backups, restores every date with official duplicity, and checks its source-tree oracle before copying a complete new fixture directory into place. It also extracts small official delta payloads and inspects container boundaries. It invokes no project reader code.

Run with Python 3.11 or newer and explicitly selected official tools:

```sh
python3 tests/producer/generate.py \
  --duplicity /path/to/official/duplicity \
  --gpg /path/to/gpg \
  --expected-version 3.0.7 \
  --output /tmp/duplicity-reader-new-fixtures
```

Output must not exist. Source trees, caches, private test-agent state, official restored trees, and an encrypted multipart probe use isolated temporary directories. `--plain-only` omits encrypted chains and test-key generation. `--small` also omits the large incompressible plain payloads. Repeat `--producer-option=--option` to pass options to backup and restore; the official executable must actually support them. New-object tracking extracts deltas independently of long/short/legacy object names. Missing tools, wrong producer version, failed backup/restoration, wrong source comparison, or failed container observations fail the command. A GnuPG agent needs permission to create a local socket.

`cargo xtask producer` runs this same recipe. Set `PRODUCER_PYTHON`, `PRODUCER_DUPLICITY`, `PRODUCER_GPG` and `PRODUCER_OUTPUT` explicitly. `PRODUCER_VERSION` defaults to `3.0.7`; `PRODUCER_PROFILE` accepts `full` (default), `plain`, or `small-plain`. The wrapper preserves argument boundaries and never overwrites an existing output. Use the direct Python invocation above for historical producer options.

The checked-in 3.0.7 corpus was produced with:

- Duplicity: `/nix/store/5j4mk0k17j37nh8a7s2pcr5z0vblx73r-duplicity-3.0.7/bin/duplicity`
- Python: `/nix/store/9r02ykx9y35lf4gk6gc30gnag1y8ncp7-python3-3.13.15/bin/python3.13`
- GnuPG: `/nix/store/krh2gxql1h8yx1h4l57vvphaw5wf1wqz-gnupg-2.4.9/bin/gpg`

The exact invoked argv is in fixture provenance. `--volsize 1`, `--current-time`, fresh per-mode archive directories, and explicit GnuPG loopback mode avoid reliance on a user's backup cache or keyring. The test passphrase is intentionally public and recorded with the exported synthetic test keys. The source recipe digest is recorded alongside produced artifacts.

## Compatibility matrix

| Producer | Actual status | Invocation/profile |
| --- | --- | --- |
| 0.8.23 | Nine snapshots passed source/official restore checks | Small plain standard, `--short-filenames`, and `--old-filenames --time-separator=_` chains |
| 2.1.3 | Three snapshots passed source/official restore checks | Small plain standard chain |
| 3.0.7 | 12 snapshots and encrypted boundary probe passed official reference checks | Recorded Nix-store tools and checked-in corpus |
| 3.2.1 | Three snapshots passed source/official restore checks | Small plain standard chain; observed record-aligned zero footer |

Historical versions require pinned release source and a compatible Python/dependency environment; merely changing the expected version does not acquire or validate that environment. Targeted legacy-name/GNU/PAX variants require additional producer recipes. A completed producer run does not establish native-reader compatibility until the native integration tests and required proofs pass.

## Pinned historical producer builds

PyPI release metadata was inspected on 2026-10-08. All three source distributions were downloaded, verified against these SHA-256 identities, and executed through their original entry points after compiling each release's own unmodified librsync C extension:

| Release metadata | Source-distribution SHA-256 | Published Python constraint | Executed isolated runtime |
| --- | --- | --- | --- |
| [0.8.23](https://pypi.org/project/duplicity/0.8.23/) | `35e0d218bc6569aa99e51795c5b521093af21283af8c4db4f20652e76c20fefd` | `>2.6`, `<4`, excludes 3.0..3.4 | Python 3.12.14; direct extension build avoids obsolete setuptools test commands |
| [2.1.3](https://pypi.org/project/duplicity/2.1.3/) | `678b88c812cbab1a3a1fd34f7f18f86fd48ca9fa8b16bc4c74bbdce14b3a5a00` | `>=3.8`, `<4` | Python 3.12.14; fasteners 0.20 |
| [3.2.1](https://pypi.org/project/duplicity/3.2.1/) | `4284e3d8f0082a85d3dcfa634901532a1087463df7a33ead417670c0d63b9ef5` | `>=3.10`, `<3.15` | Python 3.12.14; local backend, no optional cloud dependencies |

The checked-in `historical-environment.nix` requires Nixpkgs revision `774debe7a0d1b496e35677ad955a1011c6ff74f3`. Its Python 3.12.14 environment supplies future 1.0.0 and fasteners 0.20; GCC 15.3.0 compiles the original extensions against librsync 2.3.4. `build_historical.py` checks archive hashes and creates isolated launchers that select the original source and put the explicit official GnuPG binary in PATH. No upstream source patches were applied. This deliberately bypasses packaging-only setup commands, not producer algorithms. Per-version `producer-environment.json` records the source, compiler, linked-library, executable, extension, and recipe identities. Record the Nixpkgs revision/derivation and all applied build patches. A compatibility patch cannot silently alter producer file formats. Verify `duplicity --version`, option availability, and a small local full/restore cycle before running the full recipe.

The executed historical profiles use the same byte-path/metadata/type-transition source oracle and all three dates, with `--plain-only --small`. Historical symmetric/public-key/signed profiles, large multipart variants, RSA/other permitted crypto algorithms, non-UTC legacy zones, and additional metadata forms remain pending. If a release requires recipe changes, preserve version-specific argv in provenance and explain the difference rather than overwriting existing corpus truth. Add targeted `--short-filenames`, legacy timestamp/separator options where actually supported, and large encrypted multipart cases. Inspect actual chunk sizes, manifest block semantics, tar headers/termination, signature magic, and OpenPGP packet algorithms for each release; observed 3.0.7 constants cannot be assumed.

Keep large historical fixtures economical: retain complete small chains for each encryption mode, plus one necessary multipart chain per materially different format behavior. Each version's directory needs original object hashes, deterministic source expectations, official restore comparisons, producer/tool identities, and native-reader test results. Unsupported historical crypto algorithms remain explicit rejection tests. Mark a matrix cell complete only after the pinned producer run, independent native comparison, and applicable assurance gate all pass.


Reproduce the historical standard runs using a writable temporary work directory and the required Nixpkgs source selection:

```sh
nix-shell tests/producer/historical-environment.nix --run \
  'python3 tests/producer/build_historical.py --work /tmp/historical-producers --gpg /path/to/official/gpg'
python3 tests/producer/generate.py \
  --duplicity /tmp/historical-producers/producer-0.8.23 \
  --gpg /path/to/official/gpg --expected-version 0.8.23 \
  --plain-only --small --output /tmp/historical-0.8.23-standard
```

Repeat the second command with 2.1.3 and 3.2.1. For 0.8.23 short names add `--producer-option=--short-filenames`; for legacy names add `--producer-option=--old-filenames --producer-option=--time-separator=_`. Use a new output directory for each profile. The generator itself requires Python 3.11+ separately from the launcher's pinned interpreter. Duplicity 3.2.1 restore startup can probe GnuPG loopback/agent support even for plain archives; the isolated agent socket must be available.

Encountered setup/inspection failures are preserved in `tests/fixtures/historical-producers.json`: current Python 3.11/Sphinx and Python 3.13/future dependency constraints, absent GPG in launcher PATH, sandboxed 3.2.1 agent startup, and the obsolete no-footer observation assumption. None required modifying upstream producer algorithms. Actual 3.2.1 gzip volumes include zero padding, including a one-block 512-byte footer; earlier plain profiles end at complete-entry EOF. The generator now records either actual integral all-zero padding or EOF, rejecting nonzero trailing data.

The frozen 3.0.7 corpus retains its original recipe digest and provenance. Its exact generating recipe is available at repository commit `908e330`; current generator changes do not rewrite that historical evidence. Native-reader compatibility is recorded independently from every official producer/restore success.

The [interrupted-producer recipe](../fixtures/duplicity-3.0.7-restart/producer/generate_restart.py) accepts explicit official duplicity/GPG tools and a fresh work directory. The adjacent package README documents reproduction commands, original producer identities, restore receipts, and the restart signature-inventory limitation. Preserve the frozen package's original bytes; record reader acceptance separately.
