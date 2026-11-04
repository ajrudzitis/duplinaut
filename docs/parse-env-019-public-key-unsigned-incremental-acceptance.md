# PARSE-ENV-019 public-key-encrypted unsigned incremental acceptance plan

Status: proposed; no implementation or backlog status changes are included.

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

- Run the focused mixed-chain test and `cargo test -p duplicity-core` using
  only checked-in fixtures. The mixed source MUST contain exactly the six
  selected objects and no duplicate keys.
- Run the targeted Duvet implementation/test query for PARSE-ENV-019,
  `cargo xtask trace-update`, and `cargo xtask trace`. The implementation
  annotation MUST identify chain-wide signer authorization; the test
  annotation MUST identify the PKESK-encrypted unsigned-incremental case.
- Run formatting and strict core Clippy checks, then required assurance CI
  (`cargo xtask ci`). Record hosted Linux CI evidence before marking this
  acceptance complete.
- Independent review MUST verify the unsigned status of both official
  incremental manifests from the reference inspection, exact fixture
  selection, collision absence, credential/primary-fingerprint match, and
  exact third-open `Unsigned` failure with no successful snapshot/evidence.
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
