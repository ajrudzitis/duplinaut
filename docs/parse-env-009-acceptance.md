# PARSE-ENV-009 caller-credential acceptance plan

Status: implementation and full core verification complete; independent review
and hosted assurance CI remain pending.

This is a local traceability and regression-test slice for
[PARSE-ENV-009](parsing-requirements.md#parse-env-009). Its bounded scope is
the caller-provided credential path: symmetric message passphrases and
exported supported secret keys. The official encrypted-chain source-oracle
test now carries the PARSE-ENV-009 trace link and explicitly checks both
credential modes.

## Acceptance requirements

1. An implementation annotation MUST link the decode path that consumes
   caller-supplied message passphrases and exported secret-key bytes to
   PARSE-ENV-009. Source review MUST confirm that the adapter uses those
   borrowed inputs directly and does not invoke an external key agent or
   crypto application.
2. A positive symmetric test MUST use the official encrypted 3.0.7 chain and
   the fixture's caller-supplied message passphrase. It MUST reconstruct the
   expected snapshots and authenticate their contents against the independent
   fixture oracle.
3. A positive public-key test MUST use the official encrypted 3.0.7 chain,
   the caller-supplied exported secret-key bytes and their fixture password.
   It MUST reconstruct the expected snapshots and authenticate their contents
   against the same independent fixture oracle.
4. The positive tests MUST be Duvet-linked to PARSE-ENV-009 and run locally
   without network access, live AWS credentials, a keyring, a password prompt,
   or fixture modification. Diagnostics MUST NOT expose passphrases, secret
   key bytes, or decrypted payloads.

## Verification and evidence

- The focused official symmetric and public-key reader tests MUST pass and
  match the recorded independent snapshot/source-content oracle.
- Duvet's implementation and test checks for PARSE-ENV-009 MUST pass. The
  implementation annotation and test links establish traceability, not
  cryptographic primitive correctness.
- Source review MUST verify that credential use is limited to supplied
  message passwords and exported key material. The crypto adapter MUST NOT
  spawn GnuPG, an agent, or another external helper.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before this slice is recorded complete. No Verus proof is claimed for the
  pinned OpenPGP library or its cryptographic primitives.

## Current evidence

- `original_encrypted_chains_reconstruct_each_official_date_with_bounded_evidence`
  runs the checked-in official symmetric, public-key, and signed chains against
  the independent three-date snapshot/content oracle. It asserts that
  symmetric mode uses the supplied message passphrase, while public-key mode
  uses the supplied exported secret-key bytes and a present fixture password;
  both reconstruct the source-oracle snapshots and produce bounded
  authentication evidence.
- Source review found no process/keyring/external-helper use in the OpenPGP
  decode path. `CryptoError` diagnostics contain only the error variant, and
  the reader test asserts that a wrong passphrase does not appear in formatted
  errors. This does not claim secure erasure or zero internal key copies.
- The focused official-chain test, `cargo test -p duplicity-core`, targeted
  PARSE-ENV-009 Duvet query, `cargo xtask trace`, formatting checks, and strict
  core Clippy pass. Clippy emits existing warnings in vendored `vest_lib`,
  while `duplicity-core` passes `-D warnings`. Independent review and hosted
  assurance CI remain pending.

## Non-goals

This slice does not add key formats, algorithms, S2K or secret-material
protection profiles, import keys into a user's keyring, alter credential
storage policy, or prove confidentiality or side-channel resistance. It does
not expand signer authorization or OpenPGP reader-controller proofs, and it
does not require live S3 access.
