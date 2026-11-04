# PARSE-ENV-019 public-key-encrypted unsigned incremental acceptance plan

Status: accepted. Focused/full core tests, Duvet query/full trace, formatting,
strict Clippy, diff checks, local `cargo xtask ci`, provenance checks, and
independent review pass. Hosted Linux assurance and Duvet Pages passed in
[run 41818457786](https://github.com/ajrudzitis/duplinaut/actions/runs/41818457786).

This local reader test extends
[PARSE-ENV-019](parsing-requirements.md#parse-env-019) with a second encrypted
incremental profile: the official 3.0.7 public-key (PKESK) objects are
encrypted but unsigned. The signed-full to symmetric-incremental case has its
own [acceptance plan](parse-env-019-unsigned-openpgp-incremental-acceptance.md).
This case uses checked-in official objects and requires no live AWS access or
producer-fixture changes.

## Acceptance requirements

1. Before constructing the mixed source, inspect both official
   `public-key/` incremental manifest objects:
   - `duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gpg`
   - `duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.gpg`

   Inspect
   [`tests/fixtures/duplicity-3.0.7/provenance.json`](../tests/fixtures/duplicity-3.0.7/provenance.json)
   and [`tests/producer/generate.py`](../tests/producer/generate.py). For each
   exact incremental output, verify that its recorded producer command has
   `--encrypt-key` and no `--sign-key`, and that the named fixture path and
   recorded hash identify the checked-in bytes. Record this provenance check
   in test comments or test evidence. This establishes only the declared
   source-generation profile for these outputs; it is not a runtime/GPG
   requirement and makes no claim about outputs produced by other commands or
   profiles. The Rust test and reader runtime MUST NOT invoke GnuPG or any
   other external crypto tool.
2. Construct one `FixtureSource` from exactly these six original objects:
   the signed profile's full manifest and full data volume, plus the two
   public-key profile incremental manifests and corresponding `vol1` data
   volumes for the date intervals above. Preserve each selected object's
   original name, bytes, and size; recompute synthetic object identities from
   those bytes. Select no same-named objects from both profiles and assert
   there are no catalog-key collisions.
3. Read the mixed chain to `20231116T221320Z` with the correct protected
   recipient secret export and its fixture passphrase, the official signer
   certificate, its exact permitted primary fingerprint, and
   `require_signature: true`. `message_passwords` MUST be empty. The existing
   all-signed official-chain success test remains the positive control for
   these credentials and signer policy.
4. Require exactly `ReaderError::Crypto(OpenPgpError::Unsigned)` when the
   reader reaches the public-key-encrypted manifest for the first incremental
   interval. The source open count MUST be exactly three: signed full
   manifest, signed full volume, and first unsigned incremental manifest. The
   call MUST return no `ValidatedSnapshot` or authentication evidence. This
   proves the reader does not treat successful PKESK decryption as a substitute
   for the configured signature requirement.
5. Add a Duvet test annotation linking this regression to PARSE-ENV-019. The
   test MUST not print credentials, decrypted bytes, or plaintext-derived
   diagnostics.

## Verification and evidence

### Current local evidence

- `tests/fixtures/duplicity-3.0.7/provenance.json` records `duplicity
  incremental` commands for both selected public-key intervals
  (`1700086400` and `1700172800`). Each command includes
  `--encrypt-key D06847BAC3B117C46BB68BDA6F23173BAD8A7656` and has no
  `--sign-key`. In `tests/producer/generate.py`, the public-key and signed
  profiles both add `--encrypt-key`; only the signed profile adds
  `--sign-key`. This is evidence of the checked-in fixture recipe only.
- The test helper checks the exact recorded SHA-256 values for all four
  selected public-key outputs:

  | Output | SHA-256 |
  | --- | --- |
  | `duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gpg` | `217171eb4460e330153350683b91f66ef14c766ecf40ad966bf8ca14a58c4594` |
  | `duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gpg` | `eac62fc566682c7fbb63029e14326c925f26004e4640afb09aabb044a52b12b6` |
  | `duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.gpg` | `1d2f90cfa9f6f956414e6fa54efa4db9a1a551807d0f5cf69c3cf93fc46d024e` |
  | `duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gpg` | `ce224c3b2ed3abb0b54523ff07a4da4f80450369a2f9ce25f36518aeb64e80d1` |

- `signer_policy_rejects_public_key_incremental_after_signed_full` selects
  exactly six objects from the signed full and public-key incremental profiles,
  asserts the source has six entries and rejects same-key collisions. It uses
  the protected recipient secret export with its fixture passphrase, the
  official signer certificate and exact permitted primary fingerprint, and an
  empty `message_passwords` list. The read fails with exact `Unsigned` after
  three source opens. Since the API returns `Err`, no snapshot or authentication
  evidence is returned. The focused and full core test suites passed locally;
  independent review and local assurance CI passed. Hosted Linux assurance and
  Duvet Pages passed in
  [run 41818457786](https://github.com/ajrudzitis/duplinaut/actions/runs/41818457786).

- Local verification passed: focused mixed-chain test, full `cargo test -p
  duplicity-core`, targeted Duvet implementation/test query, `cargo xtask
  trace-update`, `cargo xtask trace`, `cargo fmt --all -- --check`, `cargo
  clippy -p duplicity-core --all-targets -- -D warnings`, `git diff --check`,
  and `cargo xtask ci`. Independent review passed; hosted assurance remains
  pending.

- Run the focused mixed-chain test and `cargo test -p duplicity-core` using
  only checked-in fixtures. The mixed source MUST contain exactly the six
  selected objects and no duplicate keys.
- Run the targeted Duvet implementation/test query for PARSE-ENV-019,
  `cargo xtask trace-update`, and `cargo xtask trace`. The implementation
  annotation MUST identify chain-wide signer authorization; the test
  annotation MUST identify the PKESK-encrypted unsigned-incremental case.
- Formatting, strict core Clippy, and local assurance CI (`cargo xtask ci`)
  passed. Hosted Linux assurance and Duvet Pages passed in
  [run 41818457786](https://github.com/ajrudzitis/duplinaut/actions/runs/41818457786).
- Independent review verified the unsigned status of both official incremental
  manifests from reference inspection, exact fixture selection, collision
  absence, credential/primary-fingerprint match, and exact third-open
  `Unsigned` failure with no successful snapshot/evidence.
- This is native policy-continuity evidence only. It does not prove upstream
  packet parsing, cryptographic primitives, or the complete reader controller.

## Scope limits

This acceptance covers one 3.0.7 signed-full to public-key-encrypted unsigned
incremental chain and the first unsigned incremental manifest. It does not
cover unsigned encrypted data volumes, other incremental encryption/signature
transitions, irrelevant catalog artifact classes, other producer versions,
additional algorithms, certificate validity/revocation policy, or
reader-controller refinement. It complements but does not replace the
symmetric-incremental acceptance case.
