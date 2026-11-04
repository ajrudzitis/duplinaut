# PARSE-ENV-019 unsigned OpenPGP incremental acceptance plan

Status: proposed. No implementation or backlog status changes are included.

This local reader test closes one explicit gap in
[PARSE-ENV-019](parsing-requirements.md#parse-env-019): configured signer
authorization must remain required when a chain transitions from a signed full
set to an unsigned, but still encrypted, incremental set. The broader
[OpenPGP reader integration profile](openpgp-reader-integration.md) already
defines chain-wide authorization and lists an unsigned OpenPGP incremental as
pending. The test uses checked-in official 3.0.7 fixtures and needs no live AWS
credentials.

## Acceptance requirements

1. Construct one `FixtureSource` from the checked-in 3.0.7 `signed/` and
   `symmetric/` profiles. Include the signed profile's full manifest and full
   data volume. Include the symmetric profile's incremental manifests and data
   volumes for `20231114T221320Z` to `20231115T221320Z` and
   `20231115T221320Z` to `20231116T221320Z`. Preserve each selected object's
   original bytes, basename, and size; calculate fixture source identities
   from those bytes. Do not include colliding same-name objects from both
   profiles. Signature archives are not required for reconstruction.
2. Configure `Catalog::read_snapshot_with_crypto` with the correct checked-in
   recipient secret export and its fixture passphrase, the correct symmetric
   message passphrase, the checked-in signer certificate, its exact permitted
   primary fingerprint, and `require_signature: true`. Use the same unchanged
   policy for every object in the read. The existing all-signed official-chain
   success test remains the positive control for these credentials and this
   signer policy; the mixed-chain test MUST reach an authorization failure,
   rather than a source, digest, or credential failure.
3. Read from the full-set anchor through the final incremental date. The first
   unsigned required object is the symmetric-encrypted manifest for the first
   incremental set. Require exactly
   `ReaderError::Crypto(OpenPgpError::Unsigned)`. The call MUST return no
   `ValidatedSnapshot` or authentication evidence. It MUST NOT downgrade
   signer policy because the object is encrypted OpenPGP or because an earlier
   full-set object was authorized.
4. Add a Duvet test annotation linking the regression test to PARSE-ENV-019.
   The focused test MUST assert the exact error and absence of a successful
   snapshot result; it MUST NOT print credentials, decrypted bytes, or
   plaintext-derived diagnostics.

## Verification and evidence

- Run the focused mixed-chain test and the complete
  `cargo test -p duplicity-core` suite using only checked-in fixtures.
- Run the targeted Duvet implementation/test query for PARSE-ENV-019,
  `cargo xtask trace-update`, and `cargo xtask trace`. The implementation
  annotation MUST point to the chain-wide authorization decision; the test
  annotation MUST point to the mixed-chain regression.
- Run formatting and strict core Clippy checks, then the repository's required
  assurance CI (`cargo xtask ci`). Record hosted Linux CI evidence before
  marking this acceptance complete.
- Independent review MUST confirm fixture selection has no duplicate object
  names, credentials and the exact primary fingerprint match the official
  signed-chain control, and the failure occurs at the first unsigned
  incremental manifest without returning provisional snapshot/evidence.
- This is native integration evidence for policy continuity. It does not
  formally verify OpenPGP cryptography or the complete reader controller.

## Scope limits

This acceptance covers one 3.0.7 signed-full to symmetric-encrypted-incremental
chain and the first unsigned required incremental manifest. It does not close
the broader PARSE-ENV-019 mixed-envelope matrix, unsigned encrypted data-volume
cases, irrelevant catalog-artifact behavior, other producer versions or
algorithms, certificate validity/revocation policy, reader-controller proofs,
or filesystem restoration. Plain/gzip downgrade rejection remains covered by
its existing test.
