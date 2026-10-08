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

Output must not exist. Source trees, caches, private test-agent state, official restored trees, and an encrypted multipart probe use isolated temporary directories. `--plain-only` omits encrypted chains and test-key generation. Missing tools, wrong producer version, failed backup/restoration, wrong source comparison, or failed container observations fail the command. A GnuPG agent needs permission to create a local socket.

The checked-in 3.0.7 corpus was produced with:

- Duplicity: `/nix/store/5j4mk0k17j37nh8a7s2pcr5z0vblx73r-duplicity-3.0.7/bin/duplicity`
- Python: `/nix/store/9r02ykx9y35lf4gk6gc30gnag1y8ncp7-python3-3.13.15/bin/python3.13`
- GnuPG: `/nix/store/krh2gxql1h8yx1h4l57vvphaw5wf1wqz-gnupg-2.4.9/bin/gpg`

The exact invoked argv is in fixture provenance. `--volsize 1`, `--current-time`, fresh per-mode archive directories, and explicit GnuPG loopback mode avoid reliance on a user's backup cache or keyring. The test passphrase is intentionally public and recorded with the exported synthetic test keys. The source recipe digest is recorded alongside produced artifacts.

## Compatibility matrix

| Producer | Actual status | Invocation/profile |
| --- | --- | --- |
| 0.8.23 | Pending tool acquisition and execution | Supply official executable and `--expected-version 0.8.23`; validate option/runtime compatibility before claiming success |
| 2.1.3 | Pending tool acquisition and execution | Supply official executable and `--expected-version 2.1.3`; validate option/runtime compatibility before claiming success |
| 3.0.7 | 12 snapshots and encrypted boundary probe passed official reference checks | Recorded Nix-store tools and checked-in corpus |
| 3.2.1 | Pending pinned tool acquisition and execution | Supply official executable and `--expected-version 3.2.1`; validate option/runtime compatibility before claiming success |

Historical versions require pinned release source and a compatible Python/dependency environment; merely changing the expected version does not acquire or validate that environment. Targeted legacy-name/GNU/PAX variants require additional producer recipes. A completed producer run does not establish native-reader compatibility until the native integration tests and required proofs pass.
